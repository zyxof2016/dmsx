use axum::{
    middleware,
    routing::{get, patch, post},
    Router,
};
use axum::response::IntoResponse;
use sqlx::postgres::PgPoolOptions;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::{DefaultOnResponse, TraceLayer},
};

use crate::{
    auth::{load_auth_config_from_env, spawn_jwks_refresh_task},
    desktop, handlers, migrate_embedded,
    limits,
    metrics,
    services::bootstrap,
    state::AppState,
};

fn truthy_env(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

fn dmsx_api_env() -> String {
    std::env::var("DMSX_API_ENV")
        .unwrap_or_else(|_| "dev".to_string())
        .to_ascii_lowercase()
}

fn enforce_prod_guardrails(auth: &crate::auth::AuthConfig) {
    let env = dmsx_api_env();
    if env == "dev" {
        return;
    }

    let allow_insecure = truthy_env("DMSX_API_ALLOW_INSECURE_AUTH");
    if auth.mode == crate::auth::AuthMode::Disabled && !allow_insecure {
        panic!(
            "DMSX_API_AUTH_MODE=disabled is not allowed when DMSX_API_ENV='{env}'. \
             Set DMSX_API_AUTH_MODE=jwt (OIDC/JWKS) or explicitly opt in with DMSX_API_ALLOW_INSECURE_AUTH=1."
        );
    }
}

fn request_timeout_from_env() -> std::time::Duration {
    let secs = std::env::var("DMSX_API_REQUEST_TIMEOUT_SECONDS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(30);
    std::time::Duration::from_secs(secs.max(1))
}

fn concurrency_limit_from_env() -> Option<usize> {
    let enabled = truthy_env("DMSX_API_CONCURRENCY_LIMIT_ENABLED");
    if !enabled {
        return None;
    }
    let limit = std::env::var("DMSX_API_CONCURRENCY_LIMIT")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1024);
    Some(limit.max(1))
}

fn cors_layer_from_env() -> CorsLayer {
    // Production CORS policy:
    // - Set `DMSX_API_CORS_ALLOWED_ORIGINS` to a comma-separated list of origins, e.g.
    //   `https://admin.example.com,https://ops.example.com`
    // - Or set `DMSX_API_CORS_ALLOW_ALL=1` to allow all origins (dev-like).
    // - If unset and not allowing all:
    //   - in `dev` we allow all to avoid breaking local development
    //   - otherwise we allow none (browser will block cross-origin calls)
    let allow_all = truthy_env("DMSX_API_CORS_ALLOW_ALL");
    if allow_all {
        return CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);
    }

    let allowed = std::env::var("DMSX_API_CORS_ALLOWED_ORIGINS")
        .ok()
        .map(|s| s.trim().to_string());
    if let Some(s) = allowed {
        if s == "*" {
            return CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any);
        }

        let origins: Vec<axum::http::HeaderValue> = s
            .split(',')
            .map(|o| o.trim())
            .filter(|o| !o.is_empty())
            .map(|o| o.parse::<axum::http::HeaderValue>())
            .filter_map(Result::ok)
            .collect();

        return CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods(Any)
            .allow_headers(Any);
    }

    let env = dmsx_api_env();
    if env == "dev" {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(Vec::new()))
            .allow_methods(Any)
            .allow_headers(Any)
    }
}

async fn timeout_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let timeout = request_timeout_from_env();
    match tokio::time::timeout(timeout, next.run(req)).await {
        Ok(resp) => resp,
        Err(_) => {
            let pd = dmsx_core::error::ProblemDetails {
                r#type: "about:blank",
                title: "Gateway Timeout",
                status: 504,
                detail: format!("request exceeded timeout of {}s", timeout.as_secs()),
            };
            (
                axum::http::StatusCode::GATEWAY_TIMEOUT,
                [(
                    axum::http::header::CONTENT_TYPE,
                    axum::http::HeaderValue::from_static("application/problem+json"),
                )],
                axum::Json(pd),
            )
                .into_response()
        }
    }
}

