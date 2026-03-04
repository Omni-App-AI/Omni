pub mod anthropic;
pub mod bedrock;
pub mod custom;
pub mod google;
pub mod ollama;
pub mod openai;
pub mod openai_ws;

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use crate::credentials::Credential;
use crate::error::Result;
use crate::types::{AuthMethod, ChatChunk, ChatMessage, ChatRequest, ModelInfo, ToolSchema};

/// Trait implemented by each LLM provider adapter.
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Unique provider identifier (e.g., "openai", "anthropic", "ollama").
    fn id(&self) -> &str;

    /// Human-readable display name.
    fn display_name(&self) -> &str;

    /// List available models for this provider.
    async fn list_models(&self, credential: &Credential) -> Result<Vec<ModelInfo>>;

    /// Send a chat completion request and stream the response.
    async fn chat_stream(
        &self,
        request: &ChatRequest,
        credential: &Credential,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>>;

    /// Count tokens for a given message set (provider-specific tokenizer).
    fn count_tokens(&self, messages: &[ChatMessage]) -> Result<usize>;

    /// Supported authentication methods.
    fn auth_methods(&self) -> Vec<AuthMethod>;

    /// Build a ChatRequest from messages and tool schemas with provider defaults.
    fn build_request(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolSchema>,
        model: &str,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
    ) -> ChatRequest {
        ChatRequest {
            model: model.to_string(),
            messages,
            tools,
            max_tokens,
            temperature,
            stream: true,
            thinking: None,
            effort: None,
        }
    }
}
