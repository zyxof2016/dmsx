//! `StreamCommands`：从 JetStream 拉取 `dmsx-api` 发布的命令（subject `dmsx.command.{tenant}.{device}`），
//! 使用按租户/设备稳定命名的 durable pull consumer，反序列化为 `dmsx_core::Command`
//! 并映射为 gRPC `CommandEnvelope`。

use std::pin::Pin;
use std::time::Duration;

use async_nats::jetstream::{
    self,
    consumer::{pull, AckPolicy, DeliverPolicy},
    stream,
};
use dmsx_core::Command;
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::Status;
use uuid::Uuid;

use crate::agent::CommandEnvelope;

pub(crate) fn jetstream_enabled_from_env() -> bool {
    !matches!(
        std::env::var("DMSX_NATS_JETSTREAM_ENABLED")
            .ok()
            .map(|s| s.trim().to_ascii_lowercase())
            .unwrap_or_else(|| "true".to_string())
            .as_str(),
        "0" | "false" | "no" | "off"
    )
}

fn command_stream_filter_subject(device_id: &str, tenant_id: Option<Uuid>) -> String {
    match tenant_id {
        Some(tid) => format!("dmsx.command.{tid}.{device_id}"),
        None => format!("dmsx.command.*.{device_id}"),
    }
}

fn parse_cursor_start_sequence(cursor: Option<&str>) -> Option<u64> {
    cursor
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|v| *v > 0)
}

fn consumer_name(device_id: &str, tenant_id: Option<Uuid>) -> String {
    let prefix = std::env::var("DMSX_GW_COMMAND_CONSUMER_PREFIX")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "dgw".to_string());
    match tenant_id {
        Some(tid) => {
            let tid = tid.simple().to_string();
            let did = device_id.replace('-', "");
            format!(
                "{prefix}-{}-{}",
                &tid[..12.min(tid.len())],
                &did[..12.min(did.len())]
            )
        }
        None => {
            let did = device_id.replace('-', "");
            format!("{prefix}-{}", &did[..20.min(did.len())])
        }
    }
}

fn map_command(cmd: &Command) -> CommandEnvelope {
    CommandEnvelope {
        command_id: cmd.id.0.to_string(),
        idempotency_key: cmd.idempotency_key.clone().unwrap_or_default(),
        payload_json: cmd.payload.to_string(),
        ttl_seconds: cmd.ttl_seconds,
        priority: i32::from(cmd.priority),
    }
}

