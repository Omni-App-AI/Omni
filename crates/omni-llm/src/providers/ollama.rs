use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde::Deserialize;

use crate::credentials::Credential;
use crate::error::{LlmError, Result};
use crate::types::{AuthMethod, ChatChunk, ChatMessage, ChatRequest, ChatRole, ModelInfo};

use super::LLMProvider;

const DEFAULT_ENDPOINT: &str = "http://localhost:11434";

/// Ollama provider adapter.
///
/// Supports any locally-hosted model via Ollama's API.
/// No authentication required (local API).
pub struct OllamaProvider {
    client: Client,
    endpoint: String,
}

impl OllamaProvider {
    pub fn new(endpoint: Option<&str>) -> Self {
        Self {
            client: Client::new(),
            endpoint: endpoint.unwrap_or(DEFAULT_ENDPOINT).trim_end_matches('/').to_string(),
        }
    }

    fn build_api_body(&self, request: &ChatRequest) -> serde_json::Value {
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    ChatRole::System => "system",
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                    ChatRole::Tool => "tool",
                };
                let mut msg = serde_json::json!({
                    "role": role,
                    "content": m.content,
                });
                if let Some(ref tcs) = m.tool_calls {
                    let tool_calls: Vec<serde_json::Value> = tcs
                        .iter()
                        .map(|tc| {
                            let args: serde_json::Value =
                                serde_json::from_str(&tc.arguments).unwrap_or_default();
                            serde_json::json!({
                                "function": {
                                    "name": tc.name,
                                    "arguments": args,
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

        let mut options = serde_json::Map::new();
        if let Some(temp) = request.temperature {
            options.insert(
                "temperature".to_string(),
                serde_json::json!(temp),
            );
        }
        if let Some(max_tokens) = request.max_tokens {
            options.insert(
                "num_predict".to_string(),
                serde_json::json!(max_tokens),
            );
        }
        if !options.is_empty() {
            body["options"] = serde_json::Value::Object(options);
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
impl LLMProvider for OllamaProvider {
    fn id(&self) -> &str {
        "ollama"
    }

    fn display_name(&self) -> &str {
        "Ollama"
    }

    async fn list_models(&self, _credential: &Credential) -> Result<Vec<ModelInfo>> {
        let resp = self
            .client
            .get(format!("{}/api/tags", self.endpoint))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::Provider(format!(
                "Ollama list_models failed ({}): {}",
                status, body
            )));
        }

        let body: OllamaTagsResponse = resp.json().await?;
        let models = body
            .models
            .into_iter()
            .map(|m| ModelInfo {
                id: m.name.clone(),
                name: m.name,
                context_window: 4096, // Default -- Ollama doesn't expose this
                max_output_tokens: None,
            })
            .collect();

        Ok(models)
    }

    async fn chat_stream(
        &self,
        request: &ChatRequest,
        _credential: &Credential,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>> {
        let body = self.build_api_body(request);

        let resp = self
            .client
            .post(format!("{}/api/chat", self.endpoint))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::Provider(format!(
                "Ollama chat failed ({}): {}",
                status, body
            )));
        }

        let stream = parse_ollama_ndjson_stream(resp);
        Ok(Box::pin(stream))
    }

    fn count_tokens(&self, messages: &[ChatMessage]) -> Result<usize> {
        // Ollama doesn't provide a tokenizer API.
        // Use a rough estimation: ~4 characters per token.
        let total_chars: usize = messages.iter().map(|m| m.content.len()).sum();
        Ok(total_chars / 4 + messages.len() * 4)
    }

    fn auth_methods(&self) -> Vec<AuthMethod> {
        // Ollama is local -- no authentication required.
        vec![AuthMethod::Custom {
            instructions: "No authentication required. Ensure Ollama is running locally."
                .to_string(),
        }]
    }
}

/// Parse Ollama's NDJSON streaming response into ChatChunks.
fn parse_ollama_ndjson_stream(
    resp: reqwest::Response,
) -> impl Stream<Item = Result<ChatChunk>> + Send {
    let byte_stream = resp.bytes_stream();

    futures::stream::unfold(
        (byte_stream, String::new()),
        |(mut byte_stream, mut buffer)| async move {
            use futures::TryStreamExt;

            loop {
                // Try to extract a complete line
                if let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].to_string();
                    buffer = buffer[pos + 1..].to_string();

                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<OllamaChatResponse>(line) {
                        Ok(resp) => {
                            if resp.done {
                                // Emit usage if available, then Done
                                if let (Some(pt), Some(et)) =
                                    (resp.prompt_eval_count, resp.eval_count)
                                {
                                    return Some((
                                        Ok(ChatChunk::Usage {
                                            prompt_tokens: pt,
                                            completion_tokens: et,
                                            total_tokens: pt + et,
                                        }),
                                        (byte_stream, buffer),
                                    ));
                                }
                                return Some((
                                    Ok(ChatChunk::Done),
                                    (byte_stream, buffer),
                                ));
                            }

                            if let Some(msg) = resp.message {
                                // Check for tool calls
                                if let Some(tool_calls) = msg.tool_calls {
                                    if let Some((i, tc)) = tool_calls.into_iter().enumerate().next() {
                                        let args = serde_json::to_string(&tc.function.arguments)
                                            .unwrap_or_default();
                                        return Some((
                                            Ok(ChatChunk::ToolCallDelta {
                                                index: i,
                                                id: Some(format!("ollama_tc_{}", i)),
                                                name: Some(tc.function.name),
                                                arguments_delta: args,
                                            }),
                                            (byte_stream, buffer),
                                        ));
                                    }
                                }

                                if !msg.content.is_empty() {
                                    return Some((
                                        Ok(ChatChunk::TextDelta(msg.content)),
                                        (byte_stream, buffer),
                                    ));
                                }
                            }
                            continue;
                        }
                        Err(e) => {
                            return Some((
                                Err(LlmError::Stream(format!(
                                    "Failed to parse NDJSON: {}",
                                    e
                                ))),
                                (byte_stream, buffer),
                            ));
                        }
                    }
                }

                // Need more data
                match byte_stream.try_next().await {
                    Ok(Some(bytes)) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                    }
                    Ok(None) => return None,
                    Err(e) => {
                        return Some((
                            Err(LlmError::Stream(format!("NDJSON read error: {}", e))),
                            (byte_stream, buffer),
                        ));
                    }
                }
            }
        },
    )
}

// ---- Ollama API response types ----

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    #[serde(default)]
    done: bool,
    message: Option<OllamaChatMessage>,
    prompt_eval_count: Option<u32>,
    eval_count: Option<u32>,
}

#[derive(Deserialize)]
struct OllamaChatMessage {
    #[serde(default)]
    content: String,
    tool_calls: Option<Vec<OllamaToolCall>>,
}

#[derive(Deserialize)]
struct OllamaToolCall {
    function: OllamaFunction,
}

#[derive(Deserialize)]
struct OllamaFunction {
    name: String,
    arguments: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolSchema;

