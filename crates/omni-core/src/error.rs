use thiserror::Error;

#[derive(Debug, Error)]
pub enum OmniError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("Keyring error: {0}")]
    Keyring(String),

    #[error("File watch error: {0}")]
    Notify(#[from] notify::Error),

    #[error("Permission error: {0}")]
    Permission(String),

    #[error("Scope violation: {0}")]
    ScopeViolation(String),

    #[error("CSV error: {0}")]
    Csv(String),

    #[error("Extension error: {0}")]
    Extension(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, OmniError>;
