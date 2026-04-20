//! `StreamCommands`：从 JetStream 拉取 `dmsx-api` 发布的命令（subject `dmsx.command.{tenant}.{device}`），
//! 使用按租户/设备稳定命名的 durable pull consumer，反序列化为 `dmsx_core::Command`
//! 并映射为 gRPC `CommandEnvelope`。

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use async_nats::jetstream::{
    self,
    consumer::{pull, AckPolicy, DeliverPolicy},
    stream, AckKind, Message,
};
use dmsx_core::Command;
use futures_util::StreamExt;
use tokio::sync::{mpsc, oneshot, watch};
use tokio_stream::{wrappers::ReceiverStream, Stream};
use tonic::Status;
use uuid::Uuid;

use crate::agent::CommandEnvelope;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct DeviceKey {
    tenant_id: Uuid,
    device_id: Uuid,
}

struct PendingCommand {
    command_id: Uuid,
    complete_tx: Option<oneshot::Sender<()>>,
}

struct SessionState {
    session_id: u64,
    key: DeviceKey,
    cancel_tx: watch::Sender<bool>,
    pending: Mutex<Option<PendingCommand>>,
}

pub(crate) struct CommandTracker {
    next_session_id: AtomicU64,
    sessions: Mutex<HashMap<DeviceKey, Arc<SessionState>>>,
}

#[derive(Clone)]
pub(crate) struct SessionHandle {
    session: Arc<SessionState>,
}

pub(crate) struct SessionLease {
    tracker: Arc<CommandTracker>,
    handle: SessionHandle,
}

struct SessionBoundStream {
    inner: ReceiverStream<Result<CommandEnvelope, Status>>,
    _lease: SessionLease,
}

impl CommandTracker {
    pub(crate) fn new() -> Self {
        Self {
            next_session_id: AtomicU64::new(1),
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn start_session(
        self: &Arc<Self>,
        tenant_id: Uuid,
        device_id: Uuid,
    ) -> Result<SessionLease, Status> {
        let key = DeviceKey {
            tenant_id,
            device_id,
        };
        let mut sessions = self.sessions.lock().unwrap();
        if sessions.contains_key(&key) {
            return Err(Status::already_exists(
                "stream_commands already active for this device",
            ));
        }

        let (cancel_tx, _) = watch::channel(false);
        let session = Arc::new(SessionState {
            session_id: self.next_session_id.fetch_add(1, Ordering::Relaxed),
            key,
            cancel_tx,
            pending: Mutex::new(None),
        });
        sessions.insert(key, session.clone());
        drop(sessions);

        Ok(SessionLease {
            tracker: self.clone(),
            handle: SessionHandle { session },
        })
    }

    pub(crate) fn mark_completed(
        &self,
        tenant_id: Uuid,
        device_id: Uuid,
        command_id: Uuid,
    ) -> bool {
        let key = DeviceKey {
            tenant_id,
            device_id,
        };
        let session = {
            let sessions = self.sessions.lock().unwrap();
            sessions.get(&key).cloned()
        };
        let Some(session) = session else {
            return false;
        };

        let mut pending = session.pending.lock().unwrap();
        let Some(current) = pending.as_mut() else {
            return false;
        };
        if current.command_id != command_id {
            return false;
        }
        if let Some(tx) = current.complete_tx.take() {
            let _ = tx.send(());
        }
        true
    }

    fn finish_session(&self, session: &Arc<SessionState>) {
        let mut sessions = self.sessions.lock().unwrap();
        if sessions
            .get(&session.key)
            .map(|current| current.session_id == session.session_id)
            .unwrap_or(false)
        {
            sessions.remove(&session.key);
        }
        drop(sessions);
        let _ = session.cancel_tx.send(true);
    }
}

impl SessionHandle {
    fn cancel_rx(&self) -> watch::Receiver<bool> {
        self.session.cancel_tx.subscribe()
    }

    fn register_pending(&self, command_id: Uuid) -> Result<oneshot::Receiver<()>, Status> {
        let mut pending = self.session.pending.lock().unwrap();
        if pending.is_some() {
            return Err(Status::aborted(
                "previous command is still pending result commit",
            ));
        }
        let (tx, rx) = oneshot::channel();
        *pending = Some(PendingCommand {
            command_id,
            complete_tx: Some(tx),
        });
        Ok(rx)
    }

    fn clear_pending(&self, command_id: Uuid) {
        let mut pending = self.session.pending.lock().unwrap();
        if pending
            .as_ref()
            .map(|current| current.command_id == command_id)
            .unwrap_or(false)
        {
            pending.take();
        }
    }
}

impl SessionLease {
    pub(crate) fn handle(&self) -> SessionHandle {
        self.handle.clone()
    }
}

impl Drop for SessionLease {
    fn drop(&mut self) {
        self.tracker.finish_session(&self.handle.session);
    }
}

impl Stream for SessionBoundStream {
    type Item = Result<CommandEnvelope, Status>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        Pin::new(&mut this.inner).poll_next(cx)
    }
}

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

async fn wait_for_result_commit(
    msg: &Message,
    session: &SessionHandle,
    command_id: Uuid,
    completion_rx: oneshot::Receiver<()>,
) -> bool {
    let mut cancel_rx = session.cancel_rx();
    let completion_rx = completion_rx;
    tokio::pin!(completion_rx);

    loop {
        if *cancel_rx.borrow() {
            return false;
        }

        tokio::select! {
            res = &mut completion_rx => return res.is_ok(),
            changed = cancel_rx.changed() => {
                if changed.is_err() || *cancel_rx.borrow() {
                    return false;
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(10)) => {
                if let Err(e) = msg.ack_with(AckKind::Progress).await {
                    tracing::warn!(
                        error = %e,
                        command_id = %command_id,
                        "dmsx-device-gw: progress ack failed while waiting for ReportResult"
                    );
                }
            }
        }
    }
}

async fn pull_loop(
    device_id: String,
    tenant_id: Option<Uuid>,
    cursor: Option<String>,
    tx: mpsc::Sender<Result<CommandEnvelope, Status>>,
    session: SessionHandle,
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
                    description: Some(
                        "device-gw StreamCommands durable consumer (commit on ReportResult)"
                            .to_string(),
                    ),
                    filter_subject: filter_subject.clone(),
                    deliver_policy,
                    ack_policy: AckPolicy::Explicit,
                    ack_wait: Duration::from_secs(30),
                    max_deliver: 5,
                    max_ack_pending: 1,
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
            .max_messages_per_batch(1)
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
                    let _ = msg.ack_with(AckKind::Term).await;
                    continue;
                }
            };

