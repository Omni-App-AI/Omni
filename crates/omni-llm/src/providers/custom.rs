use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;

use crate::credentials::Credential;
use crate::error::{LlmError, Result};
use crate::types::{AuthMethod, ChatChunk, ChatMessage, ChatRequest, ModelInfo};

use super::LLMProvider;

/// Custom HTTP adapter using OpenAI-compatible API format.
///
/// For self-hosted models, corporate proxies, or niche providers
/// that expose an OpenAI-compatible API.
pub struct CustomProvider {
    client: Client,
    endpoint: String,
    provider_name: String,
}

impl CustomProvider {
    pub fn new(endpoint: &str, name: Option<&str>) -> Self {
        let mut cleaned = endpoint.trim_end_matches('/').to_string();
        if cleaned.ends_with("/chat/completions") {
            cleaned = cleaned
                .strip_suffix("/chat/completions")
                .unwrap()
                .to_string();
        }

        Self {
            client: Client::new(),
            endpoint: cleaned,
            provider_name: name.unwrap_or("Custom").to_string(),
        }
    }

    fn build_api_body(&self, request: &ChatRequest) -> serde_json::Value {
        // Uses OpenAI-compatible format
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                // For tool messages with images, format content as a multimodal array
                let content = if m.role == crate::types::ChatRole::Tool {
                    if let Some(ref images) = m.images {
                        let mut blocks: Vec<serde_json::Value> = Vec::new();
                        if !m.content.is_empty() {
                            blocks.push(serde_json::json!({
                                "type": "text",
                                "text": m.content,
                            }));
                        }
                        for img in images {
                            blocks.push(serde_json::json!({
                                "type": "image_url",
                                "image_url": {
                                    "url": format!("data:{};base64,{}", img.mime_type, img.data),
                                },
                            }));
                        }
                        serde_json::json!(blocks)
                    } else {
                        serde_json::json!(m.content)
                    }
                } else {
                    serde_json::json!(m.content)
                };

                let mut msg = serde_json::json!({
                    "role": m.role,
                    "content": content,
                });
                if let Some(ref tc_id) = m.tool_call_id {
                    msg["tool_call_id"] = serde_json::json!(tc_id);
                }
                if let Some(ref tcs) = m.tool_calls {
                    let tool_calls: Vec<serde_json::Value> = tcs
                        .iter()
                        .map(|tc| {
                            serde_json::json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": tc.arguments,
                                }
                            })
                        })
                        .collect();
                    msg["tool_calls"] = serde_json::json!(tool_calls);
                }
                msg
            })
            .collect();

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "stream": request.stream,
        });

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        if !request.tools.is_empty() {
            let tools: Vec<serde_json::Value> = request
                .tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters,
                        }
                    })
                })
                .collect();
            body["tools"] = serde_json::json!(tools);
        }

        body
    }
}

#[async_trait]
impl LLMProvider for CustomProvider {
    fn id(&self) -> &str {
        "custom"
    }

    fn display_name(&self) -> &str {
        &self.provider_name
    }

