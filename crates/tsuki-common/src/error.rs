use thiserror::Error;

#[derive(Error, Debug)]
pub enum TsukiError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Target is protected: {0}")]
    ProtectedTarget(String),

    #[error("Action expired or timed out")]
    ActionExpired,

    #[error("Rollback error: {0}")]
    Rollback(String),

    #[error("Journal error: {0}")]
    Journal(String),
}

pub type Result<T> = std::result::Result<T, TsukiError>;
