use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;

/// Unified API error type that renders as a JSON error envelope.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        details: Option<serde_json::Value>,
    },

    #[error("Unauthorized: {message}")]
    Unauthorized { code: String, message: String },

    #[error("Forbidden: {message}")]
    Forbidden { message: String },

    #[error("Not found: {message}")]
    NotFound { code: String, message: String },

    #[error("Conflict: {message}")]
    Conflict { code: String, message: String },

    #[error("Payload too large")]
    PayloadTooLarge,

    #[error("Rate limited")]
    RateLimited,

    #[error("Internal error: {message}")]
    Internal { message: String },
}

/// JSON error envelope returned to clients.
#[derive(Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
    meta: ErrorMeta,
}

#[derive(Serialize)]
struct ErrorBody {
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct ErrorMeta {
    timestamp: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message, details) = match &self {
            Self::Validation {
                message, details, ..
            } => (
                StatusCode::BAD_REQUEST,
                "VALIDATION_ERROR".to_string(),
                message.clone(),
                details.clone(),
            ),
            Self::Unauthorized { code, message } => (
                StatusCode::UNAUTHORIZED,
                code.clone(),
                message.clone(),
                None,
            ),
            Self::Forbidden { message } => (
                StatusCode::FORBIDDEN,
                "FORBIDDEN".to_string(),
                message.clone(),
                None,
            ),
            Self::NotFound { code, message } => (
                StatusCode::NOT_FOUND,
                code.clone(),
                message.clone(),
                None,
            ),
            Self::Conflict { code, message } => (
                StatusCode::CONFLICT,
                code.clone(),
                message.clone(),
                None,
            ),
            Self::PayloadTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "PAYLOAD_TOO_LARGE".to_string(),
                "Request body exceeds maximum allowed size".to_string(),
                None,
            ),
            Self::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMIT_EXCEEDED".to_string(),
                "Too many requests, please try again later".to_string(),
                None,
            ),
            Self::Internal { message } => {
                tracing::error!("Internal error: {message}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR".to_string(),
                    "An internal error occurred".to_string(),
                    None,
                )
            }
        };

        let envelope = ErrorEnvelope {
            error: ErrorBody {
                code,
                message,
                details,
            },
            meta: ErrorMeta {
                timestamp: chrono::Utc::now().to_rfc3339(),
            },
        };

        (status, Json(envelope)).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            sqlx::Error::RowNotFound => Self::NotFound {
                code: "NOT_FOUND".to_string(),
                message: "The requested resource was not found".to_string(),
            },
            sqlx::Error::Database(db_err) => {
                // PostgreSQL unique constraint violation
                if db_err.code().as_deref() == Some("23505") {
                    return Self::Conflict {
                        code: "CONFLICT".to_string(),
                        message: db_err
                            .message()
                            .to_string(),
                    };
                }
                // Foreign key violation
                if db_err.code().as_deref() == Some("23503") {
                    return Self::Validation {
                        message: "Referenced resource does not exist".to_string(),
                        details: Some(json!({ "constraint": db_err.message() })),
                    };
                }
                tracing::error!("Database error: {db_err}");
                Self::Internal {
                    message: "Database error".to_string(),
                }
            }
            _ => {
                tracing::error!("Database error: {err}");
                Self::Internal {
                    message: "Database error".to_string(),
                }
            }
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        tracing::error!("Internal error: {err:#}");
        Self::Internal {
            message: "An internal error occurred".to_string(),
        }
    }
}