pub async fn build_state_from_env() -> AppState {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://dmsx:dmsx@127.0.0.1:5432/dmsx".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await
        .expect("failed to connect to database");

    tracing::info!("connected to postgres");

    migrate_embedded::run(&pool).await;

    tracing::info!("migrations applied");

    let dev_tenant = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let livekit_url =
        std::env::var("LIVEKIT_URL").unwrap_or_else(|_| "ws://127.0.0.1:7880".to_string());
    let livekit_api_key =
        std::env::var("LIVEKIT_API_KEY").unwrap_or_else(|_| "dmsx-api-key".to_string());
    let livekit_api_secret = std::env::var("LIVEKIT_API_SECRET")
        .unwrap_or_else(|_| "dmsx-api-secret-that-is-at-least-32-chars".to_string());

    let redis_url = std::env::var("DMSX_REDIS_URL")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let upload_token_hmac_secret = std::env::var("DMSX_API_UPLOAD_TOKEN_HMAC_SECRET")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            std::env::var("DMSX_GW_UPLOAD_TOKEN_HMAC_SECRET")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        });

    let command_jetstream = crate::command_jetstream::CommandJetStream::try_from_env().await;

    let state = AppState {
        db: pool,
        redis_url,
        command_jetstream,
        upload_token_hmac_secret,
        livekit_url,
        livekit_api_key,
        livekit_api_secret,
        desktop_sessions: Arc::new(RwLock::new(HashMap::new())),
        device_sessions: Arc::new(RwLock::new(HashMap::new())),
        auth: load_auth_config_from_env()
            .await
            .expect("failed to initialize auth config"),
    };

    enforce_prod_guardrails(&state.auth);

    spawn_jwks_refresh_task(state.auth.clone());
    bootstrap::ensure_default_tenant(&state, dev_tenant, "默认租户").await;

    crate::result_jetstream_ingest::spawn_background(state.clone());

    state
}

