use axum::http;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::extract::Request;
use dmsx_core::DmsxError;
use std::sync::Arc;
use tower_governor::key_extractor::KeyExtractor;
use tower_governor::{
    governor::{GovernorConfig, GovernorConfigBuilder},
    GovernorLayer, GovernorError,
};

pub fn request_body_limit_bytes_from_env() -> usize {
    std::env::var("DMSX_API_REQUEST_BODY_LIMIT_BYTES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1_048_576)
}

pub async fn request_body_limit_middleware(
    request: Request,
    next: Next,
) -> Response {
    let limit = request_body_limit_bytes_from_env();
    if matches!(request.uri().path(), "/health" | "/ready") {
        return next.run(request).await;
    }

    if let Some(content_length) = request
        .headers()
        .get(http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok())
    {
        if content_length > limit {
            return DmsxError::PayloadTooLarge(format!(
                "request body too large: content-length {content_length} > limit {limit}"
            ))
            .into_response();
        }
    }

    next.run(request).await
}

pub fn tenant_rate_limit_layer_from_env(
) -> Option<GovernorLayer<TenantKeyExtractor, governor::middleware::NoOpMiddleware>> {
    let enabled = std::env::var("DMSX_API_RATE_LIMIT_ENABLED")
        .ok()
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false);
    if !enabled {
        return None;
    }

    let per_second = std::env::var("DMSX_API_RATE_LIMIT_PER_SECOND")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(50);
    let burst = std::env::var("DMSX_API_RATE_LIMIT_BURST")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(100);

    let mut builder = GovernorConfigBuilder::default().key_extractor(TenantKeyExtractor);
    builder.per_second(per_second.max(1) as u64);
    builder.burst_size(burst.max(1));
    builder.error_handler(|err| {
        DmsxError::TooManyRequests(format!("rate limit exceeded: {err}")).into_response()
    });
    let config: GovernorConfig<TenantKeyExtractor, governor::middleware::NoOpMiddleware> =
        builder.finish().expect("rate limit config");

    Some(GovernorLayer {
        config: Arc::new(config),
    })
}

#[derive(Clone)]
pub(crate) struct TenantKeyExtractor;

impl KeyExtractor for TenantKeyExtractor {
    type Key = String;

    fn extract<T>(&self, req: &http::request::Request<T>) -> Result<Self::Key, GovernorError> {
        if let Some(tid) = tenant_id_from_path(req.uri().path()) {
            return Ok(format!("tenant:{tid}"));
        }
        if let Some(ctx) = req.extensions().get::<crate::auth::AuthContext>() {
            return Ok(format!("tenant:{}", ctx.tenant_id));
        }
        Ok("global".to_string())
    }
}

fn tenant_id_from_path(path: &str) -> Option<String> {
    let prefix = "/v1/tenants/";
    let remainder = path.strip_prefix(prefix)?;
    let tid = remainder.split('/').next()?;
    if tid.len() == 36 {
        Some(tid.to_string())
    } else {
        None
    }
}

// NOTE: Rate limiting responses are produced by GovernorConfigBuilder::error_handler.

