use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde::Deserialize;

use crate::credentials::Credential;
use crate::error::{LlmError, Result};
use crate::types::{AuthMethod, ChatChunk, ChatMessage, ChatRequest, ChatRole, ModelInfo};

use super::LLMProvider;

const DEFAULT_ENDPOINT: &str = "https://api.openai.com/v1";

/// OpenAI provider adapter.
///
/// Supports GPT-5.2, GPT-4.1, o3, o4-mini, and custom models.
/// Transport can be configured: SSE (default), WebSocket, or Auto (WS with SSE fallback).
pub struct OpenAIProvider {
    client: Client,
    endpoint: String,
    transport: super::openai_ws::OpenAITransport,
}

impl OpenAIProvider {
    pub fn new(endpoint: Option<&str>) -> Self {
        Self {
            client: Client::new(),
            endpoint: endpoint.unwrap_or(DEFAULT_ENDPOINT).trim_end_matches('/').to_string(),
            transport: super::openai_ws::OpenAITransport::Sse,
        }
    }

    /// Create an OpenAI provider with a specific transport mode.
    pub fn with_transport(endpoint: Option<&str>, transport: super::openai_ws::OpenAITransport) -> Self {
        Self {
            client: Client::new(),
            endpoint: endpoint.unwrap_or(DEFAULT_ENDPOINT).trim_end_matches('/').to_string(),
            transport,
        }
    }

    fn build_api_body(
        &self,
        request: &ChatRequest,
    ) -> serde_json::Value {
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                // For tool messages with images, format content as a multimodal array
                let content = if m.role == ChatRole::Tool {
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
            body["max_completion_tokens"] = serde_json::json!(max_tokens);
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

impl OpenAIProvider {
    /// SSE-based streaming (original implementation).
    async fn chat_stream_sse(
        &self,
        request: &ChatRequest,
        token: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>> {
        let body = self.build_api_body(request);

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.endpoint))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::Provider(format!(
                "OpenAI chat failed ({}): {}",
                status, body
            )));
        }

        let stream = parse_openai_sse_stream(resp);
        Ok(Box::pin(stream))
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn id(&self) -> &str {
        "openai"
    }

    fn display_name(&self) -> &str {
        "OpenAI"
    }

    async fn list_models(&self, credential: &Credential) -> Result<Vec<ModelInfo>> {
        let token = credential
            .bearer_token()
            .ok_or_else(|| LlmError::Auth("No bearer token available".to_string()))?;

        let resp = self
            .client
            .get(format!("{}/models", self.endpoint))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::Provider(format!(
                "OpenAI list_models failed ({}): {}",
                status, body
            )));
        }

        let body: OpenAIModelsResponse = resp.json().await?;
        let models = body
            .data
            .into_iter()
            .filter(|m| m.id.starts_with("gpt") || m.id.starts_with("o1") || m.id.starts_with("o3") || m.id.starts_with("o4"))
            .map(|m| ModelInfo {
                id: m.id.clone(),
                name: m.id,
                context_window: 1048576, // GPT-4.1+ supports up to 1M tokens
                max_output_tokens: Some(32768),
            })
            .collect();