async fn pull_loop(
    device_id: String,
    tenant_id: Option<Uuid>,
    cursor: Option<String>,
    tx: mpsc::Sender<Result<CommandEnvelope, Status>>,
) {
    let nats_url = match std::env::var("DMSX_NATS_URL")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        Some(u) => u,
        None => return,
    };

    if !jetstream_enabled_from_env() {
        tracing::info!("JetStream command pull disabled (DMSX_NATS_JETSTREAM_ENABLED)");
        return;
    }

    let stream_name = std::env::var("DMSX_NATS_COMMAND_STREAM")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "DMSX_COMMANDS".to_string());

    let client = match async_nats::connect(nats_url).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "dmsx-device-gw: NATS connect failed");
            return;
        }
    };

    let js = jetstream::new(client);

    if let Err(e) = js
        .get_or_create_stream(stream::Config {
            name: stream_name.clone(),
            subjects: vec!["dmsx.command.>".to_string()],
            ..Default::default()
        })
        .await
    {
        tracing::warn!(error = %e, "dmsx-device-gw: get_or_create_stream failed");
        return;
    }

    let stream = match js.get_stream(&stream_name).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, stream = %stream_name, "dmsx-device-gw: get_stream failed");
            return;
        }
    };

    let filter_subject = command_stream_filter_subject(&device_id, tenant_id);
    let durable_name = consumer_name(&device_id, tenant_id);
    let deliver_policy = parse_cursor_start_sequence(cursor.as_deref())
        .map(|start_sequence| DeliverPolicy::ByStartSequence { start_sequence })
        .unwrap_or(DeliverPolicy::New);

    loop {
        let consumer = match stream
            .get_or_create_consumer(
                &durable_name,
                pull::Config {
                    durable_name: Some(durable_name.clone()),
                    name: Some(durable_name.clone()),
                    description: Some("device-gw StreamCommands durable consumer".to_string()),
                    filter_subject: filter_subject.clone(),
                    deliver_policy,
                    ack_policy: AckPolicy::Explicit,
                    ack_wait: Duration::from_secs(30),
                    max_deliver: 5,
                    max_ack_pending: 8,
                    max_waiting: 1,
                    inactive_threshold: Duration::from_secs(300),
                    ..Default::default()
                },
            )
            .await
        {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    consumer = %durable_name,
                    "dmsx-device-gw: get_or_create_consumer failed"
                );
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };

        let mut messages = match consumer
            .stream()
            .max_messages_per_batch(8)
            .heartbeat(Duration::from_secs(5))
            .expires(Duration::from_secs(15))
            .messages()
            .await
        {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(error = %e, consumer = %durable_name, "dmsx-device-gw: consumer.messages failed");
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };

        tracing::info!(
            device_id = %device_id,
            consumer = %durable_name,
            filter_subject = %filter_subject,
            cursor = ?cursor,
            "stream_commands: JetStream pull started"
        );

        while let Some(item) = messages.next().await {
            let msg = match item {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!(error = %e, consumer = %durable_name, "dmsx-device-gw: jetstream message recv error");
                    break;
                }
            };

            let cmd: Command = match serde_json::from_slice(msg.payload.as_ref()) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(error = %e, "dmsx-device-gw: command JSON decode failed, term drop");
                    let _ = msg
                        .ack_with(async_nats::jetstream::AckKind::Term)
                        .await;
                    continue;
                }
            };

            if cmd.target_device_id.0.to_string() != device_id {
                tracing::warn!(
                    expected = %device_id,
                    got = %cmd.target_device_id.0,
                    "dmsx-device-gw: device_id mismatch on filtered subject; nak"
                );
                let _ = msg
                    .ack_with(async_nats::jetstream::AckKind::Nak(None))
                    .await;
                continue;
            }

            if let Some(expect_tid) = tenant_id {
                if cmd.tenant_id.0 != expect_tid {
                    tracing::warn!(
                        tenant = %expect_tid,
                        got = %cmd.tenant_id.0,
                        "dmsx-device-gw: tenant_id mismatch; nak"
                    );
                    let _ = msg
                        .ack_with(async_nats::jetstream::AckKind::Nak(None))
                        .await;
                    continue;
                }
            }

            let env = map_command(&cmd);
            if tx.send(Ok(env)).await.is_err() {
                let _ = msg
                    .ack_with(async_nats::jetstream::AckKind::Nak(None))
                    .await;
                return;
            }

            if let Err(e) = msg.ack().await {
                tracing::warn!(error = %e, "dmsx-device-gw: message ack failed");
            }
        }

        tracing::warn!(
            consumer = %durable_name,
            "stream_commands: message stream ended, recreating durable consumer stream"
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// 无 `DMSX_NATS_URL`、关闭 JetStream、或 `device_id` 为空时返回空流（与旧 stub 行为一致）。
pub fn stream_commands(
    device_id: String,
    tenant_id: Option<Uuid>,
    cursor: Option<String>,
) -> Pin<Box<dyn tokio_stream::Stream<Item = Result<CommandEnvelope, Status>> + Send + 'static>> {
    let device_id = device_id.trim().to_string();
    if device_id.is_empty() {
        return Box::pin(tokio_stream::empty());
    }

    let nats_configured = std::env::var("DMSX_NATS_URL")
        .ok()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    if !nats_configured || !jetstream_enabled_from_env() {
        return Box::pin(tokio_stream::empty());
    }

    let (tx, rx) = mpsc::channel::<Result<CommandEnvelope, Status>>(1);
    let did = device_id.clone();
    let tid = tenant_id;
    let cursor = cursor;
    tokio::spawn(async move {
        pull_loop(did, tid, cursor, tx).await;
    });

    Box::pin(ReceiverStream::new(rx))
}
