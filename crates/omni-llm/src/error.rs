use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Credential not found for provider: {0}")]
    CredentialNotFound(String),

    #[error("Keyring error: {0}")]
    Keyring(String),

    #[error("Streaming error: {0}")]
    Stream(String),

    #[error("Token limit exceeded: {used} / {limit}")]
    TokenLimitExceeded { used: usize, limit: usize },

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("All providers on cooldown")]
    AllProvidersOnCooldown,

    #[error("No providers configured")]
    NoProviders,

    #[error("Agent loop exceeded max iterations ({0})")]
    MaxIterationsExceeded(u32),

    #[error("Guardian blocked: {0}")]
    GuardianBlocked(String),

    #[error("Hook blocked: {0}")]
    HookBlocked(String),

    #[error("Tool call error: {0}")]
    ToolCall(String),

    #[error("Invalid tool name format: {0}")]
    InvalidToolName(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Timeout after {0} seconds")]
    Timeout(u64),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("Core error: {0}")]
    Core(#[from] omni_core::error::OmniError),

    #[error("Extension error: {0}")]
    Extension(#[from] omni_extensions::error::ExtensionError),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, LlmError>;