        Ok(models)
    }

    async fn chat_stream(
        &self,
        request: &ChatRequest,
        credential: &Credential,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>> {
        let token = credential
            .bearer_token()
            .ok_or_else(|| LlmError::Auth("No bearer token available".to_string()))?;

        match &self.transport {
            super::openai_ws::OpenAITransport::WebSocket => {
                let stream = super::openai_ws::ws_chat_stream(request, token, None, None).await?;
                Ok(Box::pin(stream))
            }
            super::openai_ws::OpenAITransport::Auto => {
                // Try WebSocket first, fall back to SSE
                match super::openai_ws::ws_chat_stream(request, token, None, None).await {
                    Ok(stream) => {
                        tracing::debug!("Using OpenAI WebSocket transport");
                        Ok(Box::pin(stream))
                    }
                    Err(e) => {
                        tracing::info!("WebSocket transport failed, falling back to SSE: {}", e);
                        self.chat_stream_sse(request, token).await
                    }
                }
            }
            super::openai_ws::OpenAITransport::Sse => {
                self.chat_stream_sse(request, token).await
            }
        }
    }

    fn count_tokens(&self, messages: &[ChatMessage]) -> Result<usize> {
        // Use tiktoken-rs for accurate OpenAI token counting.
        // Default to cl100k_base (GPT-4 / GPT-3.5-turbo family).
        let bpe = tiktoken_rs::cl100k_base()
            .map_err(|e| LlmError::Other(format!("Failed to load tokenizer: {}", e)))?;

        let mut total = 0;
        for msg in messages {
            // Per OpenAI's token counting rules:
            // every message follows <|start|>{role/name}\n{content}<|end|>\n
            total += 4; // overhead per message
            total += bpe.encode_with_special_tokens(&msg.content).len();
            total += bpe.encode_with_special_tokens(&format!("{:?}", msg.role)).len();
        }
        total += 2; // reply priming

        Ok(total)
    }

    fn auth_methods(&self) -> Vec<AuthMethod> {
        vec![AuthMethod::ApiKey {
            env_var_hint: Some("OPENAI_API_KEY".to_string()),
        }]
    }
}

/// Parse an SSE response from OpenAI into a stream of ChatChunks.
pub(crate) fn parse_openai_sse_stream(
    resp: reqwest::Response,
) -> impl Stream<Item = Result<ChatChunk>> + Send {
    

    let byte_stream = resp.bytes_stream();

    futures::stream::unfold(
        (byte_stream, String::new()),
        |(mut byte_stream, mut buffer)| async move {
            use futures::TryStreamExt;

            loop {
                // Check if we have a complete SSE event in the buffer
                if let Some(pos) = buffer.find("\n\n") {
                    let event = buffer[..pos].to_string();
                    buffer = buffer[pos + 2..].to_string();

                    // Parse the SSE event
                    for line in event.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data.trim() == "[DONE]" {
                                return Some((Ok(ChatChunk::Done), (byte_stream, buffer)));
                            }

                            match serde_json::from_str::<OpenAIStreamChunk>(data) {
                                Ok(chunk) => {
                                    if let Some(chat_chunk) = convert_openai_chunk(chunk) {
                                        return Some((
                                            Ok(chat_chunk),
                                            (byte_stream, buffer),
                                        ));
                                    }
                                }
                                Err(e) => {
                                    return Some((
                                        Err(LlmError::Stream(format!(
                                            "Failed to parse SSE chunk: {}",
                                            e
                                        ))),
                                        (byte_stream, buffer),
                                    ));
                                }
                            }
                        }
                    }
                    continue;
                }

                // Need more data
                match byte_stream.try_next().await {
                    Ok(Some(bytes)) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                    }
                    Ok(None) => return None, // Stream ended
                    Err(e) => {
                        return Some((
                            Err(LlmError::Stream(format!("SSE read error: {}", e))),
                            (byte_stream, buffer),
                        ));
                    }
                }
            }
        },
    )
}

/// Convert an OpenAI streaming chunk to our ChatChunk type.
fn convert_openai_chunk(chunk: OpenAIStreamChunk) -> Option<ChatChunk> {
    if let Some(usage) = chunk.usage {
        return Some(ChatChunk::Usage {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
        });
    }

    let choice = chunk.choices.into_iter().next()?;
    let delta = choice.delta;

    // Check for tool calls
    if let Some(tool_calls) = delta.tool_calls {
        if let Some(tc) = tool_calls.into_iter().next() {
            return Some(ChatChunk::ToolCallDelta {
                index: tc.index,
                id: tc.id,
                name: tc.function.as_ref().and_then(|f| f.name.clone()),
                arguments_delta: tc
                    .function
                    .as_ref()
                    .and_then(|f| f.arguments.clone())
                    .unwrap_or_default(),
            });
        }
    }

    // Check for text content
    if let Some(content) = delta.content {
        if !content.is_empty() {
            return Some(ChatChunk::TextDelta(content));
        }
    }

    None
}

// ---- OpenAI API response types ----

#[derive(Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
}

#[derive(Deserialize)]
struct OpenAIModel {
    id: String,
}

