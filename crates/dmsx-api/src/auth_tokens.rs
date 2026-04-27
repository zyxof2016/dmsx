use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use uuid::Uuid;

use crate::auth::{AuthConfig, JwtClaims};

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
