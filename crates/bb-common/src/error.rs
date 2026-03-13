use thiserror::Error;

#[derive(Debug, Error)]
pub enum BetBlockerError {
    #[error("not found: {entity} with id {id}")]
    NotFound { entity: &'static str, id: String },

    #[error("unauthorized: {reason}")]
    Unauthorized { reason: String },

    #[error("forbidden: {reason}")]
    Forbidden { reason: String },

    #[error("validation error: {message}")]
    Validation { message: String },

    #[error("conflict: {message}")]
    Conflict { message: String },

    #[error("enrollment policy violation: {message}")]
    PolicyViolation { message: String },

    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}