    async fn list_models(&self, credential: &Credential) -> Result<Vec<ModelInfo>> {
        // Try to list models from the /models endpoint (OpenAI-compatible)
        let mut req = self.client.get(format!("{}/models", self.endpoint));

        if let Some(token) = credential.bearer_token() {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let resp = req.send().await?;

        if !resp.status().is_success() {
            // Not all custom endpoints support /models -- return empty
            return Ok(Vec::new());
        }

        #[derive(serde::Deserialize)]
        struct ModelsResp {
            data: Vec<ModelEntry>,
        }
        #[derive(serde::Deserialize)]
        struct ModelEntry {
            id: String,
        }

        match resp.json::<ModelsResp>().await {
            Ok(body) => Ok(body
                .data
                .into_iter()
                .map(|m| ModelInfo {
                    id: m.id.clone(),
                    name: m.id,
                    context_window: 4096,
                    max_output_tokens: None,
                })
                .collect()),
            Err(_) => Ok(Vec::new()),
        }
    }

    async fn chat_stream(
        &self,
        request: &ChatRequest,
        credential: &Credential,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>> {
        let body = self.build_api_body(request);

        let mut req = self
            .client
            .post(format!("{}/chat/completions", self.endpoint))
            .header("Content-Type", "application/json")
            .json(&body);

        if let Some(token) = credential.bearer_token() {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let resp = req.send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::Provider(format!(
                "Custom provider chat failed ({}): {}",
                status, body
            )));
        }

        // Reuse OpenAI SSE parsing since the format is OpenAI-compatible
        let stream = super::openai::parse_openai_sse_stream(resp);
        Ok(Box::pin(stream))
    }

    fn count_tokens(&self, messages: &[ChatMessage]) -> Result<usize> {
        // Rough estimation for unknown tokenizer
        let total_chars: usize = messages.iter().map(|m| m.content.len()).sum();
        Ok(total_chars / 4 + messages.len() * 4)
    }

    fn auth_methods(&self) -> Vec<AuthMethod> {
        vec![AuthMethod::ApiKey { env_var_hint: None }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_provider_id() {
        let provider = CustomProvider::new("http://localhost:8080/v1", None);
        assert_eq!(provider.id(), "custom");
        assert_eq!(provider.display_name(), "Custom");
    }

    #[test]
    fn test_custom_provider_named() {
        let provider = CustomProvider::new("https://proxy.corp.com/v1", Some("Corp LLM"));
        assert_eq!(provider.display_name(), "Corp LLM");
    }

    #[test]
    fn test_custom_build_api_body() {
        let provider = CustomProvider::new("http://localhost:8080/v1", None);
        let request = ChatRequest {
            model: "my-model".to_string(),
            messages: vec![ChatMessage::user("Hello")],
            tools: vec![],
            max_tokens: Some(500),
            temperature: Some(0.9),
            stream: true,
            thinking: None,
            effort: None,
        };

        let body = provider.build_api_body(&request);
        assert_eq!(body["model"], "my-model");
        assert_eq!(body["stream"], true);
        assert_eq!(body["max_tokens"], 500);
        assert!((body["temperature"].as_f64().unwrap() - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_custom_auth_methods() {
        let provider = CustomProvider::new("http://localhost:8080/v1", None);
        let methods = provider.auth_methods();
        assert_eq!(methods.len(), 1);
        match &methods[0] {
            AuthMethod::ApiKey { env_var_hint } => {
                assert!(env_var_hint.is_none());
            }
            _ => panic!("Expected ApiKey"),
        }
    }

    #[test]
    fn test_custom_count_tokens() {
        let provider = CustomProvider::new("http://localhost:8080/v1", None);
        let messages = vec![ChatMessage::user("Hello world!")];
        let count = provider.count_tokens(&messages).unwrap();
        assert!(count > 0);
    }

    #[test]
    fn test_custom_build_api_body_with_images() {
        let provider = CustomProvider::new("http://localhost:8080/v1", None);
        let request = ChatRequest {
            model: "my-model".to_string(),
            messages: vec![ChatMessage::tool_result_with_images(
                "call_123",
                "App opened",
                vec![crate::types::ImageContent {
                    mime_type: "image/png".to_string(),
                    data: "base64data".to_string(),
                }],
            )],
            tools: vec![],
            max_tokens: None,
            temperature: None,
            stream: true,
            thinking: None,
            effort: None,
        };

        let body = provider.build_api_body(&request);
        let messages = body["messages"].as_array().unwrap();
        let content = &messages[0]["content"];

        assert!(content.is_array());
        let items = content.as_array().unwrap();
        assert_eq!(items.len(), 2);

        assert_eq!(items[0]["type"], "text");
        assert_eq!(items[0]["text"], "App opened");

        assert_eq!(items[1]["type"], "image_url");
        assert_eq!(
            items[1]["image_url"]["url"],
            "data:image/png;base64,base64data"
        );
    }
}
