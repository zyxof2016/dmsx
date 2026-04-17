//! JetStream 消费：设备网关上报的命令执行结果（subject `dmsx.command.result.{tenant}.{device}`），
//! 与 HTTP `POST .../commands/{id}/result` 对齐写入 Postgres。

use std::time::Duration;

use async_nats::jetstream::{
    self,
    consumer::{pull, AckPolicy, DeliverPolicy},
    stream,
    AckKind,
};
use futures_util::StreamExt;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::dto::SubmitCommandResultReq;
use crate::services::commands;
use crate::state::AppState;
use crate::repo::commands as command_repo;
use crate::db_rls;
use dmsx_core::{CommandStatus, DmsxError};

#[derive(Debug, Deserialize)]
struct CommandResultNatsEvent {
    tenant_id: Uuid,
    device_id: Uuid,
    command_id: Uuid,
    status: i32,
    exit_code: Option<i32>,
    #[serde(default)]
    stdout: String,
    #[serde(default)]
    stderr: String,
    evidence_key: Option<String>,
}

fn command_status_from_proto(status: i32) -> Result<CommandStatus, DmsxError> {
    match status {
        1 => Ok(CommandStatus::Queued),
        2 => Ok(CommandStatus::Delivered),
        3 => Ok(CommandStatus::Acked),
        4 => Ok(CommandStatus::Running),
        5 => Ok(CommandStatus::Succeeded),
        6 => Ok(CommandStatus::Failed),
        7 => Ok(CommandStatus::Expired),
        8 => Ok(CommandStatus::Cancelled),
        _ => Err(DmsxError::Validation(format!(
            "unsupported command result status {}",
            status
        ))),
    }
}

fn jetstream_enabled_from_env() -> bool {
    !matches!(
        std::env::var("DMSX_NATS_JETSTREAM_ENABLED")
            .ok()
            .map(|s| s.trim().to_ascii_lowercase())
            .unwrap_or_else(|| "true".to_string())
            .as_str(),
        "0" | "false" | "no" | "off"
    )
}

fn should_term_ack(err: &DmsxError) -> bool {
    matches!(
        err,
        DmsxError::NotFound(_)
            | DmsxError::Validation(_)
            | DmsxError::Forbidden(_)
            | DmsxError::Unauthorized(_)
            | DmsxError::Conflict(_)
    )
}

/// 与 `CommandJetStream::try_from_env` 条件一致；在 `build_state_from_env` 末尾调用。
pub fn spawn_background(st: AppState) {
    if !jetstream_enabled_from_env() {
        tracing::info!("NATS JetStream result ingest disabled (DMSX_NATS_JETSTREAM_ENABLED)");
        return;
    }
    let url = match std::env::var("DMSX_NATS_URL")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        Some(u) => u,
        None => {
            tracing::info!("NATS JetStream result ingest skipped (DMSX_NATS_URL unset)");
            return;
        }
    };

    tokio::spawn(async move {
        if let Err(e) = run_loop(st, url).await {
            tracing::error!(error = %e, "NATS JetStream result ingest exited");
        }
    });
}

async fn run_loop(st: AppState, url: String) -> Result<(), String> {
    let stream_name = std::env::var("DMSX_NATS_COMMAND_STREAM")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "DMSX_COMMANDS".to_string());

    let consumer_name = std::env::var("DMSX_NATS_RESULT_CONSUMER")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "dmsx-api-result-ingest".to_string());

    let client = async_nats::connect(url)
        .await
        .map_err(|e| format!("NATS connect: {e}"))?;
    let js = jetstream::new(client);

    js.get_or_create_stream(stream::Config {
        name: stream_name.clone(),
        subjects: vec!["dmsx.command.>".to_string()],
        ..Default::default()
    })
    .await
    .map_err(|e| format!("get_or_create_stream: {e}"))?;

    let stream = js
        .get_stream(&stream_name)
        .await
        .map_err(|e| format!("get_stream: {e}"))?;

    let consumer = stream
        .get_or_create_consumer(
            &consumer_name,
            pull::Config {
                durable_name: Some(consumer_name.clone()),
                filter_subject: "dmsx.command.result.>".to_string(),
                deliver_policy: DeliverPolicy::New,
                ack_policy: AckPolicy::Explicit,
                ..Default::default()
            },
        )
        .await
        .map_err(|e| format!("get_or_create_consumer: {e}"))?;

    let mut messages = consumer
        .messages()
        .await
        .map_err(|e| format!("consumer.messages: {e}"))?;

    tracing::info!(
        stream = %stream_name,
        consumer = %consumer_name,
        "NATS JetStream command result ingest started (filter: dmsx.command.result.>)"
    );

    while let Some(item) = messages.next().await {
        let msg = match item {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(error = %e, "result ingest: recv error");
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        let ev: CommandResultNatsEvent = match serde_json::from_slice(msg.payload.as_ref()) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "result ingest: invalid JSON, term");
                let _ = msg.ack_with(AckKind::Term).await;
                continue;
            }
        };

        let ctx = AuthContext::nats_jetstream_command_result(ev.tenant_id);
        let body = SubmitCommandResultReq {
            exit_code: ev.exit_code,
            stdout: ev.stdout,
            stderr: ev.stderr,
            evidence_key: ev.evidence_key,
        };

        let verify = async {
            let mut tx = db_rls::begin_rls_tx(&st.db, Some(ev.tenant_id), &ctx)
                .await
                .map_err(|e| DmsxError::Internal(format!("db: {e}")))?;
            let cmd = command_repo::get_command(&mut *tx, ev.tenant_id, ev.command_id)
                .await
                .map_err(|e| DmsxError::Internal(format!("db: {e}")))?
                .ok_or_else(|| {
                    DmsxError::NotFound(format!("command {}", ev.command_id))
                })?;
            if cmd.target_device_id.0 != ev.device_id {
                return Err(DmsxError::Validation(format!(
                    "command {} target_device_id mismatch (message device {})",
                    ev.command_id, ev.device_id
                )));
            }
            tx.commit().await.map_err(|e| DmsxError::Internal(format!("db: {e}")))?;
            Ok::<(), DmsxError>(())
        };

        if let Err(e) = verify.await {
            tracing::warn!(error = %e, command_id = %ev.command_id, "result ingest: verify failed");
            let _ = msg
                .ack_with(if should_term_ack(&e) {
                    AckKind::Term
                } else {
                    AckKind::Nak(None)
                })
                .await;
            continue;
        }

        let explicit_status = match command_status_from_proto(ev.status) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, command_id = %ev.command_id, status = ev.status, "result ingest: invalid status");
                let _ = msg.ack_with(AckKind::Term).await;
                continue;
            }
        };

        match commands::submit_command_result_with_status(
            &st,
            &ctx,
            ev.tenant_id,
            ev.command_id,
            &body,
            Some(explicit_status),
        )
        .await {
            Ok(_) => {
                if let Err(e) = msg.ack().await {
                    tracing::warn!(error = %e, "result ingest: ack failed");
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, command_id = %ev.command_id, "result ingest: submit failed");
                let _ = msg
                    .ack_with(if should_term_ack(&e) {
                        AckKind::Term
                    } else {
                        AckKind::Nak(None)
                    })
                    .await;
            }
        }
    }

    Ok(())
}