#[derive(Deserialize)]
struct OpenAIStreamChunk {
    #[serde(default)]
    choices: Vec<OpenAIStreamChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIDelta,
}

#[derive(Deserialize)]
struct OpenAIDelta {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAIToolCallDelta>>,
}

#[derive(Deserialize)]
struct OpenAIToolCallDelta {
    index: usize,
    id: Option<String>,
    function: Option<OpenAIFunctionDelta>,
}

#[derive(Deserialize)]
struct OpenAIFunctionDelta {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolSchema;

    #[test]
    fn test_openai_provider_id() {
        let provider = OpenAIProvider::new(None);
        assert_eq!(provider.id(), "openai");
        assert_eq!(provider.display_name(), "OpenAI");
    }

    #[test]
    fn test_openai_custom_endpoint() {
        let provider = OpenAIProvider::new(Some("https://custom.api.com/v1/"));
        assert_eq!(provider.endpoint, "https://custom.api.com/v1");
    }

    #[test]
    fn test_openai_auth_methods() {
        let provider = OpenAIProvider::new(None);
        let methods = provider.auth_methods();
        assert_eq!(methods.len(), 1);
        match &methods[0] {
            AuthMethod::ApiKey { env_var_hint } => {
                assert_eq!(env_var_hint.as_deref(), Some("OPENAI_API_KEY"));
            }
            _ => panic!("Expected ApiKey auth method"),
        }
    }

    #[test]
    fn test_count_tokens_basic() {
        let provider = OpenAIProvider::new(None);
        let messages = vec![
            ChatMessage::system("You are a helpful assistant."),
            ChatMessage::user("Hello!"),
        ];
        let count = provider.count_tokens(&messages).unwrap();
        assert!(count > 0, "Token count should be positive");
        assert!(count < 100, "Token count should be reasonable");
    }

    #[test]
    fn test_build_api_body() {
        let provider = OpenAIProvider::new(None);
        let request = ChatRequest {
            model: "gpt-4.1".to_string(),
            messages: vec![ChatMessage::user("Hello")],
            tools: vec![ToolSchema {
                name: "get_weather".to_string(),
                description: "Get weather".to_string(),
                parameters: serde_json::json!({"type": "object", "properties": {}}),
                required_permission: None,
            }],
            max_tokens: Some(1000),
            temperature: Some(0.7),
            stream: true,
            thinking: None,
            effort: None,
        };

        let body = provider.build_api_body(&request);
        assert_eq!(body["model"], "gpt-4.1");
        assert_eq!(body["stream"], true);
        assert_eq!(body["max_completion_tokens"], 1000);
        assert!((body["temperature"].as_f64().unwrap() - 0.7).abs() < 0.01);
        assert!(body["tools"].is_array());
        assert_eq!(body["tools"].as_array().unwrap().len(), 1);
        assert_eq!(body["tools"][0]["function"]["name"], "get_weather");
    }

    #[test]
    fn test_build_api_body_minimal() {
        let provider = OpenAIProvider::new(None);
        let request = ChatRequest {
            model: "gpt-4.1".to_string(),
            messages: vec![ChatMessage::user("Hi")],
            tools: vec![],
            max_tokens: None,
            temperature: None,
            stream: true,
            thinking: None,
            effort: None,
        };

        let body = provider.build_api_body(&request);
        assert_eq!(body["model"], "gpt-4.1");
        assert!(body.get("max_completion_tokens").is_none());
        assert!(body.get("temperature").is_none());
        assert!(body.get("tools").is_none());
    }

