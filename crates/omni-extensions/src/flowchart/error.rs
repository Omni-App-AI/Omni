use thiserror::Error;

#[derive(Debug, Error)]
pub enum FlowchartError {
    #[error("Flowchart not found: {0}")]
    NotFound(String),

    #[error("Flowchart validation error: {0}")]
    Validation(String),

    #[error("Node execution error in '{node_id}': {message}")]
    NodeExecution { node_id: String, message: String },

    #[error("Expression evaluation error: {0}")]
    Expression(String),

    #[error("Flowchart execution timed out after {0}ms")]
    Timeout(u64),

    #[error("Maximum node execution count exceeded ({0})")]
    MaxDepth(u32),

    #[error("Trigger node not found: {0}")]
    TriggerNotFound(String),

    #[error("Missing connection: node '{0}' has no outgoing edges")]
    MissingConnection(String),

    #[error("Cycle detected involving node: {0}")]
    CycleDetected(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Callback not available: {0}")]
    CallbackNotAvailable(String),

    #[error("Sub-flow recursion depth exceeded (max {0})")]
    SubFlowDepthExceeded(u32),

    #[error("Node timed out after {0}ms")]
    NodeTimeout(u64),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Guardian blocked content in node '{node_id}': {reason}")]
    GuardianBlocked { node_id: String, reason: String },

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, FlowchartError>;
