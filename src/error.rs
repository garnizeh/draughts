//! The API error model — §9.1.
//!
//! One shape for every failure the HTTP layer can produce, with the status code
//! attached to the variant rather than chosen at the call site.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{what} does not exist")]
    NotFound { what: &'static str },

    #[error("{0}")]
    InvalidRequest(String),

    #[error("the selected move is not legal in the current position")]
    IllegalMove,

    #[error("this match is already over")]
    MatchFinished,

    #[error("this batch cannot be cancelled from its current state")]
    BatchNotCancellable,

    #[error("no engine worker is available")]
    EngineUnavailable,

    /// The MPSC write channel is full and a durable write could not be
    /// accepted. Retryable, and distinct from a failure — under lab load this
    /// is backpressure working as designed (§11.2.5).
    #[error("the write queue is saturated; retry")]
    WriteQueueSaturated,

    /// A stored row carries a `format_version` this build cannot decode.
    /// Not a panic, not a default, not a silently skipped row (§13.7).
    #[error("stored data uses format_version {found}, which this build cannot decode")]
    UnsupportedFormatVersion { found: u32 },

    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}

impl ApiError {
    /// The stable string clients match on. Never derived from the variant name.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "not_found",
            Self::InvalidRequest(_) => "invalid_request",
            Self::IllegalMove => "illegal_move",
            Self::MatchFinished => "match_finished",
            Self::BatchNotCancellable => "batch_not_cancellable",
            Self::EngineUnavailable => "engine_unavailable",
            Self::WriteQueueSaturated => "write_queue_saturated",
            Self::UnsupportedFormatVersion { .. } => "unsupported_format_version",
            Self::Internal(_) => "internal_error",
        }
    }

    pub fn status(&self) -> StatusCode {
        match self {
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            Self::IllegalMove | Self::MatchFinished | Self::BatchNotCancellable => {
                StatusCode::CONFLICT
            }
            Self::UnsupportedFormatVersion { .. } => StatusCode::CONFLICT,
            Self::EngineUnavailable | Self::WriteQueueSaturated => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// Note the absence of a `face_unavailable` variant.
///
/// With a circuit breaker in place, an unavailable model is a *designed* steady
/// state. Commentary served from the canned fallback is a `200` carrying a
/// `fallback_reason`, never an error — a `503` there would tell the client
/// something false (§9.1).
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "serde_json::Map::is_empty")]
    pub details: serde_json::Map<String, serde_json::Value>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // An internal error's message is for the operator, not the client.
        if let Self::Internal(source) = &self {
            tracing::error!(error = ?source, "unhandled internal error");
        }

        let body = ErrorBody {
            error: ErrorDetail {
                code: self.code(),
                message: self.to_string(),
                details: serde_json::Map::new(),
            },
        };

        (self.status(), Json(body)).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;

    /// The codes in §9.1 are a contract. A rename is a breaking API change and
    /// should have to edit this test to happen.
    #[test]
    fn error_codes_and_statuses_match_the_contract() {
        let cases: Vec<(ApiError, &str, StatusCode)> = vec![
            (
                ApiError::NotFound { what: "match" },
                "not_found",
                StatusCode::NOT_FOUND,
            ),
            (
                ApiError::InvalidRequest("bad".into()),
                "invalid_request",
                StatusCode::BAD_REQUEST,
            ),
            (ApiError::IllegalMove, "illegal_move", StatusCode::CONFLICT),
            (
                ApiError::MatchFinished,
                "match_finished",
                StatusCode::CONFLICT,
            ),
            (
                ApiError::BatchNotCancellable,
                "batch_not_cancellable",
                StatusCode::CONFLICT,
            ),
            (
                ApiError::EngineUnavailable,
                "engine_unavailable",
                StatusCode::SERVICE_UNAVAILABLE,
            ),
            (
                ApiError::WriteQueueSaturated,
                "write_queue_saturated",
                StatusCode::SERVICE_UNAVAILABLE,
            ),
            (
                ApiError::UnsupportedFormatVersion { found: 255 },
                "unsupported_format_version",
                StatusCode::CONFLICT,
            ),
        ];

        for (error, code, status) in cases {
            assert_eq!(error.code(), code);
            assert_eq!(error.status(), status, "status for {code}");
        }
    }
}
