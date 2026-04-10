use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("invalid json input: {0}")]
    InvalidJsonInput(String),
    #[error("missing dependency: {0}")]
    MissingDependency(String),
    #[error("adapter error: {0}")]
    Adapter(String),
    #[error("secret storage error: {0}")]
    SecretStore(String),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
