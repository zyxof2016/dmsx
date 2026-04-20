//! DeviceGateway：未来数据面主链路的 gRPC 骨架。
//!
//! 当前仓库中，`dmsx-agent` 仍以 HTTP 控制面接口为主通信路径；
//! 本 crate 主要用于承载后续 mTLS、长连接 streaming、背压与离线重放能力。
//!
//! **命令流**：当配置 `DMSX_NATS_URL` 且启用 JetStream 时，`StreamCommands` 从 JetStream
//!（`DMSX_NATS_COMMAND_STREAM`，默认 `DMSX_COMMANDS`）按 `device_id`（及可选 `tenant_id`）拉取 `dmsx-api` 发布的命令。
//!
//! **回执**：同一 JetStream stream 下发布 `dmsx.command.result.{tenant_id}.{device_id}`，由 `dmsx-api` 消费并写入 Postgres。
//!
//! **mTLS**：配置 `DMSX_GW_TLS_CERT` / `DMSX_GW_TLS_KEY` / `DMSX_GW_TLS_CLIENT_CA` 后启用 TLS；
//! 客户端证书 SAN 须包含 URI `urn:dmsx:tenant:{uuid}:device:{uuid}`，并与 RPC 中的 `device_id` / `tenant_id` 一致。

use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio_stream::{Stream, StreamExt};
use tonic::transport::{Certificate, Identity, Server, ServerTlsConfig};
use tonic::{Request, Response, Status, Streaming};

pub mod agent {
    tonic::include_proto!("dmsx.agent.v1");
}

pub mod grpc_health {
    tonic::include_proto!("grpc.health.v1");
}

use agent::agent_service_server::{AgentService, AgentServiceServer};
use agent::{
    CommandEnvelope, EnrollRequest, EnrollResponse, FetchDesiredStateRequest,
    FetchDesiredStateResponse, HeartbeatRequest, HeartbeatResponse, ReportResultRequest,
    ReportResultResponse, StreamCommandsRequest, UploadEvidenceRequest, UploadEvidenceResponse,
};

use grpc_health::health_server::{Health, HealthServer};
use grpc_health::{HealthCheckRequest, HealthCheckResponse};

mod client_identity;
mod command_stream;
mod enroll;
mod enroll_token;
mod metrics_http;
mod rate_limit;
mod result_publish;
mod telemetry;

// ---------------------------------------------------------------------------
// gRPC Health Check (grpc.health.v1)
// ---------------------------------------------------------------------------

#[derive(Default)]
struct HealthService;

#[tonic::async_trait]
impl Health for HealthService {
    async fn check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        Ok(Response::new(HealthCheckResponse {
            status: grpc_health::health_check_response::ServingStatus::Serving as i32,
        }))
    }

    type WatchStream =
        Pin<Box<dyn Stream<Item = Result<HealthCheckResponse, Status>> + Send + 'static>>;

    async fn watch(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<Self::WatchStream>, Status> {
        Err(Status::unimplemented("watch not supported"))
    }
}

// ---------------------------------------------------------------------------
// Agent Service
// ---------------------------------------------------------------------------

struct AgentServiceImpl {
    /// 已配置客户端 CA 且未设置 `DMSX_GW_TLS_CLIENT_AUTH_OPTIONAL=1`。
    require_mtls_identity: bool,
    nats_js: Option<Arc<async_nats::jetstream::Context>>,
    active_stream_commands: Arc<std::sync::atomic::AtomicU64>,
    active_uploads: Arc<std::sync::atomic::AtomicU64>,
    tenant_rate_limiter: Option<Arc<rate_limit::TenantLimiter>>,
}

