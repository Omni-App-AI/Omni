use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use futures::Stream;
use tokio::sync::RwLock;

use crate::credentials::CredentialVault;
use crate::error::{LlmError, Result};
use crate::providers::LLMProvider;
use crate::rotation::{ProviderEntry, ProviderRotation};
use crate::types::{ChatChunk, ChatMessage, ModelInfo, ThinkingEffort, ThinkingMode, ToolSchema};

/// The LLM Bridge orchestrates provider selection, credential retrieval,
/// and streaming chat completions across multiple providers.
pub struct LLMBridge {
    providers: RwLock<HashMap<String, Arc<dyn LLMProvider>>>,
    vault: Arc<CredentialVault>,
    rotation: RwLock<Option<ProviderRotation>>,
}

impl LLMBridge {
    pub fn new(vault: Arc<CredentialVault>) -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
            vault,
            rotation: RwLock::new(None),
        }
    }

    /// Register a provider adapter using its self-reported ID.
    pub async fn register_provider(&self, provider: Arc<dyn LLMProvider>) {
        let id = provider.id().to_string();
        self.providers.write().await.insert(id, provider);
    }

    /// Register a provider adapter under a custom name.
    ///
    /// This is needed when the config key (e.g. "nvidia") differs from the
    /// provider's hardcoded `id()` (e.g. "custom"). The agent loop looks up
    /// providers by the config key, so the bridge must store them under that key.
    pub async fn register_provider_as(&self, name: &str, provider: Arc<dyn LLMProvider>) {
        self.providers
            .write()
            .await
            .insert(name.to_string(), provider);
    }

    /// Configure provider rotation with the given entries.
    pub async fn set_rotation(&self, entries: Vec<ProviderEntry>) {
        *self.rotation.write().await = Some(ProviderRotation::new(entries));
    }

    /// Get a registered provider by ID.
    pub async fn get_provider(&self, provider_id: &str) -> Option<Arc<dyn LLMProvider>> {
        self.providers.read().await.get(provider_id).cloned()
    }

    /// List all registered provider IDs.
    pub async fn list_provider_ids(&self) -> Vec<String> {
        self.providers.read().await.keys().cloned().collect()
    }

    /// Get a reference to the credential vault.
    pub fn vault(&self) -> &Arc<CredentialVault> {
        &self.vault
    }

    /// List available models for a specific provider.
    pub async fn list_models(&self, provider_id: &str) -> Result<Vec<ModelInfo>> {
        let provider = self.get_provider(provider_id).await.ok_or_else(|| {
            LlmError::Provider(format!("Provider '{}' not registered", provider_id))
        })?;

        let credential = self.vault.require(provider_id)?;
        provider.list_models(&credential).await
    }

    /// Send a chat completion request using a specific provider and stream the response.
    pub async fn chat_stream(
        &self,
        provider_id: &str,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolSchema>,
        model: &str,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>> {
        self.chat_stream_with_thinking(
            provider_id, messages, tools, model, max_tokens, temperature, None, None,
        )
        .await
    }

    /// Send a chat completion request with thinking configuration.
    pub async fn chat_stream_with_thinking(
        &self,
        provider_id: &str,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolSchema>,
        model: &str,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
        thinking: Option<ThinkingMode>,
        effort: Option<ThinkingEffort>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>> {
        let provider = self.get_provider(provider_id).await.ok_or_else(|| {
            LlmError::Provider(format!("Provider '{}' not registered", provider_id))
        })?;

        let credential = self.vault.require(provider_id)?;
        let mut request = provider.build_request(messages, tools, model, max_tokens, temperature);
        request.thinking = thinking;
        request.effort = effort;
        provider.chat_stream(&request, &credential).await
    }

    /// Send a chat completion using rotation/fallback.
    /// Tries providers in rotation order, falling back on failure.
    pub async fn chat_stream_with_fallback(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolSchema>,
        model: &str,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
        thinking: Option<ThinkingMode>,
        effort: Option<ThinkingEffort>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>> {
        let rotation = self.rotation.read().await;
        let rotation = rotation.as_ref().ok_or(LlmError::NoProviders)?;

        let max_attempts = rotation.len();
        for _ in 0..max_attempts {
            let (provider_id, credential_id) = rotation
                .next()
                .await
                .ok_or(LlmError::AllProvidersOnCooldown)?;

            let provider = match self.get_provider(provider_id).await {
                Some(p) => p,
                None => {
                    tracing::warn!(provider = provider_id, "Provider not registered, skipping");
                    rotation
                        .report_failure(provider_id, credential_id)
                        .await;
                    continue;
                }
            };

            let credential = match self.vault.retrieve(provider_id)? {
                Some(c) => c,
                None => {
                    tracing::warn!(
                        provider = provider_id,
                        "No credential found, skipping"
                    );
                    rotation
                        .report_failure(provider_id, credential_id)
                        .await;
                    continue;
                }
            };

            let mut request = provider.build_request(
                messages.clone(),
                tools.clone(),
                model,
                max_tokens,
                temperature,
            );
            request.thinking = thinking.clone();
            request.effort = effort;

            match provider.chat_stream(&request, &credential).await {
                Ok(stream) => {
                    rotation
                        .report_success(provider_id, credential_id)
                        .await;
                    return Ok(stream);
                }
                Err(e) => {
                    tracing::warn!(
                        provider = provider_id,
                        error = %e,
                        "Provider failed, trying next"
                    );
                    rotation
                        .report_failure(provider_id, credential_id)
                        .await;
                    continue;
                }
            }
        }

        Err(LlmError::AllProvidersOnCooldown)
    }

    /// Count tokens for messages using a specific provider's tokenizer.
    pub async fn count_tokens(
        &self,
        provider_id: &str,
        messages: &[ChatMessage],
    ) -> Result<usize> {
        let provider = self.get_provider(provider_id).await.ok_or_else(|| {
            LlmError::Provider(format!("Provider '{}' not registered", provider_id))
        })?;
        provider.count_tokens(messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_vault() -> Arc<CredentialVault> {
        Arc::new(CredentialVault::new())
    }

    #[tokio::test]
    async fn test_bridge_register_provider() {
        let bridge = LLMBridge::new(test_vault());

        let provider = Arc::new(crate::providers::openai::OpenAIProvider::new(None));
        bridge.register_provider(provider).await;

        let ids = bridge.list_provider_ids().await;
        assert!(ids.contains(&"openai".to_string()));
    }

    #[tokio::test]
    async fn test_bridge_get_provider() {
        let bridge = LLMBridge::new(test_vault());

        let provider = Arc::new(crate::providers::openai::OpenAIProvider::new(None));
        bridge.register_provider(provider).await;

        let p = bridge.get_provider("openai").await;
        assert!(p.is_some());
        assert_eq!(p.unwrap().id(), "openai");

        let missing = bridge.get_provider("nonexistent").await;
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_bridge_multiple_providers() {
        let bridge = LLMBridge::new(test_vault());

        bridge
            .register_provider(Arc::new(
                crate::providers::openai::OpenAIProvider::new(None),
            ))
            .await;
        bridge
            .register_provider(Arc::new(
                crate::providers::anthropic::AnthropicProvider::new(None),
            ))
            .await;
        bridge
            .register_provider(Arc::new(
                crate::providers::ollama::OllamaProvider::new(None),
            ))
            .await;

        let ids = bridge.list_provider_ids().await;
        assert_eq!(ids.len(), 3);
    }

    #[tokio::test]
    async fn test_bridge_set_rotation() {
        let bridge = LLMBridge::new(test_vault());
        bridge
            .set_rotation(vec![ProviderEntry {
                provider_id: "openai".to_string(),
                credential_id: "key1".to_string(),
                priority: 1,
            }])
            .await;

        let rotation = bridge.rotation.read().await;
        assert!(rotation.is_some());
        assert_eq!(rotation.as_ref().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_bridge_chat_stream_no_provider() {
        let bridge = LLMBridge::new(test_vault());

        let result = bridge
            .chat_stream(
                "nonexistent",
                vec![ChatMessage::user("Hi")],
                vec![],
                "model",
                None,
                None,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_bridge_chat_stream_with_thinking_no_provider() {
        let bridge = LLMBridge::new(test_vault());

        // Should fail with provider not found, but exercising the thinking param path
        let result = bridge
            .chat_stream_with_thinking(
                "nonexistent",
                vec![ChatMessage::user("Hi")],
                vec![],
                "model",
                None,
                None,
                Some(ThinkingMode::Adaptive),
                Some(ThinkingEffort::High),
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_bridge_fallback_no_rotation() {
        let bridge = LLMBridge::new(test_vault());

        let result = bridge
            .chat_stream_with_fallback(
                vec![ChatMessage::user("Hi")],
                vec![],
                "model",
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(matches!(result, Err(LlmError::NoProviders)));
    }
}
