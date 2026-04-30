use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, HeaderMap, Method},
    middleware::Next,
    response::{IntoResponse, Response},
};
use dmsx_core::DmsxError;
use jsonwebtoken::{decode, decode_header, jwk::JwkSet, Algorithm, DecodingKey, Validation};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::sleep;
use uuid::Uuid;

use crate::state::AppState;
use crate::tenant_rbac::builtin_role_permissions;

const DEFAULT_DEV_JWT_SECRET: &str = "dmsx-dev-jwt-secret-change-me-please";

#[derive(Clone, Debug)]
pub struct AuthConfig {
    pub mode: AuthMode,
    pub jwt_secret: Option<String>,
    pub jwt_issuer: Option<String>,
    pub jwt_audience: Option<String>,
    pub oidc_discovery_url: Option<String>,
    pub jwks_url: Option<String>,
    pub jwks_refresh_interval: Duration,
    pub jwks_max_stale_age: Duration,
    pub jwks_allow_startup_without_keys: bool,
    pub jwks_cache: Option<Arc<RwLock<JwksCache>>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthMode {
    Disabled,
    Jwt,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub tenant_id: Uuid,
    /// 额外可访问的租户 ID。为空时仅允许 [`Self::tenant_id`]（兼容旧令牌）。
    /// 非空时，有效集合为 `tenant_id` ∪ `allowed_tenant_ids`；带 `{tenant_id}` 的 URL 须落在该集合内。
    #[serde(default)]
    pub allowed_tenant_ids: Vec<Uuid>,
    #[serde(default)]
    pub roles: Vec<String>,
    /// 按租户覆盖 RBAC：键为租户 UUID（JSON 中为字符串键）。若当前路径租户 **存在键**（含空数组），则仅使用该数组作为本请求角色；
    /// 若 **无键**，则回退到令牌级 [`Self::roles`]。
    #[serde(default)]
    pub tenant_roles: HashMap<Uuid, Vec<String>>,
    pub exp: i64,
    pub iat: i64,
    pub iss: Option<String>,
    pub aud: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AuthContext {
    pub subject: String,
    pub tenant_id: Uuid,
    pub roles: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResourceKind {
    GlobalConfig,
    Stats,
    Devices,
    Policies,
    Commands,
    DeviceShadow,
    Artifacts,
    Compliance,
    RemoteDesktop,
    AiAssist,
    GenericTenantResource,
}

#[derive(Debug, Deserialize)]
struct OidcDiscoveryDocument {
    issuer: Option<String>,
    jwks_uri: String,
}

#[derive(Debug, Default)]
pub struct JwksCache {
    pub jwks: Option<Arc<JwkSet>>,
    pub fetched_at: Option<Instant>,
    pub last_refresh_error: Option<String>,
    pub refresh_failures: u64,
    pub stale_uses: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct AuthReadiness {
    pub mode: String,
    pub ready: bool,
    pub status: String,
    pub jwks: Option<JwksReadiness>,
}

#[derive(Clone, Debug, Serialize)]
pub struct JwksReadiness {
    pub has_keys: bool,
    pub stale: bool,
    pub within_stale_window: bool,
    pub startup_degraded: bool,
    pub refresh_failures: u64,
    pub stale_uses: u64,
    pub cache_age_seconds: Option<u64>,
    pub last_refresh_error: Option<String>,
}

impl AuthConfig {
    pub fn from_env() -> Self {
        let mode = match std::env::var("DMSX_API_AUTH_MODE")
            .unwrap_or_else(|_| "disabled".to_string())
            .to_ascii_lowercase()
            .as_str()
        {
            "disabled" | "off" | "none" => AuthMode::Disabled,
            "jwt" | "bearer" => AuthMode::Jwt,
            other => {
                tracing::warn!("unknown DMSX_API_AUTH_MODE '{other}', falling back to disabled");
                AuthMode::Disabled
            }
        };

        let jwt_secret = match mode {
            AuthMode::Disabled => None,
            AuthMode::Jwt => Some(std::env::var("DMSX_API_JWT_SECRET").unwrap_or_else(|_| {
                tracing::warn!("DMSX_API_JWT_SECRET missing, using development fallback secret");
                DEFAULT_DEV_JWT_SECRET.to_string()
            })),
        };
        let jwt_issuer = optional_env("DMSX_API_JWT_ISSUER");
        let jwt_audience = optional_env("DMSX_API_JWT_AUDIENCE");
        let oidc_discovery_url = optional_env("DMSX_API_OIDC_DISCOVERY_URL");
        let jwks_url = optional_env("DMSX_API_JWKS_URL");
        let jwks_refresh_interval = Duration::from_secs(
            std::env::var("DMSX_API_JWKS_REFRESH_SECONDS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(300),
        );
        let jwks_max_stale_age = Duration::from_secs(
            std::env::var("DMSX_API_JWKS_MAX_STALE_SECONDS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(3600),
        );
        let jwks_allow_startup_without_keys =
            std::env::var("DMSX_API_JWKS_ALLOW_STARTUP_WITHOUT_KEYS")
                .ok()
                .map(|value| {
                    matches!(
                        value.to_ascii_lowercase().as_str(),
                        "1" | "true" | "yes" | "on"
                    )
                })
                .unwrap_or(false);

        Self {
            mode,
            jwt_secret,
            jwt_issuer,
            jwt_audience,
            oidc_discovery_url,
            jwks_url,
            jwks_refresh_interval,
            jwks_max_stale_age,
            jwks_allow_startup_without_keys,
            jwks_cache: None,
        }
    }
}

pub async fn load_auth_config_from_env() -> Result<AuthConfig, DmsxError> {
    let mut config = AuthConfig::from_env();

    if config.mode == AuthMode::Disabled {
        return Ok(config);
    }

    if let Some(discovery_url) = config.oidc_discovery_url.clone() {
        let discovery = fetch_oidc_discovery(&discovery_url).await?;
        apply_oidc_discovery(&mut config, discovery);
    }

    // If we validate tokens via JWKS, require issuer + audience pinning.
    // OIDC discovery can fill `jwt_issuer`, but `jwt_audience` must be configured explicitly.
    if config.jwks_url.is_some() {
        if config
            .jwt_issuer
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            return Err(DmsxError::Internal(
                "JWKS auth enabled but DMSX_API_JWT_ISSUER is missing".into(),
            ));
        }
        if config
            .jwt_audience
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            return Err(DmsxError::Internal(
                "JWKS auth enabled but DMSX_API_JWT_AUDIENCE is missing".into(),
            ));
        }
    }

    initialize_jwks_cache(&mut config).await?;

    Ok(config)
}

pub fn spawn_jwks_refresh_task(config: AuthConfig) {
    if config.mode != AuthMode::Jwt {
        return;
    }
    if config.jwks_cache.is_none() || config.jwks_url.is_none() {
        return;
    }

    tokio::spawn(async move {
        loop {
            sleep(config.jwks_refresh_interval).await;
            if let Err(err) = refresh_jwks_cache(&config).await {
                tracing::warn!(error = %err, "background JWKS refresh failed");
            }
        }
    });
}

pub async fn auth_readiness(config: &AuthConfig) -> AuthReadiness {
    match config.mode {
        AuthMode::Disabled => AuthReadiness {
            mode: "disabled".to_string(),
            ready: true,
            status: "disabled".to_string(),
            jwks: None,
        },
        AuthMode::Jwt if config.jwks_url.is_none() => AuthReadiness {
            mode: "jwt".to_string(),
            ready: true,
            status: "ready".to_string(),
            jwks: None,
        },
        AuthMode::Jwt => {
            let Some(cache) = config.jwks_cache.as_ref() else {
                return AuthReadiness {
                    mode: "jwt".to_string(),
                    ready: false,
                    status: "not_ready".to_string(),
                    jwks: None,
                };
            };

            let guard = cache.read().await;
            let cache_age_seconds = guard.fetched_at.map(|at| at.elapsed().as_secs());
            let stale = jwks_cache_is_stale(guard.fetched_at, config.jwks_refresh_interval);
            let within_stale_window = can_use_stale_jwks(&guard, config.jwks_max_stale_age);
            let startup_degraded = guard.jwks.is_none() && guard.last_refresh_error.is_some();
            let ready = guard.jwks.is_some() && (!stale || within_stale_window);
            let status = if !ready {
                "not_ready"
            } else if stale || guard.last_refresh_error.is_some() {
                "degraded"
            } else {
                "ready"
            };

            AuthReadiness {
                mode: "jwt".to_string(),
                ready,
                status: status.to_string(),
                jwks: Some(JwksReadiness {
                    has_keys: guard.jwks.is_some(),
                    stale,
                    within_stale_window,
                    startup_degraded,
                    refresh_failures: guard.refresh_failures,
                    stale_uses: guard.stale_uses,
                    cache_age_seconds,
                    last_refresh_error: guard.last_refresh_error.clone(),
                }),
            }
        }
    }
}

impl AuthContext {
    fn from_claims(
        claims: JwtClaims,
        active_tenant_id: Uuid,
        effective_roles: Vec<String>,
    ) -> Self {
        Self {
            subject: claims.sub,
            tenant_id: active_tenant_id,
            roles: effective_roles,
        }
    }

    pub fn is_platform_admin(&self) -> bool {
        self.roles.iter().any(|r| r == "PlatformAdmin")
    }

    pub fn has_platform_scope(&self) -> bool {
        self.roles
            .iter()
            .any(|r| matches!(r.as_str(), "PlatformAdmin" | "PlatformViewer"))
    }

    /// NATS JetStream 命令回执入库：固定 subject，租户来自消息体；用于 RLS 会话变量与审计 actor。
    pub fn nats_jetstream_command_result(tenant_id: Uuid) -> Self {
        Self {
            subject: "nats-jetstream:command-result".to_string(),
            tenant_id,
            roles: vec!["TenantAdmin".to_string()],
        }
    }
}

fn jwt_permitted_tenant_ids(claims: &JwtClaims) -> HashSet<Uuid> {
    let mut ids = HashSet::new();
    ids.insert(claims.tenant_id);
    for tid in &claims.allowed_tenant_ids {
        ids.insert(*tid);
    }
    ids
}

fn effective_roles_for_tenant(claims: &JwtClaims, active_tenant: Uuid) -> Vec<String> {
    if let Some(roles) = claims.tenant_roles.get(&active_tenant) {
        return roles.clone();
    }
    claims
        .roles
        .iter()
        .filter(|role| !matches!(role.as_str(), "PlatformAdmin" | "PlatformViewer"))
        .cloned()
        .collect()
}

fn effective_roles_for_request(claims: &JwtClaims, path_tenant_id: Option<Uuid>) -> Vec<String> {
    match path_tenant_id {
        Some(active_tenant) => effective_roles_for_tenant(claims, active_tenant),
        None => claims.roles.clone(),
    }
}

fn disabled_auth_context(path: &str) -> AuthContext {
    match tenant_id_from_path(path) {
        Some(tid) => AuthContext {
            subject: "auth-disabled".into(),
            tenant_id: tid,
            roles: vec!["TenantAdmin".to_string()],
        },
        None => AuthContext {
            subject: "auth-disabled".into(),
            tenant_id: Uuid::nil(),
            roles: vec!["PlatformAdmin".to_string()],
        },
    }
}

pub async fn auth_middleware(
    State(st): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    if is_public_path(request.uri().path()) {
        return next.run(request).await;
    }
    if st.auth.mode == AuthMode::Disabled {
        let path = request.uri().path().to_string();
        request
            .extensions_mut()
            .insert(disabled_auth_context(&path));
        return next.run(request).await;
    }

    if let Some(device_ctx) = try_device_writeback_auth(
        &st,
        request.method(),
        request.uri().path(),
        request.headers(),
    )
    .await
    {
        match device_ctx {
            Ok(ctx) => {
                request.extensions_mut().insert(ctx);
                return next.run(request).await;
            }
            Err(err) => return err.into_response(),
        }
    }

    let token = match bearer_token(request.headers()) {
        Ok(token) => token,
        Err(err) => return err.into_response(),
    };

    let claims = match decode_token(&st.auth, token).await {
        Ok(claims) => claims,
        Err(err) => return err.into_response(),
    };

    let permitted = jwt_permitted_tenant_ids(&claims);
    let path_tenant_id = tenant_id_from_path(request.uri().path());
    let active_tenant_id = match path_tenant_id {
        Some(tid) => {
            if !permitted.contains(&tid) {
                return DmsxError::Forbidden(
                    "tenant in URL is not permitted for this token".into(),
                )
                .into_response();
            }
            tid
        }
        None => claims.tenant_id,
    };

    let is_platform_request = path_tenant_id.is_none() && is_platform_path(request.uri().path());
    let jwt_roles = effective_roles_for_request(&claims, path_tenant_id);
    let (effective_roles, custom_roles) = if is_platform_request {
        (claims.roles.clone(), Vec::new())
    } else {
        let binding_roles = load_effective_binding_roles(&st, active_tenant_id, &claims.sub).await;
        let effective_roles = if binding_roles.is_empty() {
            jwt_roles
        } else {
            binding_roles
        };
        let custom_roles = load_effective_custom_roles(&st, active_tenant_id).await;
        (effective_roles, custom_roles)
    };
    if let Err(err) = authorize_request(
        request.method(),
        request.uri().path(),
        &effective_roles,
        &custom_roles,
    ) {
        return err.into_response();
    }

    request.extensions_mut().insert(AuthContext::from_claims(
        claims,
        active_tenant_id,
        effective_roles,
    ));
    next.run(request).await
}

fn is_public_path(path: &str) -> bool {
    matches!(path, "/health" | "/ready")
}

fn is_platform_path(path: &str) -> bool {
    path == "/v1/tenants" || path.starts_with("/v1/config/")
}

async fn try_device_writeback_auth(
    st: &AppState,
    method: &Method,
    path: &str,
    headers: &HeaderMap,
) -> Option<Result<AuthContext, DmsxError>> {
    let token = headers
        .get("x-dmsx-device-token")
        .and_then(|value| value.to_str().ok())?
        .trim();
    if token.is_empty() {
        return Some(Err(DmsxError::Unauthorized("empty device token".into())));
    }

    let segments = path.trim_start_matches('/').split('/').collect::<Vec<_>>();
    if segments.len() < 4 || segments[0] != "v1" || segments[1] != "tenants" {
        return None;
    }
    let tid = match Uuid::parse_str(segments[2]) {
        Ok(tid) => tid,
        Err(_) => {
            return Some(Err(DmsxError::Validation(
                "invalid tenant_id in path".into(),
            )))
        }
    };

    match (method, segments.as_slice()) {
        (&Method::PATCH, ["v1", "tenants", _, "devices", did])
        | (&Method::PATCH, ["v1", "tenants", _, "devices", did, "shadow", "reported"])
        | (&Method::GET, ["v1", "tenants", _, "devices", did, "commands"]) => {
            let did = match Uuid::parse_str(did) {
                Ok(did) => did,
                Err(_) => {
                    return Some(Err(DmsxError::Validation(
                        "invalid device_id in path".into(),
                    )))
                }
            };
            Some(crate::services::devices::verify_device_writeback_token(st, tid, did, token).await)
        }
        (&Method::PATCH, ["v1", "tenants", _, "commands", cid, "status"])
        | (&Method::POST, ["v1", "tenants", _, "commands", cid, "result"]) => {
            let cid = match Uuid::parse_str(cid) {
                Ok(cid) => cid,
                Err(_) => {
                    return Some(Err(DmsxError::Validation(
                        "invalid command_id in path".into(),
                    )))
                }
            };
            Some(
                crate::services::devices::verify_device_command_writeback_token(
                    st, tid, cid, token,
                )
                .await,
            )
        }
        _ => None,
    }
}

fn optional_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

async fn load_effective_custom_roles(
    st: &AppState,
    active_tenant_id: Uuid,
) -> Vec<crate::dto::TenantCustomRole> {
    if active_tenant_id.is_nil() {
        return Vec::new();
    }

    if let Some(cached) = st
        .tenant_custom_roles
        .read()
        .await
        .get(&active_tenant_id)
        .cloned()
    {
        return cached;
    }

    let loaded = crate::tenant_rbac::load_custom_roles_from_db(&st.db, active_tenant_id)
        .await
        .unwrap_or_default();
    st.tenant_custom_roles
        .write()
        .await
        .insert(active_tenant_id, loaded.clone());
    loaded
}

async fn load_effective_binding_roles(
    st: &AppState,
    active_tenant_id: Uuid,
    subject: &str,
) -> Vec<String> {
    if active_tenant_id.is_nil() || subject.trim().is_empty() {
        return Vec::new();
    }

    if let Some(cached) = st
        .tenant_role_bindings
        .read()
        .await
        .get(&active_tenant_id)
        .cloned()
    {
        return cached
            .into_iter()
            .find(|binding| binding.subject == subject)
            .map(|binding| binding.roles)
            .unwrap_or_default();
    }

    let loaded = crate::tenant_rbac::load_role_bindings_from_db(&st.db, active_tenant_id)
        .await
        .unwrap_or_default();
    let matched = loaded
        .iter()
        .find(|binding| binding.subject == subject)
        .map(|binding| binding.roles.clone())
        .unwrap_or_default();
    st.tenant_role_bindings
        .write()
        .await
        .insert(active_tenant_id, loaded);
    matched
}

fn authorize_request(
    method: &Method,
    path: &str,
    roles: &[String],
    custom_roles: &[crate::dto::TenantCustomRole],
) -> Result<(), DmsxError> {
    if !requires_rbac(path) {
        return Ok(());
    }

    if roles.is_empty() {
        return Err(DmsxError::Forbidden(
            "JWT roles are required for RBAC-protected routes".into(),
        ));
    }

    let resource = classify_resource(path);
    let read_only = is_read_request(method);
    if roles
        .iter()
        .any(|role| is_role_allowed(role, resource, read_only, custom_roles))
    {
        return Ok(());
    }

    Err(DmsxError::Forbidden(format!(
        "roles {:?} are not allowed to {} {:?}",
        roles,
        if read_only { "read" } else { "write" },
        resource
    )))
}

fn requires_rbac(path: &str) -> bool {
    if path == "/v1/auth/logout" {
        return false;
    }
    path.starts_with("/v1/")
}

fn is_read_request(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

fn classify_resource(path: &str) -> ResourceKind {
    if is_platform_path(path) {
        return ResourceKind::GlobalConfig;
    }
    if path.ends_with("/stats") {
        return ResourceKind::Stats;
    }
    if path.contains("/compliance/findings") {
        return ResourceKind::Compliance;
    }
    if path.contains("/desktop/session") {
        return ResourceKind::RemoteDesktop;
    }
    if path.contains("/shadow") {
        return ResourceKind::DeviceShadow;
    }
    if path.contains("/commands") || path.contains("/actions") {
        return ResourceKind::Commands;
    }
    if path.contains("/policies") {
        return ResourceKind::Policies;
    }
    if path.contains("/artifacts") {
        return ResourceKind::Artifacts;
    }
    if path.contains("/devices") {
        return ResourceKind::Devices;
    }
    if path.contains("/ai/") {
        return ResourceKind::AiAssist;
    }
    ResourceKind::GenericTenantResource
}

fn permission_for_resource(resource: ResourceKind, read_only: bool) -> &'static str {
    match resource {
        ResourceKind::GlobalConfig => {
            if read_only {
                "platform.read"
            } else {
                "platform.write"
            }
        }
        ResourceKind::Stats => {
            if read_only {
                "stats.read"
            } else {
                "stats.write"
            }
        }
        ResourceKind::Devices => {
            if read_only {
                "devices.read"
            } else {
                "devices.write"
            }
        }
        ResourceKind::Policies => {
            if read_only {
                "policies.read"
            } else {
                "policies.write"
            }
        }
        ResourceKind::Commands => {
            if read_only {
                "commands.read"
            } else {
                "commands.write"
            }
        }
        ResourceKind::DeviceShadow => {
            if read_only {
                "device_shadow.read"
            } else {
                "device_shadow.write"
            }
        }
        ResourceKind::Artifacts => {
            if read_only {
                "artifacts.read"
            } else {
                "artifacts.write"
            }
        }
        ResourceKind::Compliance => {
            if read_only {
                "compliance.read"
            } else {
                "compliance.write"
            }
        }
        ResourceKind::RemoteDesktop => {
            if read_only {
                "remote_desktop.read"
            } else {
                "remote_desktop.write"
            }
        }
        ResourceKind::AiAssist => {
            if read_only {
                "ai_assist.read"
            } else {
                "ai_assist.write"
            }
        }
        ResourceKind::GenericTenantResource => {
            if read_only {
                "generic_tenant_resource.read"
            } else {
                "generic_tenant_resource.write"
            }
        }
    }
}

fn custom_role_allows(
    role: &str,
    resource: ResourceKind,
    read_only: bool,
    custom_roles: &[crate::dto::TenantCustomRole],
) -> bool {
    let Some(custom_role) = custom_roles.iter().find(|item| item.name == role) else {
        return false;
    };
    let permission = permission_for_resource(resource, read_only);
    custom_role
        .permissions
        .iter()
        .any(|item| item == permission)
}

fn is_role_allowed(
    role: &str,
    resource: ResourceKind,
    read_only: bool,
    custom_roles: &[crate::dto::TenantCustomRole],
) -> bool {
    if builtin_role_permissions(role)
        .iter()
        .any(|permission| *permission == permission_for_resource(resource, read_only))
    {
        return true;
    }

    if custom_role_allows(role, resource, read_only, custom_roles) {
        return true;
    }

    match role {
        "PlatformAdmin" => true,
        "PlatformViewer" => read_only && matches!(resource, ResourceKind::GlobalConfig),
        "TenantAdmin" => !matches!(resource, ResourceKind::GlobalConfig),
        "SiteAdmin" => site_admin_allows(resource, read_only),
        "Operator" => operator_allows(resource, read_only),
        "Auditor" => auditor_allows(resource, read_only),
        "ReadOnly" => read_only_allows(resource, read_only),
        _ => false,
    }
}

fn site_admin_allows(resource: ResourceKind, read_only: bool) -> bool {
    match resource {
        ResourceKind::GlobalConfig => false,
        ResourceKind::Policies | ResourceKind::Artifacts | ResourceKind::AiAssist => read_only,
        ResourceKind::Compliance | ResourceKind::Stats => read_only,
        ResourceKind::Devices
        | ResourceKind::Commands
        | ResourceKind::DeviceShadow
        | ResourceKind::RemoteDesktop
        | ResourceKind::GenericTenantResource => true,
    }
}

fn operator_allows(resource: ResourceKind, read_only: bool) -> bool {
    match resource {
        ResourceKind::GlobalConfig => false,
        ResourceKind::Policies | ResourceKind::Artifacts | ResourceKind::AiAssist => read_only,
        ResourceKind::Compliance | ResourceKind::Stats => read_only,
        ResourceKind::Devices
        | ResourceKind::Commands
        | ResourceKind::DeviceShadow
        | ResourceKind::RemoteDesktop => true,
        ResourceKind::GenericTenantResource => read_only,
    }
}

fn auditor_allows(resource: ResourceKind, read_only: bool) -> bool {
    read_only
        && !matches!(
            resource,
            ResourceKind::GlobalConfig | ResourceKind::RemoteDesktop | ResourceKind::AiAssist
        )
}

fn read_only_allows(resource: ResourceKind, read_only: bool) -> bool {
    read_only
        && !matches!(
            resource,
            ResourceKind::GlobalConfig | ResourceKind::RemoteDesktop | ResourceKind::AiAssist
        )
}

fn bearer_token(headers: &HeaderMap) -> Result<&str, DmsxError> {
    let value = headers
        .get(AUTHORIZATION)
        .ok_or_else(|| DmsxError::Unauthorized("missing Authorization header".into()))?;
    let raw = value
        .to_str()
        .map_err(|_| DmsxError::Unauthorized("invalid Authorization header".into()))?;
    let token = raw
        .strip_prefix("Bearer ")
        .ok_or_else(|| DmsxError::Unauthorized("expected Bearer token".into()))?;

    if token.is_empty() {
        return Err(DmsxError::Unauthorized("empty bearer token".into()));
    }

    Ok(token)
}

async fn decode_token(config: &AuthConfig, token: &str) -> Result<JwtClaims, DmsxError> {
    if config.jwks_cache.is_some() {
        return decode_token_with_jwks(config, token).await;
    }

    let secret = config
        .jwt_secret
        .as_deref()
        .ok_or_else(|| DmsxError::Internal("JWT auth enabled without secret".into()))?;
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.validate_aud = false;
    validation.required_spec_claims = ["exp"].into_iter().map(str::to_string).collect();

    let claims = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|err| {
        tracing::warn!("JWT decode failed: {err}");
        DmsxError::Unauthorized("invalid bearer token".into())
    })?;

    validate_claim_expectations(&claims, config)?;

    Ok(claims)
}

async fn decode_token_with_jwks(config: &AuthConfig, token: &str) -> Result<JwtClaims, DmsxError> {
    let header = decode_header(token)
        .map_err(|_| DmsxError::Unauthorized("invalid bearer token header".into()))?;
    let kid = header
        .kid
        .as_deref()
        .ok_or_else(|| DmsxError::Unauthorized("missing JWT kid".into()))?;
    let decoding_key = if let Some(jwk) = get_jwks(config, false).await?.find(kid) {
        DecodingKey::from_jwk(jwk)
            .map_err(|_| DmsxError::Unauthorized("failed to build decoding key from JWKS".into()))?
    } else {
        let refreshed_jwks = get_jwks(config, true).await?;
        let refreshed_jwk = refreshed_jwks
            .find(kid)
            .ok_or_else(|| DmsxError::Unauthorized(format!("unknown JWT kid '{kid}'")))?;
        DecodingKey::from_jwk(refreshed_jwk)
            .map_err(|_| DmsxError::Unauthorized("failed to build decoding key from JWKS".into()))?
    };

    let algorithm = header.alg;
    if matches!(
        algorithm,
        Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512
    ) {
        tracing::warn!("using symmetric algorithm {:?} via JWKS", algorithm);
    }

    let mut validation = Validation::new(algorithm);
    validation.validate_exp = true;
    validation.validate_aud = false;
    validation.required_spec_claims = ["exp"].into_iter().map(str::to_string).collect();

    let claims = decode::<JwtClaims>(token, &decoding_key, &validation)
        .map(|data| data.claims)
        .map_err(|err| {
            tracing::warn!("JWT decode via JWKS failed: {err}");
            DmsxError::Unauthorized("invalid bearer token".into())
        })?;

    validate_claim_expectations(&claims, config)?;

    Ok(claims)
}

async fn get_jwks(config: &AuthConfig, force_refresh: bool) -> Result<Arc<JwkSet>, DmsxError> {
    let cache = config
        .jwks_cache
        .as_ref()
        .ok_or_else(|| DmsxError::Internal("JWKS auth enabled without cache".into()))?;

    {
        let guard = cache.read().await;
        let should_refresh = force_refresh
            || guard.jwks.is_none()
            || jwks_cache_is_stale(guard.fetched_at, config.jwks_refresh_interval);
        if !should_refresh {
            if let Some(jwks) = &guard.jwks {
                return Ok(jwks.clone());
            }
        }
    }

    let mut guard = cache.write().await;
    let should_refresh = force_refresh
        || guard.jwks.is_none()
        || jwks_cache_is_stale(guard.fetched_at, config.jwks_refresh_interval);
    if should_refresh {
        refresh_jwks_cache_guard(config, &mut guard).await?;
    }

    guard
        .jwks
        .clone()
        .ok_or_else(|| DmsxError::Internal("JWKS cache missing key set after refresh".into()))
}

async fn refresh_jwks_cache(config: &AuthConfig) -> Result<(), DmsxError> {
    let cache = config
        .jwks_cache
        .as_ref()
        .ok_or_else(|| DmsxError::Internal("JWKS auth enabled without cache".into()))?;
    let mut guard = cache.write().await;
    refresh_jwks_cache_guard(config, &mut guard).await
}

async fn initialize_jwks_cache(config: &mut AuthConfig) -> Result<(), DmsxError> {
    let Some(jwks_url) = config.jwks_url.clone() else {
        return Ok(());
    };

    match fetch_jwks(&jwks_url).await {
        Ok(jwks) => {
            config.jwks_cache = Some(Arc::new(RwLock::new(JwksCache {
                jwks: Some(Arc::new(jwks)),
                fetched_at: Some(Instant::now()),
                last_refresh_error: None,
                refresh_failures: 0,
                stale_uses: 0,
            })));
            Ok(())
        }
        Err(err) if config.jwks_allow_startup_without_keys => {
            tracing::warn!("initial JWKS fetch failed, starting with empty cache: {err}");
            config.jwks_cache = Some(Arc::new(RwLock::new(JwksCache {
                jwks: None,
                fetched_at: None,
                last_refresh_error: Some(err.to_string()),
                refresh_failures: 1,
                stale_uses: 0,
            })));
            Ok(())
        }
        Err(err) => Err(err),
    }
}

async fn refresh_jwks_cache_guard(
    config: &AuthConfig,
    guard: &mut JwksCache,
) -> Result<(), DmsxError> {
    let jwks_url = config
        .jwks_url
        .as_deref()
        .ok_or_else(|| DmsxError::Internal("JWKS auth enabled without jwks_url".into()))?;
    match fetch_jwks(jwks_url).await {
        Ok(jwks) => {
            guard.jwks = Some(Arc::new(jwks));
            guard.fetched_at = Some(Instant::now());
            guard.last_refresh_error = None;
            tracing::info!("JWKS cache refreshed");
            Ok(())
        }
        Err(err) if can_use_stale_jwks(guard, config.jwks_max_stale_age) => {
            let message = err.to_string();
            guard.refresh_failures += 1;
            guard.stale_uses += 1;
            tracing::warn!(
                error = %message,
                refresh_failures = guard.refresh_failures,
                stale_uses = guard.stale_uses,
                "JWKS refresh failed, using stale cache"
            );
            guard.last_refresh_error = Some(message);
            Ok(())
        }
        Err(err) => {
            guard.refresh_failures += 1;
            guard.last_refresh_error = Some(err.to_string());
            Err(err)
        }
    }
}

fn jwks_cache_is_stale(fetched_at: Option<Instant>, refresh_interval: Duration) -> bool {
    match fetched_at {
        Some(at) => at.elapsed() >= refresh_interval,
        None => true,
    }
}

fn can_use_stale_jwks(cache: &JwksCache, max_stale_age: Duration) -> bool {
    cache.jwks.is_some()
        && matches!(
            cache.fetched_at,
            Some(fetched_at) if fetched_at.elapsed() <= max_stale_age
        )
}

fn validate_claim_expectations(claims: &JwtClaims, config: &AuthConfig) -> Result<(), DmsxError> {
    if let Some(expected_issuer) = config.jwt_issuer.as_deref() {
        match claims.iss.as_deref() {
            Some(actual_issuer) if actual_issuer == expected_issuer => {}
            Some(_) => return Err(DmsxError::Unauthorized("JWT issuer mismatch".into())),
            None => return Err(DmsxError::Unauthorized("missing JWT issuer claim".into())),
        }
    }

    if let Some(expected_audience) = config.jwt_audience.as_deref() {
        match claims.aud.as_deref() {
            Some(actual_audience) if actual_audience == expected_audience => {}
            Some(_) => return Err(DmsxError::Unauthorized("JWT audience mismatch".into())),
            None => return Err(DmsxError::Unauthorized("missing JWT audience claim".into())),
        }
    }

    Ok(())
}

async fn fetch_oidc_discovery(discovery_url: &str) -> Result<OidcDiscoveryDocument, DmsxError> {
    let url = Url::parse(discovery_url)
        .map_err(|err| DmsxError::Internal(format!("invalid OIDC discovery URL: {err}")))?;

    reqwest::get(url)
        .await
        .map_err(|err| DmsxError::Internal(format!("failed to fetch OIDC discovery: {err}")))?
        .error_for_status()
        .map_err(|err| DmsxError::Internal(format!("OIDC discovery returned error: {err}")))?
        .json::<OidcDiscoveryDocument>()
        .await
        .map_err(|err| DmsxError::Internal(format!("failed to parse OIDC discovery: {err}")))
}

async fn fetch_jwks(jwks_url: &str) -> Result<JwkSet, DmsxError> {
    let url = Url::parse(jwks_url)
        .map_err(|err| DmsxError::Internal(format!("invalid JWKS URL: {err}")))?;

    reqwest::get(url)
        .await
        .map_err(|err| DmsxError::Internal(format!("failed to fetch JWKS: {err}")))?
        .error_for_status()
        .map_err(|err| DmsxError::Internal(format!("JWKS endpoint returned error: {err}")))?
        .json::<JwkSet>()
        .await
        .map_err(|err| DmsxError::Internal(format!("failed to parse JWKS: {err}")))
}

fn apply_oidc_discovery(config: &mut AuthConfig, discovery: OidcDiscoveryDocument) {
    if config.jwt_issuer.is_none() {
        config.jwt_issuer = discovery.issuer;
    }
    config.jwks_url = Some(discovery.jwks_uri);
}

fn tenant_id_from_path(path: &str) -> Option<Uuid> {
    let mut segments = path.trim_start_matches('/').split('/');
    while let Some(segment) = segments.next() {
        if segment == "tenants" {
            return segments
                .next()
                .and_then(|value| Uuid::parse_str(value).ok());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        extract::Extension,
        http::{Request, StatusCode},
        middleware,
        response::IntoResponse,
        routing::{get, post},
        Json, Router,
    };
    use chrono::{Duration, Utc};
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::json;
    use sqlx::postgres::PgPoolOptions;
    use std::collections::HashMap;
    use tokio::net::TcpListener;
    use tower::ServiceExt;

    async fn protected(Extension(ctx): Extension<AuthContext>) -> impl IntoResponse {
        Json(json!({
            "subject": ctx.subject,
            "tenant_id": ctx.tenant_id,
            "roles": ctx.roles,
        }))
    }

    async fn health() -> impl IntoResponse {
        StatusCode::OK
    }

    async fn open() -> impl IntoResponse {
        Json(json!({ "status": "ok" }))
    }

    fn test_state(mode: AuthMode) -> AppState {
        test_state_with_claim_checks(mode, None, None)
    }

    fn test_state_with_auth(auth: AuthConfig) -> AppState {
        AppState {
            db: PgPoolOptions::new()
                .connect_lazy("postgres://dmsx:dmsx@127.0.0.1:5432/dmsx")
                .expect("lazy pool"),
            redis_url: None,
            command_jetstream: None,
            upload_token_hmac_secret: Some("test-upload-token-secret".to_string()),
            enroll_token_hmac_secret: Some("test-enroll-token-secret".to_string()),
            livekit_url: "ws://127.0.0.1:7880".to_string(),
            livekit_api_key: "key".to_string(),
            livekit_api_secret: "secret".to_string(),
            desktop_sessions: Default::default(),
            device_sessions: Default::default(),
            auth,
            tenant_custom_roles: Default::default(),
            tenant_role_bindings: Default::default(),
        }
    }

    fn test_state_with_claim_checks(
        mode: AuthMode,
        issuer: Option<&str>,
        audience: Option<&str>,
    ) -> AppState {
        let auth = AuthConfig {
            mode,
            jwt_secret: Some("test-secret-please-change-me".to_string()),
            jwt_issuer: issuer.map(str::to_string),
            jwt_audience: audience.map(str::to_string),
            oidc_discovery_url: None,
            jwks_url: None,
            jwks_refresh_interval: std::time::Duration::from_secs(300),
            jwks_max_stale_age: std::time::Duration::from_secs(3600),
            jwks_allow_startup_without_keys: false,
            jwks_cache: None,
        };
        test_state_with_auth(auth)
    }

    fn test_router(state: AppState) -> Router {
        Router::new()
            .route("/health", get(health))
            .route("/v1/tenants/{tenant_id}/open", get(open))
            .route("/v1/tenants/{tenant_id}/devices", get(protected))
            .route("/v1/tenants/{tenant_id}/commands", post(open))
            .route("/v1/tenants/{tenant_id}/policies", post(open))
            .route("/v1/config/livekit", get(open))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            ))
            .with_state(state)
    }

    fn issue_token(secret: &str, tenant_id: Uuid, roles: Vec<String>) -> String {
        issue_token_with_claims(secret, tenant_id, roles, Some("dmsx-tests"), None)
    }

    fn issue_token_with_claims(
        secret: &str,
        tenant_id: Uuid,
        roles: Vec<String>,
        issuer: Option<&str>,
        audience: Option<&str>,
    ) -> String {
        let now = Utc::now();
        let claims = JwtClaims {
            sub: "user-123".to_string(),
            tenant_id,
            allowed_tenant_ids: vec![],
            roles,
            tenant_roles: HashMap::new(),
            iat: now.timestamp(),
            exp: (now + Duration::minutes(5)).timestamp(),
            iss: issuer.map(str::to_string),
            aud: audience.map(str::to_string),
        };
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("encode token")
    }

    fn issue_token_with_allowed(
        secret: &str,
        tenant_id: Uuid,
        allowed_tenant_ids: Vec<Uuid>,
        roles: Vec<String>,
    ) -> String {
        issue_token_with_allowed_and_tenant_roles(
            secret,
            tenant_id,
            allowed_tenant_ids,
            roles,
            HashMap::new(),
        )
    }

    fn issue_token_with_allowed_and_tenant_roles(
        secret: &str,
        tenant_id: Uuid,
        allowed_tenant_ids: Vec<Uuid>,
        roles: Vec<String>,
        tenant_roles: HashMap<Uuid, Vec<String>>,
    ) -> String {
        let now = Utc::now();
        let claims = JwtClaims {
            sub: "user-123".to_string(),
            tenant_id,
            allowed_tenant_ids,
            roles,
            tenant_roles,
            iat: now.timestamp(),
            exp: (now + Duration::minutes(5)).timestamp(),
            iss: Some("dmsx-tests".to_string()),
            aud: None,
        };
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("encode token")
    }

    fn issue_token_with_tenant_roles(
        secret: &str,
        tenant_id: Uuid,
        global_roles: Vec<String>,
        tenant_roles: HashMap<Uuid, Vec<String>>,
    ) -> String {
        let now = Utc::now();
        let claims = JwtClaims {
            sub: "user-123".to_string(),
            tenant_id,
            allowed_tenant_ids: vec![],
            roles: global_roles,
            tenant_roles,
            iat: now.timestamp(),
            exp: (now + Duration::minutes(5)).timestamp(),
            iss: Some("dmsx-tests".to_string()),
            aud: None,
        };
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("encode token")
    }

    fn issue_token_with_claims_and_kid(
        secret: &str,
        tenant_id: Uuid,
        roles: Vec<String>,
        issuer: Option<&str>,
        audience: Option<&str>,
        kid: Option<&str>,
    ) -> String {
        let now = Utc::now();
        let claims = JwtClaims {
            sub: "user-123".to_string(),
            tenant_id,
            allowed_tenant_ids: vec![],
            roles,
            tenant_roles: HashMap::new(),
            iat: now.timestamp(),
            exp: (now + Duration::minutes(5)).timestamp(),
            iss: issuer.map(str::to_string),
            aud: audience.map(str::to_string),
        };
        let mut header = Header::default();
        header.kid = kid.map(str::to_string);
        encode(
            &header,
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("encode token")
    }

    fn test_jwks() -> JwkSet {
        serde_json::from_value(json!({
            "keys": [{
                "kty": "oct",
                "kid": "key-1",
                "use": "sig",
                "alg": "HS256",
                "k": "dGVzdC1zZWNyZXQtcGxlYXNlLWNoYW5nZS1tZQ"
            }]
        }))
        .expect("jwks")
    }

    async fn start_jwks_server(
        initial_body: serde_json::Value,
    ) -> (String, Arc<tokio::sync::RwLock<serde_json::Value>>) {
        let body = Arc::new(tokio::sync::RwLock::new(initial_body));
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");

        let app = Router::new().route(
            "/keys",
            get({
                let body = body.clone();
                move || {
                    let body = body.clone();
                    async move { Json(body.read().await.clone()) }
                }
            }),
        );

        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve jwks");
        });

        (format!("http://{addr}/keys"), body)
    }

    async fn response_body(response: Response) -> serde_json::Value {
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        serde_json::from_slice(&bytes).expect("json body")
    }

    #[tokio::test]
    async fn disabled_mode_allows_request_without_authorization() {
        let tenant_id = Uuid::new_v4();
        let router = test_router(test_state(AuthMode::Disabled));
        let request = Request::builder()
            .uri(format!("/v1/tenants/{tenant_id}/open"))
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn jwt_mode_rejects_missing_authorization_header() {
        let tenant_id = Uuid::new_v4();
        let router = test_router(test_state(AuthMode::Jwt));
        let request = Request::builder()
            .uri(format!("/v1/tenants/{tenant_id}/devices"))
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        let body = response_body(response).await;

        assert_eq!(body["title"], "Unauthorized");
        assert_eq!(body["detail"], "missing Authorization header");
    }

    #[tokio::test]
    async fn jwt_mode_rejects_tenant_mismatch() {
        let secret = "test-secret-please-change-me";
        let token_tenant_id = Uuid::new_v4();
        let path_tenant_id = Uuid::new_v4();
        let router = test_router(test_state(AuthMode::Jwt));
        let request = Request::builder()
            .uri(format!("/v1/tenants/{path_tenant_id}/devices"))
            .header(
                AUTHORIZATION,
                format!(
                    "Bearer {}",
                    issue_token(secret, token_tenant_id, vec!["TenantAdmin".to_string()])
                ),
            )
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        let body = response_body(response).await;

        assert_eq!(body["title"], "Forbidden");
        assert_eq!(
            body["detail"],
            "tenant in URL is not permitted for this token"
        );
    }

    #[tokio::test]
    async fn jwt_mode_accepts_path_tenant_in_allowed_tenant_ids() {
        let secret = "test-secret-please-change-me";
        let primary = Uuid::new_v4();
        let other = Uuid::new_v4();
        let router = test_router(test_state(AuthMode::Jwt));
        let request = Request::builder()
            .uri(format!("/v1/tenants/{other}/devices"))
            .header(
                AUTHORIZATION,
                format!(
                    "Bearer {}",
                    issue_token_with_allowed(
                        secret,
                        primary,
                        vec![other],
                        vec!["TenantAdmin".to_string()],
                    )
                ),
            )
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        let body = response_body(response).await;

        assert_eq!(body["tenant_id"], other.to_string());
        assert_eq!(body["subject"], "user-123");
    }

    #[tokio::test]
    async fn jwt_mode_per_tenant_roles_override_global() {
        let secret = "test-secret-please-change-me";
        let tid = Uuid::new_v4();
        let mut tenant_roles = HashMap::new();
        tenant_roles.insert(tid, vec!["ReadOnly".to_string()]);
        let token = issue_token_with_tenant_roles(
            secret,
            tid,
            vec!["TenantAdmin".to_string()],
            tenant_roles,
        );
        let router = test_router(test_state(AuthMode::Jwt));

        let request = Request::builder()
            .uri(format!("/v1/tenants/{tid}/devices"))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .expect("request");
        let response = router.clone().oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_body(response).await;
        assert_eq!(body["roles"], json!(["ReadOnly"]));

        let request = Request::builder()
            .method("POST")
            .uri(format!("/v1/tenants/{tid}/policies"))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .expect("request");
        let response = router.oneshot(request).await.expect("response");
        let body = response_body(response).await;
        assert_eq!(body["title"], "Forbidden");
    }

    #[tokio::test]
    async fn jwt_mode_per_tenant_explicit_empty_roles_denies() {
        let secret = "test-secret-please-change-me";
        let tid = Uuid::new_v4();
        let mut tenant_roles = HashMap::new();
        tenant_roles.insert(tid, vec![]);
        let router = test_router(test_state(AuthMode::Jwt));
        let request = Request::builder()
            .uri(format!("/v1/tenants/{tid}/devices"))
            .header(
                AUTHORIZATION,
                format!(
                    "Bearer {}",
                    issue_token_with_tenant_roles(
                        secret,
                        tid,
                        vec!["TenantAdmin".to_string()],
                        tenant_roles,
                    )
                ),
            )
            .body(Body::empty())
            .expect("request");
        let response = router.oneshot(request).await.expect("response");
        let body = response_body(response).await;
        assert_eq!(body["title"], "Forbidden");
        assert_eq!(
            body["detail"],
            "JWT roles are required for RBAC-protected routes"
        );
    }

    #[tokio::test]
    async fn jwt_mode_allowed_tenant_without_override_uses_global_roles() {
        let secret = "test-secret-please-change-me";
        let primary = Uuid::new_v4();
        let other = Uuid::new_v4();
        let mut tenant_roles = HashMap::new();
        tenant_roles.insert(other, vec!["ReadOnly".to_string()]);
        let token = issue_token_with_allowed_and_tenant_roles(
            secret,
            primary,
            vec![other],
            vec!["TenantAdmin".to_string()],
            tenant_roles,
        );
        let router = test_router(test_state(AuthMode::Jwt));

        let request = Request::builder()
            .uri(format!("/v1/tenants/{primary}/devices"))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .expect("request");
        let response = router.clone().oneshot(request).await.expect("response");
        let body = response_body(response).await;
        assert_eq!(body["roles"], json!(["TenantAdmin"]));

        let request = Request::builder()
            .uri(format!("/v1/tenants/{other}/devices"))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .expect("request");
        let response = router.oneshot(request).await.expect("response");
        let body = response_body(response).await;
        assert_eq!(body["roles"], json!(["ReadOnly"]));
    }

    #[tokio::test]
    async fn tenant_roles_do_not_override_platform_routes() {
        let secret = "test-secret-please-change-me";
        let primary = Uuid::new_v4();
        let other = Uuid::new_v4();
        let mut tenant_roles = HashMap::new();
        tenant_roles.insert(other, vec!["ReadOnly".to_string()]);
        let token = issue_token_with_allowed_and_tenant_roles(
            secret,
            primary,
            vec![other],
            vec!["PlatformAdmin".to_string()],
            tenant_roles,
        );
        let router = test_router(test_state(AuthMode::Jwt));

        let request = Request::builder()
            .uri("/v1/config/livekit")
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn jwt_mode_accepts_valid_token_and_injects_context() {
        let secret = "test-secret-please-change-me";
        let tenant_id = Uuid::new_v4();
        let router = test_router(test_state(AuthMode::Jwt));
        let request = Request::builder()
            .uri(format!("/v1/tenants/{tenant_id}/devices"))
            .header(
                AUTHORIZATION,
                format!(
                    "Bearer {}",
                    issue_token(secret, tenant_id, vec!["TenantAdmin".to_string()])
                ),
            )
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        let status = response.status();
        let body = response_body(response).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["tenant_id"], tenant_id.to_string());
        assert_eq!(body["subject"], "user-123");
    }

    #[tokio::test]
    async fn health_path_bypasses_authentication() {
        let router = test_router(test_state(AuthMode::Jwt));
        let request = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn read_only_role_can_access_read_route() {
        let secret = "test-secret-please-change-me";
        let tenant_id = Uuid::new_v4();
        let router = test_router(test_state(AuthMode::Jwt));
        let request = Request::builder()
            .uri(format!("/v1/tenants/{tenant_id}/devices"))
            .header(
                AUTHORIZATION,
                format!(
                    "Bearer {}",
                    issue_token(secret, tenant_id, vec!["ReadOnly".to_string()])
                ),
            )
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn read_only_role_cannot_access_write_route() {
        let secret = "test-secret-please-change-me";
        let tenant_id = Uuid::new_v4();
        let router = test_router(test_state(AuthMode::Jwt));
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!("/v1/tenants/{tenant_id}/commands"))
            .header(
                AUTHORIZATION,
                format!(
                    "Bearer {}",
                    issue_token(secret, tenant_id, vec!["ReadOnly".to_string()])
                ),
            )
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        let body = response_body(response).await;

        assert_eq!(body["title"], "Forbidden");
    }

    #[tokio::test]
    async fn operator_cannot_write_policy_routes() {
        let secret = "test-secret-please-change-me";
        let tenant_id = Uuid::new_v4();
        let router = test_router(test_state(AuthMode::Jwt));
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!("/v1/tenants/{tenant_id}/policies"))
            .header(
                AUTHORIZATION,
                format!(
                    "Bearer {}",
                    issue_token(secret, tenant_id, vec!["Operator".to_string()])
                ),
            )
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        let body = response_body(response).await;

        assert_eq!(body["title"], "Forbidden");
    }

    #[tokio::test]
    async fn tenant_admin_cannot_access_global_config_route() {
        let secret = "test-secret-please-change-me";
        let tenant_id = Uuid::new_v4();
        let router = test_router(test_state(AuthMode::Jwt));
        let request = Request::builder()
            .uri("/v1/config/livekit")
            .header(
                AUTHORIZATION,
                format!(
                    "Bearer {}",
                    issue_token(secret, tenant_id, vec!["TenantAdmin".to_string()])
                ),
            )
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        let body = response_body(response).await;

        assert_eq!(body["title"], "Forbidden");
    }

    #[tokio::test]
    async fn platform_admin_can_access_global_config_route() {
        let secret = "test-secret-please-change-me";
        let tenant_id = Uuid::new_v4();
        let router = test_router(test_state(AuthMode::Jwt));
        let request = Request::builder()
            .uri("/v1/config/livekit")
            .header(
                AUTHORIZATION,
                format!(
                    "Bearer {}",
                    issue_token(secret, tenant_id, vec!["PlatformAdmin".to_string()])
                ),
            )
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn platform_viewer_can_read_but_not_write_global_config_route() {
        let secret = "test-secret-please-change-me";
        let tenant_id = Uuid::new_v4();
        let router = test_router(test_state(AuthMode::Jwt));

        let request = Request::builder()
            .uri("/v1/config/livekit")
            .header(
                AUTHORIZATION,
                format!(
                    "Bearer {}",
                    issue_token(secret, tenant_id, vec!["PlatformViewer".to_string()])
                ),
            )
            .body(Body::empty())
            .expect("request");
        let response = router.clone().oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);

        let request = Request::builder()
            .method(Method::PUT)
            .uri("/v1/config/settings/test")
            .header(
                AUTHORIZATION,
                format!(
                    "Bearer {}",
                    issue_token(secret, tenant_id, vec!["PlatformViewer".to_string()])
                ),
            )
            .header("content-type", "application/json")
            .body(Body::from(r#"{"value":{"enabled":true}}"#))
            .expect("request");
        let response = router.oneshot(request).await.expect("response");
        let body = response_body(response).await;
        assert_eq!(body["title"], "Forbidden");
    }

    #[tokio::test]
    async fn jwt_mode_rejects_issuer_mismatch() {
        let secret = "test-secret-please-change-me";
        let tenant_id = Uuid::new_v4();
        let router = test_router(test_state_with_claim_checks(
            AuthMode::Jwt,
            Some("https://issuer.example"),
            None,
        ));
        let request = Request::builder()
            .uri(format!("/v1/tenants/{tenant_id}/devices"))
            .header(
                AUTHORIZATION,
                format!(
                    "Bearer {}",
                    issue_token_with_claims(
                        secret,
                        tenant_id,
                        vec!["TenantAdmin".to_string()],
                        Some("https://other-issuer.example"),
                        None,
                    )
                ),
            )
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        let body = response_body(response).await;

        assert_eq!(body["title"], "Unauthorized");
        assert_eq!(body["detail"], "JWT issuer mismatch");
    }

    #[tokio::test]
    async fn jwt_mode_rejects_audience_mismatch() {
        let secret = "test-secret-please-change-me";
        let tenant_id = Uuid::new_v4();
        let router = test_router(test_state_with_claim_checks(
            AuthMode::Jwt,
            None,
            Some("dmsx-api"),
        ));
        let request = Request::builder()
            .uri(format!("/v1/tenants/{tenant_id}/devices"))
            .header(
                AUTHORIZATION,
                format!(
                    "Bearer {}",
                    issue_token_with_claims(
                        secret,
                        tenant_id,
                        vec!["TenantAdmin".to_string()],
                        Some("dmsx-tests"),
                        Some("other-audience"),
                    )
                ),
            )
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        let body = response_body(response).await;

        assert_eq!(body["title"], "Unauthorized");
        assert_eq!(body["detail"], "JWT audience mismatch");
    }

    #[test]
    fn oidc_discovery_applies_missing_issuer_and_jwks_uri() {
        let mut config = AuthConfig {
            mode: AuthMode::Jwt,
            jwt_secret: None,
            jwt_issuer: None,
            jwt_audience: None,
            oidc_discovery_url: Some(
                "https://issuer.example/.well-known/openid-configuration".to_string(),
            ),
            jwks_url: None,
            jwks_refresh_interval: std::time::Duration::from_secs(300),
            jwks_max_stale_age: std::time::Duration::from_secs(3600),
            jwks_allow_startup_without_keys: false,
            jwks_cache: None,
        };

        apply_oidc_discovery(
            &mut config,
            OidcDiscoveryDocument {
                issuer: Some("https://issuer.example".to_string()),
                jwks_uri: "https://issuer.example/keys".to_string(),
            },
        );

        assert_eq!(config.jwt_issuer.as_deref(), Some("https://issuer.example"));
        assert_eq!(
            config.jwks_url.as_deref(),
            Some("https://issuer.example/keys")
        );
    }

    #[tokio::test]
    async fn jwks_mode_rejects_token_without_kid() {
        let tenant_id = Uuid::new_v4();
        let router = test_router(test_state_with_auth(AuthConfig {
            mode: AuthMode::Jwt,
            jwt_secret: None,
            jwt_issuer: None,
            jwt_audience: None,
            oidc_discovery_url: None,
            jwks_url: Some("https://issuer.example/keys".to_string()),
            jwks_refresh_interval: std::time::Duration::from_secs(300),
            jwks_max_stale_age: std::time::Duration::from_secs(3600),
            jwks_allow_startup_without_keys: false,
            jwks_cache: Some(Arc::new(RwLock::new(JwksCache {
                jwks: Some(Arc::new(test_jwks())),
                fetched_at: Some(Instant::now()),
                last_refresh_error: None,
                refresh_failures: 0,
                stale_uses: 0,
            }))),
        }));
        let request = Request::builder()
            .uri(format!("/v1/tenants/{tenant_id}/devices"))
            .header(
                AUTHORIZATION,
                format!(
                    "Bearer {}",
                    issue_token_with_claims_and_kid(
                        "test-secret-please-change-me",
                        tenant_id,
                        vec!["TenantAdmin".to_string()],
                        Some("dmsx-tests"),
                        None,
                        None,
                    )
                ),
            )
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        let body = response_body(response).await;

        assert_eq!(body["title"], "Unauthorized");
        assert_eq!(body["detail"], "missing JWT kid");
    }

    #[tokio::test]
    async fn jwks_mode_refreshes_on_unknown_kid() {
        let tenant_id = Uuid::new_v4();
        let (jwks_url, _server_state) = start_jwks_server(json!({
            "keys": [{
                "kty": "oct",
                "kid": "key-2",
                "use": "sig",
                "alg": "HS256",
                "k": "dGVzdC1zZWNyZXQtcGxlYXNlLWNoYW5nZS1tZQ"
            }]
        }))
        .await;
        let router = test_router(test_state_with_auth(AuthConfig {
            mode: AuthMode::Jwt,
            jwt_secret: None,
            jwt_issuer: None,
            jwt_audience: None,
            oidc_discovery_url: None,
            jwks_url: Some(jwks_url),
            jwks_refresh_interval: std::time::Duration::from_secs(300),
            jwks_max_stale_age: std::time::Duration::from_secs(3600),
            jwks_allow_startup_without_keys: false,
            jwks_cache: Some(Arc::new(RwLock::new(JwksCache {
                jwks: Some(Arc::new(test_jwks())),
                fetched_at: Some(Instant::now()),
                last_refresh_error: None,
                refresh_failures: 0,
                stale_uses: 0,
            }))),
        }));
        let request = Request::builder()
            .uri(format!("/v1/tenants/{tenant_id}/devices"))
            .header(
                AUTHORIZATION,
                format!(
                    "Bearer {}",
                    issue_token_with_claims_and_kid(
                        "test-secret-please-change-me",
                        tenant_id,
                        vec!["TenantAdmin".to_string()],
                        Some("dmsx-tests"),
                        None,
                        Some("key-2"),
                    )
                ),
            )
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn background_jwks_refresh_updates_cached_keys() {
        let (jwks_url, server_state) = start_jwks_server(json!({
            "keys": [{
                "kty": "oct",
                "kid": "key-1",
                "use": "sig",
                "alg": "HS256",
                "k": "dGVzdC1zZWNyZXQtcGxlYXNlLWNoYW5nZS1tZQ"
            }]
        }))
        .await;
        let config = AuthConfig {
            mode: AuthMode::Jwt,
            jwt_secret: None,
            jwt_issuer: None,
            jwt_audience: None,
            oidc_discovery_url: None,
            jwks_url: Some(jwks_url),
            jwks_refresh_interval: std::time::Duration::from_millis(25),
            jwks_max_stale_age: std::time::Duration::from_secs(3600),
            jwks_allow_startup_without_keys: false,
            jwks_cache: Some(Arc::new(RwLock::new(JwksCache {
                jwks: Some(Arc::new(test_jwks())),
                fetched_at: Some(Instant::now()),
                last_refresh_error: None,
                refresh_failures: 0,
                stale_uses: 0,
            }))),
        };

        spawn_jwks_refresh_task(config.clone());
        *server_state.write().await = json!({
            "keys": [{
                "kty": "oct",
                "kid": "key-2",
                "use": "sig",
                "alg": "HS256",
                "k": "dGVzdC1zZWNyZXQtcGxlYXNlLWNoYW5nZS1tZQ"
            }]
        });

        sleep(std::time::Duration::from_millis(60)).await;

        let jwks = get_jwks(&config, false).await.expect("jwks");
        assert!(jwks.find("key-2").is_some());
    }

    #[tokio::test]
    async fn jwks_refresh_failure_uses_recent_stale_cache() {
        let config = AuthConfig {
            mode: AuthMode::Jwt,
            jwt_secret: None,
            jwt_issuer: None,
            jwt_audience: None,
            oidc_discovery_url: None,
            jwks_url: Some("http://127.0.0.1:9/keys".to_string()),
            jwks_refresh_interval: std::time::Duration::from_secs(1),
            jwks_max_stale_age: std::time::Duration::from_secs(300),
            jwks_allow_startup_without_keys: false,
            jwks_cache: Some(Arc::new(RwLock::new(JwksCache {
                jwks: Some(Arc::new(test_jwks())),
                fetched_at: Some(Instant::now()),
                last_refresh_error: None,
                refresh_failures: 0,
                stale_uses: 0,
            }))),
        };

        refresh_jwks_cache(&config)
            .await
            .expect("stale jwks should be used");

        let guard = config.jwks_cache.as_ref().expect("cache").read().await;
        assert!(guard.jwks.as_ref().expect("jwks").find("key-1").is_some());
        assert!(guard.last_refresh_error.is_some());
    }

    #[tokio::test]
    async fn jwks_refresh_failure_rejects_expired_stale_cache() {
        let config = AuthConfig {
            mode: AuthMode::Jwt,
            jwt_secret: None,
            jwt_issuer: None,
            jwt_audience: None,
            oidc_discovery_url: None,
            jwks_url: Some("http://127.0.0.1:9/keys".to_string()),
            jwks_refresh_interval: std::time::Duration::from_secs(1),
            jwks_max_stale_age: std::time::Duration::from_secs(1),
            jwks_allow_startup_without_keys: false,
            jwks_cache: Some(Arc::new(RwLock::new(JwksCache {
                jwks: Some(Arc::new(test_jwks())),
                fetched_at: Some(Instant::now() - std::time::Duration::from_secs(5)),
                last_refresh_error: None,
                refresh_failures: 0,
                stale_uses: 0,
            }))),
        };

        let err = refresh_jwks_cache(&config)
            .await
            .expect_err("expired stale jwks should fail");
        assert!(err.to_string().contains("failed to fetch JWKS"));

        let guard = config.jwks_cache.as_ref().expect("cache").read().await;
        assert!(guard.last_refresh_error.is_some());
    }

    #[tokio::test]
    async fn initial_jwks_fetch_can_start_with_empty_cache_when_allowed() {
        let mut config = AuthConfig {
            mode: AuthMode::Jwt,
            jwt_secret: None,
            jwt_issuer: None,
            jwt_audience: None,
            oidc_discovery_url: None,
            jwks_url: Some("http://127.0.0.1:9/keys".to_string()),
            jwks_refresh_interval: std::time::Duration::from_secs(300),
            jwks_max_stale_age: std::time::Duration::from_secs(3600),
            jwks_allow_startup_without_keys: true,
            jwks_cache: None,
        };

        initialize_jwks_cache(&mut config)
            .await
            .expect("startup should allow empty jwks cache");

        let guard = config.jwks_cache.as_ref().expect("cache").read().await;
        assert!(guard.jwks.is_none());
        assert!(guard.fetched_at.is_none());
        assert!(guard.last_refresh_error.is_some());
        assert_eq!(guard.refresh_failures, 1);
    }

    #[tokio::test]
    async fn auth_readiness_reports_degraded_when_using_stale_jwks() {
        let config = AuthConfig {
            mode: AuthMode::Jwt,
            jwt_secret: None,
            jwt_issuer: None,
            jwt_audience: None,
            oidc_discovery_url: None,
            jwks_url: Some("https://issuer.example/keys".to_string()),
            jwks_refresh_interval: std::time::Duration::from_secs(1),
            jwks_max_stale_age: std::time::Duration::from_secs(300),
            jwks_allow_startup_without_keys: false,
            jwks_cache: Some(Arc::new(RwLock::new(JwksCache {
                jwks: Some(Arc::new(test_jwks())),
                fetched_at: Some(Instant::now() - std::time::Duration::from_secs(5)),
                last_refresh_error: Some("temporary failure".to_string()),
                refresh_failures: 1,
                stale_uses: 1,
            }))),
        };

        let readiness = auth_readiness(&config).await;

        assert!(readiness.ready);
        assert_eq!(readiness.status, "degraded");
        assert_eq!(readiness.jwks.as_ref().expect("jwks").stale_uses, 1);
    }

    #[tokio::test]
    async fn jwt_mode_rejects_missing_roles_for_rbac_route() {
        let secret = "test-secret-please-change-me";
        let tenant_id = Uuid::new_v4();
        let router = test_router(test_state(AuthMode::Jwt));
        let request = Request::builder()
            .uri(format!("/v1/tenants/{tenant_id}/devices"))
            .header(
                AUTHORIZATION,
                format!("Bearer {}", issue_token(secret, tenant_id, Vec::new())),
            )
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        let body = response_body(response).await;

        assert_eq!(body["title"], "Forbidden");
        assert_eq!(
            body["detail"],
            "JWT roles are required for RBAC-protected routes"
        );
    }
}
