use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use once_cell::sync::Lazy;
use prometheus_client::{
    encoding::text::encode,
    metrics::{
        counter::Counter,
        family::Family,
        histogram::{exponential_buckets, Histogram},
    },
    registry::Registry,
};
use prometheus_client_derive_encode::EncodeLabelSet;
use std::time::Instant;

use crate::state::AppState;

fn metrics_enabled() -> bool {
    std::env::var("DMSX_API_METRICS_ENABLED")
        .ok()
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(true)
}

fn metrics_bearer() -> Option<String> {
    std::env::var("DMSX_API_METRICS_BEARER")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct RequestLabels {
    pub method: String,
    pub path_group: String,
    pub status: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct DurationLabels {
    pub method: String,
    pub path_group: String,
}

static REGISTRY: Lazy<std::sync::Mutex<Registry>> = Lazy::new(|| std::sync::Mutex::new(build_registry()));
static HTTP_REQUESTS_TOTAL: Lazy<Family<RequestLabels, Counter>> = Lazy::new(Family::default);
fn request_duration_histogram() -> Histogram {
    // 5ms .. ~10s
    Histogram::new(exponential_buckets(0.005, 2.0, 12))
}

static HTTP_REQUEST_DURATION_SECONDS: Lazy<Family<DurationLabels, Histogram>> =
    Lazy::new(|| Family::<DurationLabels, Histogram>::new_with_constructor(request_duration_histogram));

fn build_registry() -> Registry {
    let mut reg = Registry::default();
    reg.register("http_requests_total", "Total number of HTTP requests", HTTP_REQUESTS_TOTAL.clone());
    reg.register(
        "http_request_duration_seconds",
        "HTTP request duration in seconds",
        HTTP_REQUEST_DURATION_SECONDS.clone(),
    );
    reg
}

fn status_group(code: StatusCode) -> String {
    let n = code.as_u16();
    format!("{}xx", n / 100)
}

fn path_group(path: &str) -> &'static str {
    if path == "/health" {
        return "/health";
    }
    if path == "/ready" {
        return "/ready";
    }
    if path == "/metrics" {
        return "/metrics";
    }
    if path.starts_with("/v1/tenants/") {
        return "/v1/tenants/{tenant_id}/...";
    }
    if path.starts_with("/v1/config/") {
        return "/v1/config/...";
    }
    if path.starts_with("/v1/") {
        return "/v1/...";
    }
    "/other"
}

pub async fn metrics_handler(
    State(_st): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !metrics_enabled() {
        return StatusCode::NOT_FOUND.into_response();
    }
    if let Some(expected) = metrics_bearer() {
        let expected_header = if expected.to_ascii_lowercase().starts_with("bearer ") {
            expected
        } else {
            format!("Bearer {expected}")
        };
        let actual = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if actual != expected_header {
            return StatusCode::UNAUTHORIZED.into_response();
        }
    }

    let mut body = String::new();
    if let Ok(reg) = REGISTRY.lock() {
        let _ = encode(&mut body, &*reg);
    }
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
        .into_response()
}

pub async fn metrics_middleware(
    req: axum::extract::Request,
    next: Next,
) -> axum::response::Response {
    let start = Instant::now();
    let method = req.method().as_str().to_string();
    let pg = path_group(req.uri().path()).to_string();
    let resp = next.run(req).await;
    let code = resp.status();

    let labels = RequestLabels {
        method,
        path_group: pg,
        status: status_group(code),
    };
    HTTP_REQUESTS_TOTAL.get_or_create(&labels).inc();

    let dlabels = DurationLabels {
        method: labels.method.clone(),
        path_group: labels.path_group.clone(),
    };
    HTTP_REQUEST_DURATION_SECONDS
        .get_or_create(&dlabels)
        .observe(start.elapsed().as_secs_f64());

    resp
}