    #[test]
    fn test_ollama_provider_id() {
        let provider = OllamaProvider::new(None);
        assert_eq!(provider.id(), "ollama");
        assert_eq!(provider.display_name(), "Ollama");
        assert_eq!(provider.endpoint, "http://localhost:11434");
    }

    #[test]
    fn test_ollama_custom_endpoint() {
        let provider = OllamaProvider::new(Some("http://192.168.1.100:11434/"));
        assert_eq!(provider.endpoint, "http://192.168.1.100:11434");
    }

    #[test]
    fn test_ollama_auth_methods() {
        let provider = OllamaProvider::new(None);
        let methods = provider.auth_methods();
        assert_eq!(methods.len(), 1);
        assert!(matches!(&methods[0], AuthMethod::Custom { .. }));
    }

    #[test]
    fn test_ollama_count_tokens() {
        let provider = OllamaProvider::new(None);
        let messages = vec![ChatMessage::user("Hello world!")];
        let count = provider.count_tokens(&messages).unwrap();
        assert!(count > 0);
    }

    #[test]
    fn test_ollama_build_api_body() {
        let provider = OllamaProvider::new(None);
        let request = ChatRequest {
            model: "llama3.1".to_string(),
            messages: vec![
                ChatMessage::system("Be helpful"),
                ChatMessage::user("Hi"),
            ],
            tools: vec![],
            max_tokens: Some(500),
            temperature: Some(0.8),
            stream: true,
            thinking: None,
            effort: None,
        };

        let body = provider.build_api_body(&request);
        assert_eq!(body["model"], "llama3.1");
        assert_eq!(body["stream"], true);
        assert!((body["options"]["temperature"].as_f64().unwrap() - 0.8).abs() < 0.01);
        assert_eq!(body["options"]["num_predict"], 500);

        let msgs = body["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[1]["role"], "user");
    }

    #[test]
    fn test_ollama_build_api_body_with_tools() {
        let provider = OllamaProvider::new(None);
        let request = ChatRequest {
            model: "llama3.1".to_string(),
            messages: vec![ChatMessage::user("What time is it?")],
            tools: vec![ToolSchema {
                name: "get_time".to_string(),
                description: "Get current time".to_string(),
                parameters: serde_json::json!({"type": "object", "properties": {}}),
                required_permission: None,
            }],
            max_tokens: None,
            temperature: None,
            stream: true,
            thinking: None,
            effort: None,
        };

        let body = provider.build_api_body(&request);
        assert!(body.get("options").is_none()); // No options when not set
        let tools = body["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["function"]["name"], "get_time");
    }

    #[test]
    fn test_parse_ollama_chat_response_text() {
        let json = r#"{"done":false,"message":{"role":"assistant","content":"Hello"}}"#;
        let resp: OllamaChatResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.done);
        assert_eq!(resp.message.unwrap().content, "Hello");
    }

    #[test]
    fn test_parse_ollama_chat_response_done() {
        let json = r#"{"done":true,"prompt_eval_count":10,"eval_count":20}"#;
        let resp: OllamaChatResponse = serde_json::from_str(json).unwrap();
        assert!(resp.done);
        assert_eq!(resp.prompt_eval_count, Some(10));
        assert_eq!(resp.eval_count, Some(20));
    }
}
