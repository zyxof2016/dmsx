//! JetStream 发布：命令在 Postgres 落库成功后，将完整 `Command` JSON 投递到 NATS。
//!
//! Subject：`dmsx.command.{tenant_id}.{device_id}`（与 `docs/DEPLOYMENT.md` 说明一致）
//! Stream：默认 `DMSX_COMMANDS`，subjects 覆盖 `dmsx.command.>`

use std::sync::Arc;

use async_nats::jetstream;
use dmsx_core::Command;
use tracing::warn;

/// 控制面在 `DMSX_NATS_URL` 配置且未显式关闭 JetStream 时初始化。
#[derive(Clone)]
pub struct CommandJetStream {
    js: jetstream::Context,
}

impl CommandJetStream {
    /// 返回 `None`：未配置 URL、关闭 JetStream、或连接/建流失败（失败仅打日志，不阻塞 API 启动）。
    pub async fn try_from_env() -> Option<Arc<Self>> {
        let url = std::env::var("DMSX_NATS_URL")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())?;

        let jetstream_enabled = match std::env::var("DMSX_NATS_JETSTREAM_ENABLED")
            .ok()
            .map(|s| s.trim().to_ascii_lowercase())
            .unwrap_or_else(|| "true".to_string())
            .as_str()
        {
            "0" | "false" | "no" | "off" => false,
            _ => true,
        };
        if !jetstream_enabled {
            tracing::info!("NATS JetStream command publish disabled (DMSX_NATS_JETSTREAM_ENABLED)");
            return None;
        }

        let stream_name = std::env::var("DMSX_NATS_COMMAND_STREAM")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "DMSX_COMMANDS".to_string());

        let client = match async_nats::connect(url).await {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "NATS connect failed; command JetStream publish disabled");
                return None;
            }
        };

        let js = jetstream::new(client);

        let cfg = jetstream::stream::Config {
            name: stream_name,
            subjects: vec!["dmsx.command.>".to_string()],
            ..Default::default()
        };

        if let Err(e) = js.get_or_create_stream(cfg).await {
            warn!(error = %e, "NATS JetStream get_or_create_stream failed; command publish disabled");
            return None;
        }

        tracing::info!("NATS JetStream command stream ready (subjects: dmsx.command.>)");
        Some(Arc::new(Self { js }))
    }

    /// 异步发布：不阻塞 HTTP 路径；失败只记 warn。
    pub fn publish_command_created(&self, cmd: &Command) {
        let js = self.js.clone();
        let tid = cmd.tenant_id.0;
        let did = cmd.target_device_id.0;
        let subject = format!("dmsx.command.{tid}.{did}");
        let body = match serde_json::to_vec(cmd) {
            Ok(b) => b,
            Err(e) => {
                warn!(error = %e, "command json serialize for NATS failed");
                return;
            }
        };

        tokio::spawn(async move {
            match js.publish(subject.clone(), body.into()).await {
                Ok(ack) => {
                    if let Err(e) = ack.await {
                        warn!(
                            error = %e,
                            subject = %subject,
                            "NATS JetStream publish ack failed"
                        );
                    }
                }
                Err(e) => warn!(
                    error = %e,
                    subject = %subject,
                    "NATS JetStream publish failed"
                ),
            }
        });
    }
}