            if cmd.target_device_id.0.to_string() != device_id {
                tracing::warn!(
                    expected = %device_id,
                    got = %cmd.target_device_id.0,
                    "dmsx-device-gw: device_id mismatch on filtered subject; nak"
                );
                let _ = msg.ack_with(AckKind::Nak(None)).await;
                continue;
            }

            if let Some(expect_tid) = tenant_id {
                if cmd.tenant_id.0 != expect_tid {
                    tracing::warn!(
                        tenant = %expect_tid,
                        got = %cmd.tenant_id.0,
                        "dmsx-device-gw: tenant_id mismatch; nak"
                    );
                    let _ = msg.ack_with(AckKind::Nak(None)).await;
                    continue;
                }
            }

            let command_id = cmd.id.0;
            let completion_rx = match session.register_pending(command_id) {
                Ok(rx) => rx,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        command_id = %command_id,
                        "dmsx-device-gw: failed to register pending command; nak"
                    );
                    let _ = msg.ack_with(AckKind::Nak(None)).await;
                    continue;
                }
            };

            let env = map_command(&cmd);
            if tx.send(Ok(env)).await.is_err() {
                session.clear_pending(command_id);
                let _ = msg.ack_with(AckKind::Nak(None)).await;
                return;
            }

            if wait_for_result_commit(&msg, &session, command_id, completion_rx).await {
                session.clear_pending(command_id);
                if let Err(e) = msg.ack().await {
                    tracing::warn!(error = %e, command_id = %command_id, "dmsx-device-gw: message ack failed");
                }
                continue;
            }

            session.clear_pending(command_id);
            let _ = msg.ack_with(AckKind::Nak(None)).await;
            return;
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
    session_lease: SessionLease,
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
    let session = session_lease.handle();
    tokio::spawn(async move {
        pull_loop(did, tid, cursor, tx, session).await;
    });

    Box::pin(SessionBoundStream {
        inner: ReceiverStream::new(rx),
        _lease: session_lease,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracker_rejects_second_active_session_for_same_device() {
        let tracker = Arc::new(CommandTracker::new());
        let tenant_id = Uuid::nil();
        let device_id = Uuid::from_u128(1);

        let _first = tracker.start_session(tenant_id, device_id).unwrap();
        let second = tracker.start_session(tenant_id, device_id);

        assert!(second.is_err());
    }

    #[tokio::test]
    async fn tracker_completes_only_matching_pending_command() {
        let tracker = Arc::new(CommandTracker::new());
        let tenant_id = Uuid::nil();
        let device_id = Uuid::from_u128(2);
        let lease = tracker.start_session(tenant_id, device_id).unwrap();
        let session = lease.handle();

        let wrong_command = Uuid::from_u128(3);
        let command_id = Uuid::from_u128(4);
        let completion = session.register_pending(command_id).unwrap();

        assert!(!tracker.mark_completed(tenant_id, device_id, wrong_command));
        assert!(tracker.mark_completed(tenant_id, device_id, command_id));
        assert!(completion.await.is_ok());
    }
}