impl AgentServiceImpl {
    fn new(require_mtls_identity: bool, nats_js: Option<Arc<async_nats::jetstream::Context>>) -> Self {
        Self {
            require_mtls_identity,
            nats_js,
            active_stream_commands: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            active_uploads: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            tenant_rate_limiter: rate_limit::from_env(),
        }
    }
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn client_auth_optional_from_env() -> bool {
    matches!(
        std::env::var("DMSX_GW_TLS_CLIENT_AUTH_OPTIONAL")
            .ok()
            .map(|s| s.trim().to_ascii_lowercase())
            .unwrap_or_default()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn enroll_enabled_from_env() -> bool {
    let has_secret = std::env::var("DMSX_GW_ENROLL_HMAC_SECRET")
        .ok()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    let has_ca_cert = std::env::var("DMSX_GW_ENROLL_CA_CERT")
        .ok()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    let has_ca_key = std::env::var("DMSX_GW_ENROLL_CA_KEY")
        .ok()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    has_secret && has_ca_cert && has_ca_key
}

fn concurrency_per_connection_from_env() -> usize {
    std::env::var("DMSX_GW_CONCURRENCY_PER_CONNECTION")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(64)
        .max(1)
}

async fn load_server_tls() -> Result<Option<ServerTlsConfig>, std::io::Error> {
    let cert_path = std::env::var("DMSX_GW_TLS_CERT")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let key_path = std::env::var("DMSX_GW_TLS_KEY")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let (cert_path, key_path) = match (cert_path, key_path) {
        (None, None) => return Ok(None),
        (Some(c), Some(k)) => (c, k),
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "DMSX_GW_TLS_CERT and DMSX_GW_TLS_KEY must both be set to enable TLS",
            ));
        }
    };

    let cert_pem = tokio::fs::read_to_string(&cert_path).await?;
    let key_pem = tokio::fs::read_to_string(&key_path).await?;
    let identity = Identity::from_pem(cert_pem.as_bytes(), key_pem.as_bytes());

    let ca_path = std::env::var("DMSX_GW_TLS_CLIENT_CA")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let mut cfg = ServerTlsConfig::new().identity(identity);
    if let Some(ca) = ca_path {
        let ca_pem = tokio::fs::read_to_string(ca).await?;
        cfg = cfg.client_ca_root(Certificate::from_pem(ca_pem.as_bytes()));
        // Allow unauthenticated handshake when Enroll is enabled so new devices can enroll
        // before obtaining a client certificate; non-Enroll RPCs still enforce identity in code.
        cfg = cfg.client_auth_optional(client_auth_optional_from_env() || enroll_enabled_from_env());
    }

    Ok(Some(cfg))
}

