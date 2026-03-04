use thiserror::Error;

#[derive(Debug, Error)]
pub enum GuardianError {
    #[error("Signature database error: {0}")]
    SignatureDb(String),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("ML model error: {0}")]
    Model(String),

    #[error("Tokenizer error: {0}")]
    Tokenizer(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, GuardianError>;
