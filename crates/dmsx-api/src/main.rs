mod db;
mod desktop;
mod dto;
mod handlers;
mod state;

use axum::{
    middleware,
    routing::{get, patch, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use state::AppState;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dmsx_api=info,tower_http=info,sqlx=warn".into()),
        )
        .init();

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
    let _ = db::ensure_tenant(&pool, dev_tenant, "默认租户").await;

    let livekit_url = std::env::var("LIVEKIT_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:7880".to_string());
    let livekit_api_key = std::env::var("LIVEKIT_API_KEY")
        .unwrap_or_else(|_| "dmsx-api-key".to_string());
    let livekit_api_secret = std::env::var("LIVEKIT_API_SECRET")
        .unwrap_or_else(|_| "dmsx-api-secret-that-is-at-least-32-chars".to_string());

    let st = AppState {
        db: pool,
        livekit_url,
        livekit_api_key,
        livekit_api_secret,
        desktop_sessions: Arc::new(RwLock::new(HashMap::new())),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(handlers::health))
        .route("/v1/tenants/{tenant_id}/stats", get(handlers::stats))
        // --- Devices ---
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
        // --- Policies ---
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
        // --- Device Shadow ---
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
        // --- Device Actions (remote control) ---
        .route(
            "/v1/tenants/{tenant_id}/devices/{device_id}/actions",
            post(handlers::device_action),
        )
        .route(
            "/v1/tenants/{tenant_id}/devices/{device_id}/commands",
            get(handlers::device_commands_list),
        )
        // --- Commands ---
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
        // --- Artifacts ---
        .route(
            "/v1/tenants/{tenant_id}/artifacts",
            get(handlers::artifacts_list).post(handlers::artifacts_create),
        )
        // --- Compliance ---
        .route(
            "/v1/tenants/{tenant_id}/compliance/findings",
            get(handlers::compliance_list),
        )
        // --- Remote Desktop ---
        .route(
            "/v1/tenants/{tenant_id}/devices/{device_id}/desktop/session",
            post(desktop::session_create).delete(desktop::session_delete),
        )
        .route(
            "/v1/tenants/{tenant_id}/devices/{device_id}/desktop/ws/viewer",
            get(desktop::ws_viewer),
        )
        .route(
            "/v1/tenants/{tenant_id}/devices/{device_id}/desktop/ws/agent",
            get(desktop::ws_agent),
        )
        .route("/v1/config/livekit", get(desktop::livekit_config))
        // --- AI ---
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
        .layer(middleware::from_fn(handlers::auth_middleware))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(st);

    let bind = std::env::var("DMSX_API_BIND").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .unwrap_or_else(|e| panic!("bind {bind}: {e}"));
    tracing::info!(
        "dmsx-api listening on http://{}",
        listener.local_addr().unwrap()
    );
    axum::serve(listener, app).await.expect("serve");
}
