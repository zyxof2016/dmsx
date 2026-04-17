//! `StreamCommands`：从 JetStream 拉取 `dmsx-api` 发布的命令（subject `dmsx.command.{tenant}.{device}`），
//! 过滤 `dmsx.command.*.{device_id}`，反序列化为 `dmsx_core::Command` 并映射为 gRPC `CommandEnvelope`。

use std::pin::Pin;

use async_nats::jetstream::{
    self,
    consumer::{pull, DeliverPolicy},
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

    let consumer_label = format!("dmsx-gw-{}", Uuid::new_v4().as_simple());
    let filter_subject = command_stream_filter_subject(&device_id, tenant_id);

    let consumer = match stream
        .get_or_create_consumer(
            &consumer_label,
            pull::Config {
                durable_name: None,
                name: Some(consumer_label.clone()),
                filter_subject,
                deliver_policy: DeliverPolicy::New,
                ..Default::default()
            },
        )
        .await
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                error = %e,
                consumer = %consumer_label,
                "dmsx-device-gw: get_or_create_consumer failed"
            );
            return;
        }
    };

    let mut messages = match consumer.messages().await {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(error = %e, "dmsx-device-gw: consumer.messages failed");
            return;
        }
    };

    tracing::info!(
        device_id = %device_id,
        consumer = %consumer_label,
        "stream_commands: JetStream pull started"
    );

    while let Some(item) = messages.next().await {
        let msg = match item {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(error = %e, "dmsx-device-gw: jetstream message recv error");
                continue;
            }
        };

        let cmd: Command = match serde_json::from_slice(msg.payload.as_ref()) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "dmsx-device-gw: command JSON decode failed, ack drop");
                let _ = msg.ack().await;
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
            break;
        }

        if let Err(e) = msg.ack().await {
            tracing::warn!(error = %e, "dmsx-device-gw: message ack failed");
        }
    }
}

/// 无 `DMSX_NATS_URL`、关闭 JetStream、或 `device_id` 为空时返回空流（与旧 stub 行为一致）。
pub fn stream_commands(
    device_id: String,
    tenant_id: Option<Uuid>,
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

    let (tx, rx) = mpsc::channel::<Result<CommandEnvelope, Status>>(32);
    let did = device_id.clone();
    let tid = tenant_id;
    tokio::spawn(async move {
        pull_loop(did, tid, tx).await;
    });

    Box::pin(ReceiverStream::new(rx))
}
