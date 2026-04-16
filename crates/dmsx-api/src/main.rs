use dmsx_api::app::{build_router, build_state_from_env};

#[tokio::main]
async fn main() {
    let _telemetry = dmsx_api::telemetry::init_tracing("dmsx-api");

    let st = build_state_from_env().await;
    let app = build_router(st);

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
