//! DeviceGateway：gRPC 骨架。生产环境需启用 mTLS、连接限流与 OpenTelemetry。

use std::pin::Pin;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio_stream::{Stream, StreamExt};
use tonic::{transport::Server, Request, Response, Status, Streaming};

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

#[derive(Default)]
struct AgentServiceImpl;

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[tonic::async_trait]
impl AgentService for AgentServiceImpl {
    async fn enroll(
        &self,
        _request: Request<EnrollRequest>,
    ) -> Result<Response<EnrollResponse>, Status> {
        tracing::info!("enroll (stub)");
        Err(Status::unimplemented(
            "enroll: 接入 CA + enrollment 服务后实现",
        ))
    }

    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        let inner = request.into_inner();
        tracing::debug!(device_id = %inner.device_id, "heartbeat");
        Ok(Response::new(HeartbeatResponse {
            server_time_unix: now_unix(),
        }))
    }

    async fn fetch_desired_state(
        &self,
        request: Request<FetchDesiredStateRequest>,
    ) -> Result<Response<FetchDesiredStateResponse>, Status> {
        let inner = request.into_inner();
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
        let inner = request.into_inner();
        tracing::info!(device_id = %inner.device_id, "stream_commands (stub empty stream)");
        let s = tokio_stream::empty::<Result<CommandEnvelope, Status>>();
        Ok(Response::new(Box::pin(s) as Self::StreamCommandsStream))
    }

    async fn report_result(
        &self,
        request: Request<ReportResultRequest>,
    ) -> Result<Response<ReportResultResponse>, Status> {
        let inner = request.into_inner();
        tracing::info!(
            device_id = %inner.device_id,
            command_id = %inner.command_id,
            status = inner.status,
            "report_result"
        );
        Ok(Response::new(ReportResultResponse { accepted: true }))
    }

    async fn upload_evidence(
        &self,
        request: Request<Streaming<UploadEvidenceRequest>>,
    ) -> Result<Response<UploadEvidenceResponse>, Status> {
        let mut stream = request.into_inner();
        let mut total_bytes: usize = 0;
        while let Some(chunk) = stream.next().await {
            let req = chunk?;
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
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dmsx_device_gw=info".into()),
        )
        .init();

    let bind_addr = std::env::var("DMSX_GW_BIND").unwrap_or_else(|_| "0.0.0.0:50051".to_string());
    let addr = bind_addr.parse()?;
    tracing::info!("dmsx-device-gw listening on grpc://{}", addr);

    Server::builder()
        .add_service(HealthServer::new(HealthService))
        .add_service(AgentServiceServer::new(AgentServiceImpl))
        .serve(addr)
        .await?;

    Ok(())
}