    #[test]
    fn test_build_api_body_with_tool_calls() {
        let provider = OpenAIProvider::new(None);
        let tc = crate::types::ToolCall {
            id: "call_123".to_string(),
            name: "get_weather".to_string(),
            arguments: "{\"city\":\"London\"}".to_string(),
        };
        let request = ChatRequest {
            model: "gpt-4.1".to_string(),
            messages: vec![
                ChatMessage::user("What's the weather?"),
                ChatMessage::assistant_with_tool_calls("", vec![tc]),
                ChatMessage::tool_result("call_123", "Sunny, 20C"),
            ],
            tools: vec![],
            max_tokens: None,
            temperature: None,
            stream: true,
            thinking: None,
            effort: None,
        };

        let body = provider.build_api_body(&request);
        let msgs = body["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 3);
        assert!(msgs[1]["tool_calls"].is_array());
        assert_eq!(msgs[2]["tool_call_id"], "call_123");
    }

    #[test]
    fn test_convert_openai_chunk_text() {
        let chunk = OpenAIStreamChunk {
            choices: vec![OpenAIStreamChoice {
                delta: OpenAIDelta {
                    content: Some("Hello".to_string()),
                    tool_calls: None,
                },
            }],
            usage: None,
        };
        let result = convert_openai_chunk(chunk);
        assert!(matches!(result, Some(ChatChunk::TextDelta(ref s)) if s == "Hello"));
    }

    #[test]
    fn test_convert_openai_chunk_tool_call() {
        let chunk = OpenAIStreamChunk {
            choices: vec![OpenAIStreamChoice {
                delta: OpenAIDelta {
                    content: None,
                    tool_calls: Some(vec![OpenAIToolCallDelta {
                        index: 0,
                        id: Some("call_1".to_string()),
                        function: Some(OpenAIFunctionDelta {
                            name: Some("get_weather".to_string()),
                            arguments: Some("{".to_string()),
                        }),
                    }]),
                },
            }],
            usage: None,
        };
        let result = convert_openai_chunk(chunk);
        match result {
            Some(ChatChunk::ToolCallDelta {
                index,
                id,
                name,
                arguments_delta,
            }) => {
                assert_eq!(index, 0);
                assert_eq!(id, Some("call_1".to_string()));
                assert_eq!(name, Some("get_weather".to_string()));
                assert_eq!(arguments_delta, "{");
            }
            _ => panic!("Expected ToolCallDelta"),
        }
    }

    #[test]
    fn test_convert_openai_chunk_usage() {
        let chunk = OpenAIStreamChunk {
            choices: vec![],
            usage: Some(OpenAIUsage {
                prompt_tokens: 100,
                completion_tokens: 50,
                total_tokens: 150,
            }),
        };
        let result = convert_openai_chunk(chunk);
        match result {
            Some(ChatChunk::Usage {
                prompt_tokens,
                completion_tokens,
                total_tokens,
            }) => {
                assert_eq!(prompt_tokens, 100);
                assert_eq!(completion_tokens, 50);
                assert_eq!(total_tokens, 150);
            }
            _ => panic!("Expected Usage"),
        }
    }

    #[test]
    fn test_convert_openai_chunk_empty() {
        let chunk = OpenAIStreamChunk {
            choices: vec![OpenAIStreamChoice {
                delta: OpenAIDelta {
                    content: Some(String::new()),
                    tool_calls: None,
                },
            }],
            usage: None,
        };
        let result = convert_openai_chunk(chunk);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_sse_data() {
        // Test the SSE parsing logic with a simulated chunk
        let data = r#"{"id":"chatcmpl-123","choices":[{"delta":{"content":"Hi"}}]}"#;
        let chunk: OpenAIStreamChunk = serde_json::from_str(data).unwrap();
        let result = convert_openai_chunk(chunk);
        assert!(matches!(result, Some(ChatChunk::TextDelta(ref s)) if s == "Hi"));
    }

    #[test]
    fn test_openai_with_transport() {
        use super::super::openai_ws::OpenAITransport;

        let provider = OpenAIProvider::with_transport(None, OpenAITransport::Auto);
        assert_eq!(provider.id(), "openai");
        assert!(matches!(provider.transport, OpenAITransport::Auto));

        let ws_provider = OpenAIProvider::with_transport(
            Some("https://custom.api.com/v1"),
            OpenAITransport::WebSocket,
        );
        assert_eq!(ws_provider.endpoint, "https://custom.api.com/v1");
        assert!(matches!(ws_provider.transport, OpenAITransport::WebSocket));

        let sse_provider = OpenAIProvider::new(None);
        assert!(matches!(sse_provider.transport, OpenAITransport::Sse));
    }
}
