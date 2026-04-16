use serde::Serialize;
use thiserror::Error;

/// 平台统一错误。
///
/// 启用 `axum` feature 后自动实现 `IntoResponse`，输出 RFC 7807 Problem Details。
#[derive(Debug, Error)]
pub enum DmsxError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("validation: {0}")]
    Validation(String),
    #[error("too many requests: {0}")]
    TooManyRequests(String),
    #[error("payload too large: {0}")]
    PayloadTooLarge(String),
    #[error("internal: {0}")]
    Internal(String),
}

/// RFC 7807 Problem Details 序列化体。
#[derive(Debug, Serialize)]
pub struct ProblemDetails {
    pub r#type: &'static str,
    pub title: &'static str,
    pub status: u16,
    pub detail: String,
}

impl DmsxError {
    pub fn problem_details(&self) -> ProblemDetails {
        match self {
            Self::NotFound(d) => ProblemDetails {
                r#type: "about:blank",
                title: "Not Found",
                status: 404,
                detail: d.clone(),
            },
            Self::Unauthorized(d) => ProblemDetails {
                r#type: "about:blank",
                title: "Unauthorized",
                status: 401,
                detail: d.clone(),
            },
            Self::Forbidden(d) => ProblemDetails {
                r#type: "about:blank",
                title: "Forbidden",
                status: 403,
                detail: d.clone(),
            },
            Self::Conflict(d) => ProblemDetails {
                r#type: "about:blank",
                title: "Conflict",
                status: 409,
                detail: d.clone(),
            },
            Self::Validation(d) => ProblemDetails {
                r#type: "about:blank",
                title: "Bad Request",
                status: 400,
                detail: d.clone(),
            },
            Self::TooManyRequests(d) => ProblemDetails {
                r#type: "about:blank",
                title: "Too Many Requests",
                status: 429,
                detail: d.clone(),
            },
            Self::PayloadTooLarge(d) => ProblemDetails {
                r#type: "about:blank",
                title: "Payload Too Large",
                status: 413,
                detail: d.clone(),
            },
            Self::Internal(d) => ProblemDetails {
                r#type: "about:blank",
                title: "Internal Server Error",
                status: 500,
                detail: d.clone(),
            },
        }
    }
}

#[cfg(feature = "axum")]
mod axum_impl {
    use super::*;
    use axum::response::{IntoResponse, Response};
    use http::StatusCode;

    impl IntoResponse for DmsxError {
        fn into_response(self) -> Response {
            let pd = self.problem_details();
            let status =
                StatusCode::from_u16(pd.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let body = axum::Json(pd);
            (
                status,
                [(
                    http::header::CONTENT_TYPE,
                    http::HeaderValue::from_static("application/problem+json"),
                )],
                body,
            )
                .into_response()
        }
    }
}
