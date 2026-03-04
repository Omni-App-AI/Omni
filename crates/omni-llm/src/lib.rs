//! Omni LLM Bridge & Provider System
//!
//! Multi-provider LLM integration with streaming support,
//! credential management, agent loop orchestration, and
//! tool call routing.

pub mod agent;
pub mod bridge;
pub mod credentials;
pub mod error;
pub mod guardian_bridge;
pub mod hooks;
pub mod mcp;
pub mod providers;
pub mod rotation;
pub mod system_prompt;
pub mod tools;
pub mod types;

pub use agent::{AgentLoop, DelegatedActionHandler};
pub use bridge::LLMBridge;
pub use guardian_bridge::ExtensionToolRegistry;
pub use credentials::{Credential, CredentialVault};
pub use error::{LlmError, Result};
pub use hooks::{HookHandler, HookPoint, HookRegistry, HookResult, HookContext};
pub use mcp::McpManager;
pub use providers::LLMProvider;
pub use rotation::{ProviderEntry, ProviderRotation};
pub use tools::NativeToolRegistry;
pub use system_prompt::{ExtensionContext, RuntimeContext, SystemPromptBuilder};
pub use types::{AgentResult, AuthMethod, ChatChunk, ChatMessage, ChatRequest, ChatRole, ModelInfo, ToolCall, ToolSchema};
