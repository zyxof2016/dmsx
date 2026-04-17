//! `Enroll` implementation (internal beta).
//!
//! - Verifies enrollment token (HMAC).
//! - Expects `EnrollRequest.public_key_pem` to contain a **PEM CSR** (PKCS#10).
//! - Issues a client certificate signed by a configured CA.
//! - Writes SAN URI `urn:dmsx:tenant:{uuid}:device:{uuid}`.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rcgen::{
    Certificate, CertificateParams, CertificateSigningRequestParams, DnType, ExtendedKeyUsagePurpose,
    Ia5String, KeyPair, KeyUsagePurpose, SanType,
};
use tonic::Status;
use uuid::Uuid;

use crate::enroll_token;

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn cert_ttl_days_from_env() -> i64 {
    std::env::var("DMSX_GW_ENROLL_CERT_TTL_DAYS")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(30)
        .clamp(1, 3650)
}

async fn read_pem_from_path_env(var: &str) -> Result<String, Status> {
    let path = std::env::var(var)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Status::failed_precondition(format!("Enroll disabled: {var} is not set")))?;
    tokio::fs::read_to_string(path)
        .await
        .map_err(|e| Status::internal(format!("read {var}: {e}")))
}

pub async fn issue_device_cert(
    enrollment_token: &str,
    csr_pem: &str,
) -> Result<(Uuid, String, String, i64, Uuid), Status> {
    let claims = enroll_token::verify(enrollment_token, now_unix())?;
    let tenant_id = claims.tenant_id;
    let device_id = claims.device_id.unwrap_or_else(Uuid::new_v4);

    let ca_cert_pem = read_pem_from_path_env("DMSX_GW_ENROLL_CA_CERT").await?;
    let ca_key_pem = read_pem_from_path_env("DMSX_GW_ENROLL_CA_KEY").await?;

    let ca_params = CertificateParams::from_ca_cert_pem(&ca_cert_pem)
        .map_err(|e| Status::internal(format!("parse ca cert: {e}")))?;
    let ca_key = KeyPair::from_pem(&ca_key_pem)
        .map_err(|e| Status::internal(format!("parse ca key: {e}")))?;
    let ca_cert: Certificate = ca_params
        .self_signed(&ca_key)
        .map_err(|e| Status::internal(format!("build ca cert: {e}")))?;

    // `public_key_pem` is treated as CSR PEM in internal beta.
    let csr = CertificateSigningRequestParams::from_pem(csr_pem)
        .map_err(|_| Status::invalid_argument("public_key_pem must be a PEM CSR (PKCS#10)"))?;

    let mut params: CertificateParams = csr.params;

    // Override identity-related fields to prevent requester-controlled SAN spoofing.
    let uri = format!("urn:dmsx:tenant:{tenant_id}:device:{device_id}");
    let uri = Ia5String::try_from(uri.as_str())
        .map_err(|_| Status::internal("failed to build SAN URI (IA5String)"))?;
    params.subject_alt_names = vec![SanType::URI(uri)];

    params.key_usages = vec![KeyUsagePurpose::DigitalSignature, KeyUsagePurpose::KeyEncipherment];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];

    // Help ops debugging (not a security primitive).
    params
        .distinguished_name
        .push(DnType::CommonName, format!("dmsx-device-{device_id}"));

    let ttl_days = cert_ttl_days_from_env();
    let expires_unix = now_unix() + (ttl_days * 24 * 3600);

    // rcgen uses `time` internally; use system time bounds.
    params.not_before = SystemTime::now()
        .checked_sub(Duration::from_secs(60))
        .unwrap_or(SystemTime::now())
        .into();
    params.not_after = SystemTime::now()
        .checked_add(Duration::from_secs((ttl_days * 24 * 3600) as u64))
        .ok_or_else(|| Status::internal("cert ttl overflow"))?
        .into();

    let cert = params
        .signed_by(&csr.public_key, &ca_cert, &ca_key)
        .map_err(|e| Status::internal(format!("sign cert: {e}")))?;

    let issued_cert_pem = cert.pem();

    Ok((device_id, issued_cert_pem, ca_cert_pem, expires_unix, tenant_id))
}

