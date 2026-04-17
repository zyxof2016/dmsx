//! 将命令执行结果发布到 JetStream，subject：`dmsx.command.result.{tenant_id}.{device_id}`（与 `dmsx-api` 入库消费者约定一致）。

use std::sync::Arc;

use async_nats::jetstream::{self, stream};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct CommandResultEvent {
    pub tenant_id: Uuid,
    pub device_id: Uuid,
    pub command_id: Uuid,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub evidence_key: Option<String>,
    pub status: i32,
}

pub async fn publish_command_result(
    js: &jetstream::Context,
    stream_name: &str,
    event: &CommandResultEvent,
) -> Result<(), String> {
    js.get_or_create_stream(stream::Config {
        name: stream_name.to_string(),
        subjects: vec!["dmsx.command.>".to_string()],
        ..Default::default()
    })
    .await
    .map_err(|e| format!("get_or_create_stream: {e}"))?;

    let subject = format!(
        "dmsx.command.result.{}.{}",
        event.tenant_id, event.device_id
    );
    let body = serde_json::to_vec(event).map_err(|e| e.to_string())?;
    let ack = js
        .publish(subject.clone(), body.into())
        .await
        .map_err(|e| format!("publish: {e}"))?;
    ack.await.map_err(|e| format!("publish ack: {e}"))?;
    Ok(())
}

pub async fn connect_jetstream_from_env() -> Option<Arc<jetstream::Context>> {
    if !super::command_stream::jetstream_enabled_from_env() {
        tracing::info!("JetStream result publish disabled (DMSX_NATS_JETSTREAM_ENABLED)");
        return None;
    }
    let nats_url = std::env::var("DMSX_NATS_URL")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())?;
    let client = async_nats::connect(nats_url)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "dmsx-device-gw: NATS connect failed (result publish)");
            e
        })
        .ok()?;
    Some(Arc::new(jetstream::new(client)))
}
