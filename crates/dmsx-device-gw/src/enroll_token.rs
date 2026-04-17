//! Enrollment token parsing / verification (internal beta).
//!
//! Token format (v1):
//! `v1.<payload_b64url>.<sig_b64url>`
//!
//! - payload: JSON `{ "tenant_id": "<uuid>", "device_id": "<uuid?>", "exp": <unix_seconds> }`
//! - sig: HMAC-SHA256(secret, payload_b64url)
//!
//! Secret is read from `DMSX_GW_ENROLL_HMAC_SECRET`.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use tonic::Status;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Deserialize)]
pub struct EnrollTokenClaims {
    pub tenant_id: Uuid,
    pub device_id: Option<Uuid>,
    pub exp: i64,
}

fn secret_from_env() -> Result<Vec<u8>, Status> {
    let s = std::env::var("DMSX_GW_ENROLL_HMAC_SECRET")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            Status::failed_precondition("Enroll disabled: DMSX_GW_ENROLL_HMAC_SECRET is not set")
        })?;
    Ok(s.into_bytes())
}

pub fn verify(token: &str, now_unix: i64) -> Result<EnrollTokenClaims, Status> {
    let token = token.trim();
    let mut parts = token.split('.');
    let Some(ver) = parts.next() else {
        return Err(Status::invalid_argument("enrollment_token format invalid"));
    };
    if ver != "v1" {
        return Err(Status::invalid_argument("enrollment_token version unsupported"));
    }
    let Some(payload_b64) = parts.next() else {
        return Err(Status::invalid_argument("enrollment_token missing payload"));
    };
    let Some(sig_b64) = parts.next() else {
        return Err(Status::invalid_argument("enrollment_token missing signature"));
    };
    if parts.next().is_some() {
        return Err(Status::invalid_argument("enrollment_token has extra segments"));
    }

    let secret = secret_from_env()?;

    let expected_sig = {
        let mut mac =
            HmacSha256::new_from_slice(&secret).map_err(|_| Status::internal("hmac init"))?;
        mac.update(payload_b64.as_bytes());
        mac.finalize().into_bytes()
    };
    let got_sig = URL_SAFE_NO_PAD
        .decode(sig_b64.as_bytes())
        .map_err(|_| Status::invalid_argument("enrollment_token signature is not base64url"))?;

    if expected_sig.as_slice() != got_sig.as_slice() {
        return Err(Status::unauthenticated("enrollment_token signature mismatch"));
    }

    let payload_raw = URL_SAFE_NO_PAD
        .decode(payload_b64.as_bytes())
        .map_err(|_| Status::invalid_argument("enrollment_token payload is not base64url"))?;

    let claims: EnrollTokenClaims = serde_json::from_slice(&payload_raw)
        .map_err(|_| Status::invalid_argument("enrollment_token payload is not valid JSON"))?;

    if claims.exp <= now_unix {
        return Err(Status::unauthenticated("enrollment_token expired"));
    }

    Ok(claims)
}

