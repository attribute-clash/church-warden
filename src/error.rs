use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("authentication failed")]
    AuthenticationFailed,
    #[error("authorization denied")]
    AuthorizationDenied,
    #[error("entity not found: {0}")]
    NotFound(String),
    #[error("invalid operation: {0}")]
    InvalidOperation(String),
}

pub type AppResult<T> = Result<T, AppError>;
