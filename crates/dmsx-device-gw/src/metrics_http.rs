//! Minimal Prometheus metrics endpoint for device-gw.
//!
//! Exposes gauges derived from in-process counters.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::registry::Registry;

#[derive(Clone)]
pub struct MetricsState {
    pub active_stream_commands: Arc<AtomicU64>,
    pub active_uploads: Arc<AtomicU64>,
}

fn truthy_env(name: &str) -> bool {
    matches!(
        std::env::var(name)
            .ok()
            .map(|v| v.trim().to_ascii_lowercase())
            .as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}

pub fn enabled_from_env() -> bool {
    truthy_env("DMSX_GW_METRICS_ENABLED")
}

pub fn bind_from_env() -> String {
    std::env::var("DMSX_GW_METRICS_BIND")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "0.0.0.0:9090".to_string())
}

fn render_metrics(st: &MetricsState) -> Vec<u8> {
    let mut reg = Registry::default();

    let g_streams: Gauge<i64> = Gauge::default();
    g_streams.set(st.active_stream_commands.load(Ordering::Relaxed) as i64);
    reg.register(
        "dmsx_gw_active_stream_commands",
        "Active StreamCommands server streams",
        g_streams,
    );

    let g_uploads: Gauge<i64> = Gauge::default();
    g_uploads.set(st.active_uploads.load(Ordering::Relaxed) as i64);
    reg.register(
        "dmsx_gw_active_uploads",
        "Active UploadEvidence client streams",
        g_uploads,
    );

    let mut buf = String::new();
    let _ = encode(&mut buf, &reg);
    buf.into_bytes()
}

pub async fn serve_http(metrics: MetricsState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bind = bind_from_env();
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!("dmsx-device-gw metrics listening on http://{}/metrics", bind);

    loop {
        let (mut socket, _peer) = listener.accept().await?;
        let body = render_metrics(&metrics);
        let resp = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/plain; version=0.0.4\r\ncontent-length: {}\r\n\r\n",
            body.len()
        );
        // Very small server: always returns metrics for any path.
        use tokio::io::AsyncWriteExt;
        socket.write_all(resp.as_bytes()).await?;
        socket.write_all(&body).await?;
        let _ = socket.shutdown().await;
    }
}