#[tonic::async_trait]
impl AgentService for AgentServiceImpl {
    async fn enroll(
        &self,
        request: Request<EnrollRequest>,
    ) -> Result<Response<EnrollResponse>, Status> {
        let inner = request.into_inner();
        let claims = enroll_token::verify(&inner.enrollment_token, now_unix())?;
        rate_limit::check(&self.tenant_rate_limiter, claims.tenant_id)?;
        let (device_id, issued_cert_pem, ca_cert_pem, cert_expires_unix, tenant_id) =
            enroll::issue_device_cert(&claims, &inner.public_key_pem).await?;
        tracing::info!(
            tenant_id = %tenant_id,
            device_id = %device_id,
            "enroll: issued device certificate"
        );
        Ok(Response::new(EnrollResponse {
            device_id: device_id.to_string(),
            issued_cert_pem,
            ca_cert_pem,
            cert_expires_unix,
        }))
    }

    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        let inner = request.get_ref();
        let did = client_identity::resolve_device_only(
            &request,
            self.require_mtls_identity,
            &inner.device_id,
        )?;
        if self.require_mtls_identity {
            let certs = request
                .peer_certs()
                .ok_or_else(|| Status::unauthenticated("mTLS: peer certificates not available"))?;
            let id = client_identity::identity_from_peer_certs_der(certs.as_ref().as_slice())?;
            rate_limit::check(&self.tenant_rate_limiter, id.tenant_id)?;
            if id.device_id != did {
                return Err(Status::permission_denied("device_id does not match client certificate"));
            }
        }
        tracing::debug!(device_id = %inner.device_id, "heartbeat");
        Ok(Response::new(HeartbeatResponse {
            server_time_unix: now_unix(),
        }))
    }

    async fn fetch_desired_state(
        &self,
        request: Request<FetchDesiredStateRequest>,
    ) -> Result<Response<FetchDesiredStateResponse>, Status> {
        let inner = request.get_ref();
        let did = client_identity::resolve_device_only(
            &request,
            self.require_mtls_identity,
            &inner.device_id,
        )?;
        if self.require_mtls_identity {
            let certs = request
                .peer_certs()
                .ok_or_else(|| Status::unauthenticated("mTLS: peer certificates not available"))?;
            let id = client_identity::identity_from_peer_certs_der(certs.as_ref().as_slice())?;
            rate_limit::check(&self.tenant_rate_limiter, id.tenant_id)?;
            if id.device_id != did {
                return Err(Status::permission_denied("device_id does not match client certificate"));
            }
        }
        tracing::debug!(device_id = %inner.device_id, "fetch_desired_state");
        Ok(Response::new(FetchDesiredStateResponse {
            policy_revision_id: String::new(),
            spec_version: 0,
            spec_json: "{}".to_string(),
        }))
    }

    type StreamCommandsStream =
        Pin<Box<dyn Stream<Item = Result<CommandEnvelope, Status>> + Send + 'static>>;

    async fn stream_commands(
        &self,
        request: Request<StreamCommandsRequest>,
    ) -> Result<Response<Self::StreamCommandsStream>, Status> {
        self.active_stream_commands
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        struct Guard(Arc<std::sync::atomic::AtomicU64>);
        impl Drop for Guard {
            fn drop(&mut self) {
                self.0.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        let _guard = Guard(self.active_stream_commands.clone());

        let (tid, did) = client_identity::resolve_tenant_device(
            &request,
            self.require_mtls_identity,
            &request.get_ref().tenant_id,
            &request.get_ref().device_id,
        )?;
        rate_limit::check(&self.tenant_rate_limiter, tid)?;
        tracing::info!(tenant_id = %tid, device_id = %did, "stream_commands");
        let s: Self::StreamCommandsStream =
            command_stream::stream_commands(
                did.to_string(),
                Some(tid),
                Some(request.get_ref().cursor.clone()),
            );
        Ok(Response::new(s))
    }

    async fn report_result(
        &self,
        request: Request<ReportResultRequest>,
    ) -> Result<Response<ReportResultResponse>, Status> {
        let (tid, did) = client_identity::resolve_tenant_device(
            &request,
            self.require_mtls_identity,
            &request.get_ref().tenant_id,
            &request.get_ref().device_id,
        )?;
        rate_limit::check(&self.tenant_rate_limiter, tid)?;
        let inner = request.into_inner();
        let command_id = uuid::Uuid::parse_str(inner.command_id.trim()).map_err(|_| {
            Status::invalid_argument("command_id must be a UUID")
        })?;

        let js = match &self.nats_js {
            Some(j) => j.as_ref(),
            None => {
                tracing::warn!("report_result: NATS/JetStream not configured; accepted=false");
                return Ok(Response::new(ReportResultResponse { accepted: false }));
            }
        };

        let stream_name = std::env::var("DMSX_NATS_COMMAND_STREAM")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "DMSX_COMMANDS".to_string());

        let ev = result_publish::CommandResultEvent {
            tenant_id: tid,
            device_id: did,
            command_id,
            exit_code: Some(inner.exit_code),
            stdout: inner.stdout_snippet,
            stderr: inner.stderr_snippet,
            evidence_key: if inner.evidence_object_key.is_empty() {
                None
            } else {
                Some(inner.evidence_object_key)
            },
            status: inner.status,
        };

        result_publish::publish_command_result(js, &stream_name, &ev)
            .await
            .map_err(|e| Status::internal(e))?;

        tracing::info!(
            tenant_id = %tid,
            device_id = %did,
            command_id = %command_id,
            "report_result published to JetStream"
        );

        Ok(Response::new(ReportResultResponse { accepted: true }))
    }

    async fn upload_evidence(
        &self,
        request: Request<Streaming<UploadEvidenceRequest>>,
    ) -> Result<Response<UploadEvidenceResponse>, Status> {
        self.active_uploads
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        struct Guard(Arc<std::sync::atomic::AtomicU64>);
        impl Drop for Guard {
            fn drop(&mut self) {
                self.0.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        let _guard = Guard(self.active_uploads.clone());

        let require_mtls_identity = self.require_mtls_identity;
        let peer_certs = request.peer_certs();
        let mut stream = request.into_inner();
        let mut total_bytes: usize = 0;
        let mut validated_device = false;
        while let Some(chunk) = stream.next().await {
            let req = chunk?;
            if !validated_device {
                let did = uuid::Uuid::parse_str(req.device_id.trim())
                    .map_err(|_| Status::invalid_argument("device_id must be a UUID"))?;
                if require_mtls_identity {
                    let certs = peer_certs
                        .clone()
                        .ok_or_else(|| {
                            Status::unauthenticated("mTLS: peer certificates not available")
                        })?;
                    let id = client_identity::identity_from_peer_certs_der(certs.as_ref().as_slice())?;
                    rate_limit::check(&self.tenant_rate_limiter, id.tenant_id)?;
                    if id.device_id != did {
                        return Err(Status::permission_denied(
                            "device_id does not match client certificate",
                        ));
                    }
                }
                validated_device = true;
            }
            total_bytes += req.chunk.len();
            const MAX_EVIDENCE_BYTES: usize = 256 * 1024 * 1024; // 256 MiB
            if total_bytes > MAX_EVIDENCE_BYTES {
                return Err(Status::resource_exhausted("evidence too large (>256 MiB)"));
            }
        }
        Ok(Response::new(UploadEvidenceResponse {
            object_key: "stub/object".to_string(),
        }))
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _telemetry = telemetry::init_tracing("dmsx-device-gw");

    let bind_addr = std::env::var("DMSX_GW_BIND").unwrap_or_else(|_| "0.0.0.0:50051".to_string());
    let addr = bind_addr.parse()?;

    let tls_cfg = load_server_tls().await?;
    let client_ca_configured = std::env::var("DMSX_GW_TLS_CLIENT_CA")
        .ok()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    let require_mtls_identity = tls_cfg.is_some() && client_ca_configured;

    let nats_js = result_publish::connect_jetstream_from_env().await;
    let svc_impl = AgentServiceImpl::new(require_mtls_identity, nats_js);

    if metrics_http::enabled_from_env() {
        let st = metrics_http::MetricsState {
            active_stream_commands: svc_impl.active_stream_commands.clone(),
            active_uploads: svc_impl.active_uploads.clone(),
        };
        tokio::spawn(async move {
            if let Err(e) = metrics_http::serve_http(st).await {
                tracing::error!(error = %e, "metrics server exited");
            }
        });
    }

    {
        let streams = svc_impl.active_stream_commands.clone();
        let uploads = svc_impl.active_uploads.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                let s = streams.load(std::sync::atomic::Ordering::Relaxed);
                let u = uploads.load(std::sync::atomic::Ordering::Relaxed);
                tracing::info!(active_stream_commands = s, active_uploads = u, "gw.activity");
            }
        });
    }

    tracing::info!(
        "dmsx-device-gw listening on grpc://{} (tls={}, require_mtls_identity={})",
        addr,
        tls_cfg.is_some(),
        require_mtls_identity
    );

    let mut server = Server::builder();
    if let Some(tls) = tls_cfg {
        server = server.tls_config(tls)?;
    }
    server = server.concurrency_limit_per_connection(concurrency_per_connection_from_env());

    server
        .add_service(HealthServer::new(HealthService))
        .add_service(AgentServiceServer::new(svc_impl))
        .serve(addr)
        .await?;

    Ok(())
}
