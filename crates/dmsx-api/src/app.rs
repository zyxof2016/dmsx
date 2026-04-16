use axum::{
    middleware,
    routing::{get, patch, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use crate::{
    auth::{load_auth_config_from_env, spawn_jwks_refresh_task},
    desktop, handlers,
    services::bootstrap,
    state::AppState,
};

pub async fn build_state_from_env() -> AppState {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://dmsx:dmsx@127.0.0.1:5432/dmsx".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await
        .expect("failed to connect to database");

    tracing::info!("connected to postgres");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    tracing::info!("migrations applied");

    let dev_tenant = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let livekit_url =
        std::env::var("LIVEKIT_URL").unwrap_or_else(|_| "ws://127.0.0.1:7880".to_string());
    let livekit_api_key =
        std::env::var("LIVEKIT_API_KEY").unwrap_or_else(|_| "dmsx-api-key".to_string());
    let livekit_api_secret = std::env::var("LIVEKIT_API_SECRET")
        .unwrap_or_else(|_| "dmsx-api-secret-that-is-at-least-32-chars".to_string());

    let state = AppState {
        db: pool,
        livekit_url,
        livekit_api_key,
        livekit_api_secret,
        desktop_sessions: Arc::new(RwLock::new(HashMap::new())),
        device_sessions: Arc::new(RwLock::new(HashMap::new())),
        auth: load_auth_config_from_env()
            .await
            .expect("failed to initialize auth config"),
    };

    spawn_jwks_refresh_task(state.auth.clone());
    bootstrap::ensure_default_tenant(&state, dev_tenant, "默认租户").await;

    state
}

pub fn build_router(st: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(handlers::health))
        .route("/ready", get(handlers::ready))
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
            "/v1/tenants/{tenant_id}/artifacts",
            get(handlers::artifacts_list).post(handlers::artifacts_create),
        )
        .route(
            "/v1/tenants/{tenant_id}/compliance/findings",
            get(handlers::compliance_list),
        )
        .route(
            "/v1/tenants/{tenant_id}/devices/{device_id}/desktop/session",
            post(desktop::session_create).delete(desktop::session_delete),
        )
        .route("/v1/config/livekit", get(desktop::livekit_config))
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
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(st)
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
    use serde_json::Value;
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::auth::{AuthConfig, AuthMode, JwtClaims, JwksCache};

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
            roles,
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
        assert_eq!(body["detail"], "tenant in URL does not match JWT tenant_id");
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