pub fn build_router(st: AppState) -> Router {
    let cors = cors_layer_from_env();

    let rate_limit_layer = limits::tenant_rate_limit_layer_from_env();
    let set_request_id = SetRequestIdLayer::x_request_id(MakeRequestUuid);
    let propagate_request_id = PropagateRequestIdLayer::x_request_id();

    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(|req: &axum::http::Request<_>| {
            let request_id = req
                .headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("-");
            tracing::info_span!(
                "http.request",
                request_id = %request_id,
                method = %req.method(),
                uri = %req.uri(),
                version = ?req.version()
            )
        })
        .on_response(DefaultOnResponse::new().level(tracing::Level::INFO));

    let base_xcut = ServiceBuilder::new()
        .layer(set_request_id)
        .layer(propagate_request_id)
        .layer(trace_layer);

    let concurrency_limit = concurrency_limit_from_env();

    let public = Router::new()
        .route("/health", get(handlers::health))
        .route("/ready", get(handlers::ready))
        .route("/metrics", get(metrics::metrics_handler))
        .with_state(st.clone());

    let mut api = Router::new()
        .route("/v1/tenants", post(handlers::tenants_create))
        .route(
            "/v1/tenants/{tenant_id}/orgs/{org_id}/sites",
            post(handlers::sites_create),
        )
        .route(
            "/v1/tenants/{tenant_id}/sites/{site_id}/groups",
            post(handlers::groups_create),
        )
        .route("/v1/tenants/{tenant_id}/orgs", post(handlers::orgs_create))
        .route("/v1/tenants/{tenant_id}/stats", get(handlers::stats))
        .route(
            "/v1/tenants/{tenant_id}/devices",
            get(handlers::devices_list).post(handlers::devices_create),
        )
        .route(
            "/v1/tenants/{tenant_id}/devices/{device_id}",
            get(handlers::devices_get)
                .patch(handlers::devices_patch)
                .delete(handlers::devices_delete),
        )
        .route(
            "/v1/tenants/{tenant_id}/policies",
            get(handlers::policies_list).post(handlers::policies_create),
        )
        .route(
            "/v1/tenants/{tenant_id}/policies/{policy_id}",
            get(handlers::policies_get)
                .patch(handlers::policies_patch)
                .delete(handlers::policies_delete),
        )
        .route(
            "/v1/tenants/{tenant_id}/policies/{policy_id}/revisions",
            post(handlers::policy_publish),
        )
        .route(
            "/v1/tenants/{tenant_id}/policies/editor",
            post(handlers::policy_editor_create_and_publish),
        )
        .route(
            "/v1/tenants/{tenant_id}/devices/{device_id}/shadow",
            get(handlers::shadow_get),
        )
        .route(
            "/v1/tenants/{tenant_id}/devices/{device_id}/shadow/desired",
            patch(handlers::shadow_update_desired),
        )
        .route(
            "/v1/tenants/{tenant_id}/devices/{device_id}/shadow/reported",
            patch(handlers::shadow_update_reported),
        )
        .route(
            "/v1/tenants/{tenant_id}/devices/{device_id}/actions",
            post(handlers::device_action),
        )
        .route(
            "/v1/tenants/{tenant_id}/devices/{device_id}/commands",
            get(handlers::device_commands_list),
        )
        .route(
            "/v1/tenants/{tenant_id}/commands",
            get(handlers::commands_list).post(handlers::commands_create),
        )
        .route(
            "/v1/tenants/{tenant_id}/commands/{command_id}",
            get(handlers::commands_get),
        )
        .route(
            "/v1/tenants/{tenant_id}/commands/{command_id}/status",
            patch(handlers::command_status_update),
        )
        .route(
            "/v1/tenants/{tenant_id}/commands/{command_id}/result",
            get(handlers::command_result_get).post(handlers::command_result_submit),
        )
        .route(
            "/v1/tenants/{tenant_id}/commands/{command_id}/evidence-upload-token",
            post(handlers::command_evidence_upload_token_issue),
        )
        .route(
            "/v1/tenants/{tenant_id}/artifacts",
            get(handlers::artifacts_list).post(handlers::artifacts_create),
        )
        .route(
            "/v1/tenants/{tenant_id}/compliance/findings",
            get(handlers::compliance_list),
        )
        .route(
            "/v1/tenants/{tenant_id}/audit-logs",
            get(handlers::audit_logs_list),
        )
        .route(
            "/v1/tenants/{tenant_id}/devices/{device_id}/desktop/session",
            post(desktop::session_create).delete(desktop::session_delete),
        )
        .route("/v1/config/livekit", get(desktop::livekit_config))
        .route(
            "/v1/config/settings/{key}",
            get(handlers::system_settings_get).put(handlers::system_settings_put),
        )
        .route(
            "/v1/config/rbac/roles",
            get(handlers::rbac_roles_list),
        )
        .route(
            "/v1/tenants/{tenant_id}/ai/anomalies",
            post(handlers::ai_anomaly_detect),
        )
        .route(
            "/v1/tenants/{tenant_id}/ai/recommendations",
            post(handlers::ai_recommend_policies),
        )
        .route("/v1/tenants/{tenant_id}/ai/chat", post(handlers::ai_chat))
        .route(
            "/v1/tenants/{tenant_id}/ai/predictions",
            post(handlers::ai_predict_maintenance),
        )
        .layer(middleware::from_fn_with_state(
            st.clone(),
            crate::auth::auth_middleware,
        ))
        .layer(middleware::from_fn(limits::request_body_limit_middleware))
        .layer(middleware::from_fn(metrics::metrics_middleware))
        .layer(middleware::from_fn(timeout_middleware))
        .layer(cors)
        .with_state(st);

    if let Some(layer) = rate_limit_layer {
        api = api.layer(layer);
    }

    let mut public = public.layer(base_xcut.clone());
    let mut api = api.layer(base_xcut);

    if let Some(limit) = concurrency_limit {
        public = public.layer(ConcurrencyLimitLayer::new(limit));
        api = api.layer(ConcurrencyLimitLayer::new(limit));
    }

    public.merge(api)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{header::AUTHORIZATION, Request, StatusCode},
    };
    use chrono::{Duration, Utc};
    use jsonwebtoken::{encode, EncodingKey, Header};
    use once_cell::sync::Lazy;
    use serde_json::Value;
    use sqlx::postgres::PgPoolOptions;
    use std::sync::Mutex;
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::auth::{AuthConfig, AuthMode, JwtClaims, JwksCache};

    static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    fn test_state(mode: AuthMode) -> AppState {
        test_state_with_auth(AuthConfig {
            mode,
            jwt_secret: Some("test-secret-please-change-me".to_string()),
            jwt_issuer: None,
            jwt_audience: None,
            oidc_discovery_url: None,
            jwks_url: None,
            jwks_refresh_interval: std::time::Duration::from_secs(300),
            jwks_max_stale_age: std::time::Duration::from_secs(3600),
            jwks_allow_startup_without_keys: false,
            jwks_cache: None,
        })
    }

    fn test_state_with_auth(auth: AuthConfig) -> AppState {
        AppState {
            db: PgPoolOptions::new()
                .connect_lazy("postgres://dmsx:dmsx@127.0.0.1:5432/dmsx")
                .expect("lazy pool"),
            redis_url: None,
            command_jetstream: None,
            upload_token_hmac_secret: Some("test-upload-token-secret".to_string()),
            livekit_url: "ws://127.0.0.1:7880".to_string(),
            livekit_api_key: "test-livekit-key".to_string(),
            livekit_api_secret: "test-livekit-secret".to_string(),
            desktop_sessions: Arc::new(RwLock::new(HashMap::new())),
            device_sessions: Arc::new(RwLock::new(HashMap::new())),
            auth,
        }
    }

    fn issue_token(secret: &str, tenant_id: Uuid, roles: Vec<String>) -> String {
        issue_token_with_claims(secret, tenant_id, roles, Some("dmsx-api-tests"), None)
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
            sub: "integration-user".to_string(),
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

    async fn response_body(response: axum::response::Response) -> Value {
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read response body");
        serde_json::from_slice(&bytes).expect("json body")
    }

    #[tokio::test]
    async fn health_route_bypasses_auth_on_real_router() {
        let router = build_router(test_state(AuthMode::Jwt));
        let request = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        let status = response.status();
        let body = response_body(response).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ok");
        assert_eq!(body["service"], "dmsx-api");
    }

    #[tokio::test]
    async fn ready_route_bypasses_auth_on_real_router() {
        let router = build_router(test_state(AuthMode::Jwt));
        let request = Request::builder()
            .uri("/ready")
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        let status = response.status();
        let body = response_body(response).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ok");
        assert_eq!(body["auth"]["status"], "ready");
    }

    #[tokio::test]
    async fn ready_route_reports_jwks_not_ready_when_cache_is_empty() {
        let router = build_router(test_state_with_auth(AuthConfig {
            mode: AuthMode::Jwt,
            jwt_secret: None,
            jwt_issuer: None,
            jwt_audience: None,
            oidc_discovery_url: None,
            jwks_url: Some("https://issuer.example/keys".to_string()),
            jwks_refresh_interval: std::time::Duration::from_secs(300),
            jwks_max_stale_age: std::time::Duration::from_secs(3600),
            jwks_allow_startup_without_keys: true,
            jwks_cache: Some(Arc::new(RwLock::new(JwksCache {
                jwks: None,
                fetched_at: None,
                last_refresh_error: Some("startup fetch failed".to_string()),
                refresh_failures: 1,
                stale_uses: 0,
            }))),
        }));
        let request = Request::builder()
            .uri("/ready")
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        let status = response.status();
        let body = response_body(response).await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body["status"], "not_ready");
        assert_eq!(body["auth"]["ready"], false);
        assert_eq!(body["auth"]["jwks"]["startup_degraded"], true);
    }

    #[tokio::test]
    async fn request_body_limit_returns_problem_details_413() {
        std::env::set_var("DMSX_API_REQUEST_BODY_LIMIT_BYTES", "10");
        let router = build_router(test_state(AuthMode::Disabled));
        let request = Request::builder()
            .method("POST")
            .uri("/v1/tenants")
            .header("content-length", "25")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"this-is-long"}"#))
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        let status = response.status();
        let body = response_body(response).await;

        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(body["title"], "Payload Too Large");
        std::env::remove_var("DMSX_API_REQUEST_BODY_LIMIT_BYTES");
    }

    #[tokio::test]
    async fn rate_limit_returns_problem_details_429() {
        std::env::set_var("DMSX_API_RATE_LIMIT_ENABLED", "true");
        std::env::set_var("DMSX_API_RATE_LIMIT_PER_SECOND", "1");
        std::env::set_var("DMSX_API_RATE_LIMIT_BURST", "1");

        let router = build_router(test_state(AuthMode::Disabled));

        let request1 = Request::builder()
            .uri("/v1/config/livekit")
            .body(Body::empty())
            .expect("request1");
        let response1 = router.clone().oneshot(request1).await.expect("response1");
        assert_eq!(response1.status(), StatusCode::OK);

        let request2 = Request::builder()
            .uri("/v1/config/livekit")
            .body(Body::empty())
            .expect("request2");
        let response2 = router.oneshot(request2).await.expect("response2");
        let status2 = response2.status();
        let body2 = response_body(response2).await;

        assert_eq!(status2, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(body2["title"], "Too Many Requests");

        std::env::remove_var("DMSX_API_RATE_LIMIT_ENABLED");
        std::env::remove_var("DMSX_API_RATE_LIMIT_PER_SECOND");
        std::env::remove_var("DMSX_API_RATE_LIMIT_BURST");
    }

    #[tokio::test]
    async fn metrics_can_be_disabled_via_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("DMSX_API_METRICS_ENABLED", "false");

        let router = build_router(test_state(AuthMode::Disabled));
        let request = Request::builder()
            .uri("/metrics")
            .body(Body::empty())
            .expect("request");
        let response = router.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        std::env::remove_var("DMSX_API_METRICS_ENABLED");
    }

    #[tokio::test]
    async fn metrics_bearer_rejects_missing_or_mismatched_token() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("DMSX_API_METRICS_ENABLED", "true");
        std::env::set_var("DMSX_API_METRICS_BEARER", "secret-token");

        let router = build_router(test_state(AuthMode::Disabled));

        let req1 = Request::builder()
            .uri("/metrics")
            .body(Body::empty())
            .expect("req1");
        let resp1 = router.clone().oneshot(req1).await.expect("resp1");
        assert_eq!(resp1.status(), StatusCode::UNAUTHORIZED);

        let req2 = Request::builder()
            .uri("/metrics")
            .header(AUTHORIZATION, "Bearer wrong")
            .body(Body::empty())
            .expect("req2");
        let resp2 = router.clone().oneshot(req2).await.expect("resp2");
        assert_eq!(resp2.status(), StatusCode::UNAUTHORIZED);

        let req3 = Request::builder()
            .uri("/metrics")
            .header(AUTHORIZATION, "Bearer secret-token")
            .body(Body::empty())
            .expect("req3");
        let resp3 = router.oneshot(req3).await.expect("resp3");
        assert_eq!(resp3.status(), StatusCode::OK);

        std::env::remove_var("DMSX_API_METRICS_ENABLED");
        std::env::remove_var("DMSX_API_METRICS_BEARER");
    }

    #[tokio::test]
    async fn livekit_config_route_returns_payload_in_disabled_mode() {
        let router = build_router(test_state(AuthMode::Disabled));
        let request = Request::builder()
            .uri("/v1/config/livekit")
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        let status = response.status();
        let body = response_body(response).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["enabled"], true);
        assert_eq!(body["url"], "ws://127.0.0.1:7880");
    }

    #[tokio::test]
    async fn protected_route_rejects_missing_authorization_on_real_router() {
        let tenant_id = Uuid::new_v4();
        let router = build_router(test_state(AuthMode::Jwt));
        let request = Request::builder()
            .uri(format!("/v1/tenants/{tenant_id}/stats"))
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        let status = response.status();
        let body = response_body(response).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["title"], "Unauthorized");
        assert_eq!(body["detail"], "missing Authorization header");
    }

    #[tokio::test]
    async fn protected_route_rejects_tenant_mismatch_before_hitting_db() {
        let secret = "test-secret-please-change-me";
        let token_tenant_id = Uuid::new_v4();
        let path_tenant_id = Uuid::new_v4();
        let router = build_router(test_state(AuthMode::Jwt));
        let request = Request::builder()
            .uri(format!("/v1/tenants/{path_tenant_id}/stats"))
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
        let status = response.status();
        let body = response_body(response).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["title"], "Forbidden");
        assert_eq!(
            body["detail"],
            "tenant in URL is not permitted for this token"
        );
    }

    #[tokio::test]
    async fn read_only_role_cannot_write_on_real_router() {
        let secret = "test-secret-please-change-me";
        let tenant_id = Uuid::new_v4();
        let router = build_router(test_state(AuthMode::Jwt));
        let request = Request::builder()
            .method("POST")
            .uri(format!("/v1/tenants/{tenant_id}/artifacts"))
            .header(
                AUTHORIZATION,
                format!(
                    "Bearer {}",
                    issue_token(secret, tenant_id, vec!["ReadOnly".to_string()])
                ),
            )
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"pkg","version":"1.0.0","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","object_key":"obj"}"#))
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        let status = response.status();
        let body = response_body(response).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["title"], "Forbidden");
    }

    #[tokio::test]
    async fn tenant_admin_cannot_access_global_config_on_real_router() {
        let secret = "test-secret-please-change-me";
        let tenant_id = Uuid::new_v4();
        let router = build_router(test_state(AuthMode::Jwt));
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
        let status = response.status();
        let body = response_body(response).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["title"], "Forbidden");
    }

    #[tokio::test]
    async fn tenant_admin_cannot_post_create_tenant_on_real_router() {
        let secret = "test-secret-please-change-me";
        let tenant_id = Uuid::new_v4();
        let router = build_router(test_state(AuthMode::Jwt));
        let request = Request::builder()
            .method("POST")
            .uri("/v1/tenants")
            .header(
                AUTHORIZATION,
                format!(
                    "Bearer {}",
                    issue_token(secret, tenant_id, vec!["TenantAdmin".to_string()])
                ),
            )
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"new-tenant-from-tenant-admin"}"#))
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        let status = response.status();
        let body = response_body(response).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["title"], "Forbidden");
    }

    #[tokio::test]
    async fn operator_cannot_write_policy_on_real_router() {
        let secret = "test-secret-please-change-me";
        let tenant_id = Uuid::new_v4();
        let router = build_router(test_state(AuthMode::Jwt));
        let request = Request::builder()
            .method("POST")
            .uri(format!("/v1/tenants/{tenant_id}/policies"))
            .header(
                AUTHORIZATION,
                format!(
                    "Bearer {}",
                    issue_token(secret, tenant_id, vec!["Operator".to_string()])
                ),
            )
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"name":"policy-a","scope_kind":"tenant"}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        let status = response.status();
        let body = response_body(response).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["title"], "Forbidden");
    }

    #[tokio::test]
    async fn protected_route_rejects_issuer_mismatch_on_real_router() {
        let secret = "test-secret-please-change-me";
        let tenant_id = Uuid::new_v4();
        let router = build_router(test_state_with_auth(AuthConfig {
            mode: AuthMode::Jwt,
            jwt_secret: Some(secret.to_string()),
            jwt_issuer: Some("https://issuer.example".to_string()),
            jwt_audience: None,
            oidc_discovery_url: None,
            jwks_url: None,
            jwks_refresh_interval: std::time::Duration::from_secs(300),
            jwks_max_stale_age: std::time::Duration::from_secs(3600),
            jwks_allow_startup_without_keys: false,
            jwks_cache: None,
        }));
        let request = Request::builder()
            .uri(format!("/v1/tenants/{tenant_id}/stats"))
            .header(
                AUTHORIZATION,
                format!(
                    "Bearer {}",
                    issue_token_with_claims(
                        secret,
                        tenant_id,
                        vec!["TenantAdmin".to_string()],
                        Some("https://wrong-issuer.example"),
                        None,
                    )
                ),
            )
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("response");
        let status = response.status();
        let body = response_body(response).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["title"], "Unauthorized");
        assert_eq!(body["detail"], "JWT issuer mismatch");
    }
}
