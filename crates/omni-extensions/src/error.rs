use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExtensionError {
    #[error("Manifest error: {0}")]
    Manifest(#[from] ManifestError),

    #[error("Sandbox error: {0}")]
    Sandbox(String),

    #[error("Extension not found: {0}")]
    NotFound(String),

    #[error("Extension not active: {0}")]
    NotActive(String),

    #[error("Extension disabled: {0}")]
    Disabled(String),

    #[error("Extension missing entrypoint: {0}")]
    MissingEntrypoint(String),

    #[error("No memory export in WASM module")]
    NoMemory,

    #[error("Tool invocation timed out")]
    Timeout,

    #[error("Invalid UTF-8 from extension")]
    InvalidUtf8,

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("WASM error: {0}")]
    Wasm(String),

    #[error("Core error: {0}")]
    Core(#[from] omni_core::error::OmniError),

    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("Invalid extension ID '{0}': must be reverse-domain format with at least 5 characters")]
    InvalidId(String),

    #[error("Invalid version '{0}': must be valid SemVer")]
    InvalidVersion(String),

    #[error("Unknown capability: {0}")]
    UnknownCapability(String),

    #[error("Invalid tool schema for '{tool}': {error}")]
    InvalidToolSchema { tool: String, error: String },

    #[error("Invalid runtime config: {0}")]
    InvalidRuntimeConfig(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, ExtensionError>;
