use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

use crate::auth::{AuthConfig, JwtClaims};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Serialize, Deserialize)]
struct LoginTransactionClaims {
    purpose: String,
    account_id: Uuid,
    username: String,
    nonce: Uuid,
    iat: i64,
    exp: i64,
}

pub fn issue_login_token(
    auth: &AuthConfig,
    subject: &str,
    tenant_id: Uuid,
    allowed_tenant_ids: Vec<Uuid>,
    roles: Vec<String>,
    tenant_roles: std::collections::HashMap<Uuid, Vec<String>>,
) -> Result<String, dmsx_core::DmsxError> {
    let secret = auth.jwt_secret.as_deref().ok_or_else(|| {
        dmsx_core::DmsxError::Internal("jwt secret is not configured for login issuance".into())
    })?;
    let now = Utc::now();
    let claims = JwtClaims {
        sub: subject.to_string(),
        tenant_id,
        allowed_tenant_ids,
        roles,
        tenant_roles,
        iat: now.timestamp(),
        exp: (now + Duration::hours(8)).timestamp(),
        iss: auth.jwt_issuer.clone(),
        aud: auth.jwt_audience.clone(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| dmsx_core::DmsxError::Internal("failed to issue login token".into()))
}

pub fn issue_login_transaction_token(
    auth: &AuthConfig,
    account_id: Uuid,
    username: &str,
) -> Result<String, dmsx_core::DmsxError> {
    let secret = auth.jwt_secret.as_deref().ok_or_else(|| {
        dmsx_core::DmsxError::Internal("jwt secret is not configured for login selection".into())
    })?;
    let now = Utc::now();
    let claims = LoginTransactionClaims {
        purpose: "login_select".to_string(),
        account_id,
        username: username.to_string(),
        nonce: Uuid::new_v4(),
        iat: now.timestamp(),
        exp: (now + Duration::minutes(5)).timestamp(),
    };
    let payload_raw = serde_json::to_vec(&claims).map_err(|_| {
        dmsx_core::DmsxError::Internal("failed to serialize login transaction".into())
    })?;
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload_raw);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| dmsx_core::DmsxError::Internal("login transaction hmac init".into()))?;
    mac.update(payload_b64.as_bytes());
    let sig_b64 = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    Ok(format!("v1.{payload_b64}.{sig_b64}"))
}

pub fn verify_login_transaction_token(
    auth: &AuthConfig,
    token: &str,
    username: &str,
) -> Result<Uuid, dmsx_core::DmsxError> {
    let secret = auth.jwt_secret.as_deref().ok_or_else(|| {
        dmsx_core::DmsxError::Internal("jwt secret is not configured for login selection".into())
    })?;
    let parts: Vec<&str> = token.trim().split('.').collect();
    if parts.len() != 3 || parts[0] != "v1" {
        return Err(dmsx_core::DmsxError::Unauthorized(
            "登录选择凭证无效".into(),
        ));
    }

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| dmsx_core::DmsxError::Internal("login transaction hmac init".into()))?;
    mac.update(parts[1].as_bytes());
    let sig = URL_SAFE_NO_PAD
        .decode(parts[2])
        .map_err(|_| dmsx_core::DmsxError::Unauthorized("登录选择凭证无效".into()))?;
    mac.verify_slice(&sig)
        .map_err(|_| dmsx_core::DmsxError::Unauthorized("登录选择凭证无效".into()))?;

    let payload_raw = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|_| dmsx_core::DmsxError::Unauthorized("登录选择凭证无效".into()))?;
    let claims: LoginTransactionClaims = serde_json::from_slice(&payload_raw)
        .map_err(|_| dmsx_core::DmsxError::Unauthorized("登录选择凭证无效".into()))?;
    if claims.purpose != "login_select" || claims.username != username.trim() {
        return Err(dmsx_core::DmsxError::Unauthorized(
            "登录选择凭证无效".into(),
        ));
    }
    if Utc::now().timestamp() > claims.exp {
        return Err(dmsx_core::DmsxError::Unauthorized(
            "登录选择凭证已过期，请重新登录".into(),
        ));
    }
    Ok(claims.account_id)
}

#[cfg(test)]
mod tests {
    use std::time::Duration as StdDuration;

    use dmsx_core::DmsxError;

    use super::*;
    use crate::auth::AuthMode;

    fn auth_config() -> AuthConfig {
        AuthConfig {
            mode: AuthMode::Jwt,
            jwt_secret: Some("test-login-transaction-secret".to_string()),
            jwt_issuer: None,
            jwt_audience: None,
            oidc_discovery_url: None,
            jwks_url: None,
            jwks_refresh_interval: StdDuration::from_secs(60),
            jwks_max_stale_age: StdDuration::from_secs(60),
            jwks_allow_startup_without_keys: false,
            jwks_cache: None,
        }
    }

    #[test]
    fn login_transaction_token_round_trips_for_same_account() {
        let auth = auth_config();
        let account_id = Uuid::new_v4();

        let token = issue_login_transaction_token(&auth, account_id, "alice").unwrap();

        assert_eq!(
            verify_login_transaction_token(&auth, &token, "alice").unwrap(),
            account_id
        );
    }

    #[test]
    fn login_transaction_token_rejects_wrong_username() {
        let auth = auth_config();
        let token = issue_login_transaction_token(&auth, Uuid::new_v4(), "alice").unwrap();

        let err = verify_login_transaction_token(&auth, &token, "bob").unwrap_err();

        assert!(matches!(err, DmsxError::Unauthorized(_)));
    }

    #[test]
    fn login_transaction_token_rejects_tampered_signature() {
        let auth = auth_config();
        let token = issue_login_transaction_token(&auth, Uuid::new_v4(), "alice").unwrap();
        let tampered = format!("{token}a");

        let err = verify_login_transaction_token(&auth, &tampered, "alice").unwrap_err();

        assert!(matches!(err, DmsxError::Unauthorized(_)));
    }
}
