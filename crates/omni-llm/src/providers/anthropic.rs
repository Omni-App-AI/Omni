use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde::Deserialize;

use crate::credentials::Credential;
use crate::error::{LlmError, Result};
use crate::types::{
    AuthMethod, ChatChunk, ChatMessage, ChatRequest, ChatRole, ModelInfo,
    ThinkingContent, ThinkingMode,
};

use super::LLMProvider;

const DEFAULT_ENDPOINT: &str = "https://api.anthropic.com/v1";
const API_VERSION: &str = "2023-06-01";

/// Anthropic provider adapter.
///
/// Supports Claude Opus, Sonnet, and Haiku model families.
pub struct AnthropicProvider {
    client: Client,
    endpoint: String,
}

impl AnthropicProvider {
    pub fn new(endpoint: Option<&str>) -> Self {
        Self {
            client: Client::new(),
            endpoint: endpoint.unwrap_or(DEFAULT_ENDPOINT).trim_end_matches('/').to_string(),
        }
    }

    fn build_api_body(&self, request: &ChatRequest) -> serde_json::Value {
        // Anthropic separates system messages from the messages array.
        let mut system_text = String::new();
        let mut messages: Vec<serde_json::Value> = Vec::new();

        for msg in &request.messages {
            match msg.role {
                ChatRole::System => {
                    if !system_text.is_empty() {
                        system_text.push('\n');
                    }
                    system_text.push_str(&msg.content);
                }
                ChatRole::User => {
                    messages.push(serde_json::json!({
                        "role": "user",
                        "content": msg.content,
                    }));
                }
                ChatRole::Assistant => {
                    let has_tool_calls = msg.tool_calls.is_some();
                    let has_thinking = msg.thinking_content.is_some();

                    if has_tool_calls || has_thinking {
                        let mut content_blocks: Vec<serde_json::Value> = Vec::new();

                        // Include thinking blocks first (for multi-turn preservation)
                        if let Some(ref thinking) = msg.thinking_content {
                            for tc in thinking {
                                match tc {
                                    ThinkingContent::Thinking(tb) => {
                                        content_blocks.push(serde_json::json!({
                                            "type": "thinking",
                                            "thinking": tb.thinking,
                                            "signature": tb.signature,
                                        }));
                                    }
                                    ThinkingContent::Redacted(rb) => {
                                        content_blocks.push(serde_json::json!({
                                            "type": "redacted_thinking",
                                            "data": rb.data,
                                        }));
                                    }
                                }
                            }
                        }

                        if !msg.content.is_empty() {
                            content_blocks.push(serde_json::json!({
                                "type": "text",
                                "text": msg.content,
                            }));
                        }

                        if let Some(ref tcs) = msg.tool_calls {
                            for tc in tcs {
                                let args: serde_json::Value =
                                    serde_json::from_str(&tc.arguments).unwrap_or_default();
                                content_blocks.push(serde_json::json!({
                                    "type": "tool_use",
                                    "id": tc.id,
                                    "name": tc.name,
                                    "input": args,
                                }));
                            }
                        }

                        messages.push(serde_json::json!({
                            "role": "assistant",
                            "content": content_blocks,
                        }));
                    } else {
                        messages.push(serde_json::json!({
                            "role": "assistant",
                            "content": msg.content,
                        }));
                    }
                }
                ChatRole::Tool => {
                    // Anthropic expects tool results as user messages with tool_result content blocks.
                    // When images are attached, format as multimodal content array.
                    let tool_content = if let Some(ref images) = msg.images {
                        let mut blocks: Vec<serde_json::Value> = Vec::new();
                        // Add text block
                        if !msg.content.is_empty() {
                            blocks.push(serde_json::json!({
                                "type": "text",
                                "text": msg.content,
                            }));
                        }
                        // Add image blocks
                        for img in images {
                            blocks.push(serde_json::json!({
                                "type": "image",
                                "source": {
                                    "type": "base64",
                                    "media_type": img.mime_type,
                                    "data": img.data,
                                },
                            }));
                        }
                        serde_json::json!(blocks)
                    } else {
                        serde_json::json!(msg.content)
                    };

                    messages.push(serde_json::json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": msg.tool_call_id.as_deref().unwrap_or(""),
                            "content": tool_content,
                        }],
                    }));
                }
            }
        }

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "stream": request.stream,
        });

        if !system_text.is_empty() {
            body["system"] = serde_json::json!(system_text);
        }

        // Add thinking configuration
        if let Some(ref thinking) = request.thinking {
            match thinking {
                ThinkingMode::Adaptive => {
                    body["thinking"] = serde_json::json!({"type": "adaptive"});
                }
                ThinkingMode::Enabled { budget_tokens } => {
                    body["thinking"] = serde_json::json!({
                        "type": "enabled",
                        "budget_tokens": budget_tokens,
                    });
                }
            }
            // When thinking is enabled, we omit temperature entirely.
            // Anthropic defaults to 1.0 and rejects any other value.
        } else if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        // Add effort level for adaptive thinking
        if let Some(ref effort) = request.effort {
            let effort_str = match effort {
                crate::types::ThinkingEffort::Low => "low",
                crate::types::ThinkingEffort::Medium => "medium",
                crate::types::ThinkingEffort::High => "high",
                crate::types::ThinkingEffort::Max => "max",
            };
            body["output_config"] = serde_json::json!({"effort": effort_str});
        }

        if !request.tools.is_empty() {
            let tools: Vec<serde_json::Value> = request
                .tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.parameters,
                    })
                })
                .collect();
            body["tools"] = serde_json::json!(tools);
        }

        body
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    fn id(&self) -> &str {
        "anthropic"
    }

    fn display_name(&self) -> &str {
        "Anthropic"
    }

    async fn list_models(&self, _credential: &Credential) -> Result<Vec<ModelInfo>> {
        // Anthropic doesn't have a /models endpoint; return known models.
        Ok(vec![
            ModelInfo {
                id: "claude-opus-4-6".to_string(),
                name: "Claude Opus 4.6".to_string(),
                context_window: 200000,
                max_output_tokens: Some(128000),
            },
            ModelInfo {
                id: "claude-sonnet-4-6".to_string(),
                name: "Claude Sonnet 4.6".to_string(),
                context_window: 200000,
                max_output_tokens: Some(64000),
            },
            ModelInfo {
                id: "claude-haiku-4-5-20251001".to_string(),
                name: "Claude Haiku 4.5".to_string(),
                context_window: 200000,
                max_output_tokens: Some(64000),
            },
        ])
    }

    async fn chat_stream(
        &self,
        request: &ChatRequest,
        credential: &Credential,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>> {
        let token = credential
            .bearer_token()
            .ok_or_else(|| LlmError::Auth("No API key available".to_string()))?;

        let body = self.build_api_body(request);

        let resp = self
            .client
            .post(format!("{}/messages", self.endpoint))
            .header("x-api-key", token)
            .header("anthropic-version", API_VERSION)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::Provider(format!(
                "Anthropic chat failed ({}): {}",
                status, body
            )));
        }

        let stream = parse_anthropic_sse_stream(resp);
        Ok(Box::pin(stream))
    }

    fn count_tokens(&self, messages: &[ChatMessage]) -> Result<usize> {
        // Anthropic uses a similar tokenizer to Claude (based on BPE).
        // Use cl100k_base as a reasonable approximation.
        let bpe = tiktoken_rs::cl100k_base()
            .map_err(|e| LlmError::Other(format!("Failed to load tokenizer: {}", e)))?;

        let mut total = 0;
        for msg in messages {
            total += 4; // overhead per message
            total += bpe.encode_with_special_tokens(&msg.content).len();
        }
        total += 2; // reply priming
        Ok(total)
    }

    fn auth_methods(&self) -> Vec<AuthMethod> {
        vec![
            AuthMethod::ApiKey {
                env_var_hint: Some("ANTHROPIC_API_KEY".to_string()),
            },
            AuthMethod::OAuth {
                authorize_url: "https://console.anthropic.com/oauth/authorize".to_string(),
                token_url: "https://console.anthropic.com/oauth/token".to_string(),
                scopes: vec!["api".to_string()],
            },
        ]
    }
}

/// Parse Anthropic SSE stream into ChatChunks.
fn parse_anthropic_sse_stream(
    resp: reqwest::Response,
) -> impl Stream<Item = Result<ChatChunk>> + Send {
    

    let byte_stream = resp.bytes_stream();

    futures::stream::unfold(
        (byte_stream, String::new(), AnthropicStreamState::default()),
        |(mut byte_stream, mut buffer, mut state)| async move {
            use futures::TryStreamExt;

            loop {
                // Try to extract a complete SSE event
                if let Some(pos) = buffer.find("\n\n") {
                    let event_block = buffer[..pos].to_string();
                    buffer = buffer[pos + 2..].to_string();

                    let mut event_type = String::new();
                    let mut event_data = String::new();

                    for line in event_block.lines() {
                        if let Some(et) = line.strip_prefix("event: ") {
                            event_type = et.to_string();
                        } else if let Some(d) = line.strip_prefix("data: ") {
                            event_data = d.to_string();
                        }
                    }

                    if let Some(chunk) =
                        convert_anthropic_event(&event_type, &event_data, &mut state)
                    {
                        return Some((chunk, (byte_stream, buffer, state)));
                    }
                    continue;
                }

                // Need more data
                match byte_stream.try_next().await {
                    Ok(Some(bytes)) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                    }
                    Ok(None) => return None,
                    Err(e) => {
                        return Some((
                            Err(LlmError::Stream(format!("SSE read error: {}", e))),
                            (byte_stream, buffer, state),
                        ));
                    }
                }
            }
        },
    )
}

#[derive(Default)]
struct AnthropicStreamState {
    current_tool_index: usize,
    /// Whether we're currently inside a thinking content block.
    in_thinking_block: bool,
}

fn convert_anthropic_event(
    event_type: &str,
    data: &str,
    state: &mut AnthropicStreamState,
) -> Option<Result<ChatChunk>> {
    match event_type {
        "content_block_delta" => {
            let parsed: std::result::Result<AnthropicContentBlockDelta, _> =
                serde_json::from_str(data);
            match parsed {
                Ok(delta) => match delta.delta.r#type.as_str() {
                    "text_delta" => Some(Ok(ChatChunk::TextDelta(
                        delta.delta.text.unwrap_or_default(),
                    ))),
                    "thinking_delta" => Some(Ok(ChatChunk::ThinkingDelta(
                        delta.delta.thinking.unwrap_or_default(),
                    ))),
                    "input_json_delta" => Some(Ok(ChatChunk::ToolCallDelta {
                        index: state.current_tool_index,
                        id: None,
                        name: None,
                        arguments_delta: delta.delta.partial_json.unwrap_or_default(),
                    })),
                    "signature_delta" => {
                        // Emit signature deltas so the agent loop can accumulate them
                        // and attach to thinking blocks for multi-turn preservation.
                        Some(Ok(ChatChunk::SignatureDelta(
                            delta.delta.signature.unwrap_or_default(),
                        )))
                    }
                    _ => None,
                },
                Err(e) => Some(Err(LlmError::Stream(format!("Parse error: {}", e)))),
            }
        }
        "content_block_start" => {
            let parsed: std::result::Result<AnthropicContentBlockStart, _> =
                serde_json::from_str(data);
            match parsed {
                Ok(start) => {
                    if start.content_block.r#type == "tool_use" {
                        state.in_thinking_block = false;
                        state.current_tool_index = start.index;
                        Some(Ok(ChatChunk::ToolCallDelta {
                            index: start.index,
                            id: Some(start.content_block.id.unwrap_or_default()),
                            name: Some(start.content_block.name.unwrap_or_default()),
                            arguments_delta: String::new(),
                        }))
                    } else if start.content_block.r#type == "thinking" {
                        state.in_thinking_block = true;
                        // Emit an empty thinking delta to signal the start of a thinking block
                        Some(Ok(ChatChunk::ThinkingDelta(String::new())))
                    } else {
                        state.in_thinking_block = false;
                        None
                    }
                }
                Err(e) => Some(Err(LlmError::Stream(format!("Parse error: {}", e)))),
            }
        }
        "message_delta" => {
            let parsed: std::result::Result<AnthropicMessageDelta, _> =
                serde_json::from_str(data);
            match parsed {
                Ok(md) => {
                    md.usage.map(|usage| Ok(ChatChunk::Usage {
                        prompt_tokens: 0, // Anthropic sends input tokens in message_start
                        completion_tokens: usage.output_tokens,
                        total_tokens: usage.output_tokens,
                    }))
                }
                Err(_) => None,
            }
        }
        "message_stop" => Some(Ok(ChatChunk::Done)),
        _ => None,
    }
}

// ---- Anthropic API response types ----

#[derive(Deserialize)]
struct AnthropicContentBlockDelta {
    delta: AnthropicDelta,
}

#[derive(Deserialize)]
struct AnthropicDelta {
    r#type: String,
    text: Option<String>,
    partial_json: Option<String>,
    thinking: Option<String>,
    signature: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicContentBlockStart {
    index: usize,
    content_block: AnthropicContentBlock,
}

#[derive(Deserialize)]
struct AnthropicContentBlock {
    r#type: String,
    id: Option<String>,
    name: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicMessageDelta {
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    output_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolSchema;

    #[test]
    fn test_anthropic_provider_id() {
        let provider = AnthropicProvider::new(None);
        assert_eq!(provider.id(), "anthropic");
        assert_eq!(provider.display_name(), "Anthropic");
    }

    #[test]
    fn test_anthropic_custom_endpoint() {
        let provider = AnthropicProvider::new(Some("https://custom.anthropic.com/v1/"));
        assert_eq!(provider.endpoint, "https://custom.anthropic.com/v1");
    }

    #[test]
    fn test_anthropic_auth_methods() {
        let provider = AnthropicProvider::new(None);
        let methods = provider.auth_methods();
        assert_eq!(methods.len(), 2);
        assert!(matches!(&methods[0], AuthMethod::ApiKey { .. }));
        assert!(matches!(&methods[1], AuthMethod::OAuth { .. }));
    }

    #[test]
    fn test_anthropic_build_api_body_system_separate() {
        let provider = AnthropicProvider::new(None);
        let request = ChatRequest {
            model: "claude-sonnet-4-6".to_string(),
            messages: vec![
                ChatMessage::system("You are helpful."),
                ChatMessage::user("Hello"),
            ],
            tools: vec![],
            max_tokens: Some(2048),
            temperature: Some(0.5),
            stream: true,
            thinking: None,
            effort: None,
        };

        let body = provider.build_api_body(&request);
        assert_eq!(body["model"], "claude-sonnet-4-6");
        assert_eq!(body["system"], "You are helpful.");
        assert_eq!(body["max_tokens"], 2048);
        assert!((body["temperature"].as_f64().unwrap() - 0.5).abs() < 0.01);

        let msgs = body["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 1); // system not in messages
        assert_eq!(msgs[0]["role"], "user");
    }

    #[test]
    fn test_anthropic_build_api_body_tool_use() {
        let provider = AnthropicProvider::new(None);
        let tc = crate::types::ToolCall {
            id: "toolu_123".to_string(),
            name: "get_weather".to_string(),
            arguments: "{\"city\":\"London\"}".to_string(),
        };
        let request = ChatRequest {
            model: "claude-sonnet-4-6".to_string(),
            messages: vec![
                ChatMessage::user("Weather?"),
                ChatMessage::assistant_with_tool_calls("Let me check.", vec![tc]),
                ChatMessage::tool_result("toolu_123", "Sunny"),
            ],
            tools: vec![ToolSchema {
                name: "get_weather".to_string(),
                description: "Get weather".to_string(),
                parameters: serde_json::json!({"type": "object"}),
                required_permission: None,
            }],
            max_tokens: None,
            temperature: None,
            stream: true,
            thinking: None,
            effort: None,
        };

        let body = provider.build_api_body(&request);
        let msgs = body["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 3);

        // Assistant message should have content blocks
        let asst = &msgs[1];
        assert_eq!(asst["role"], "assistant");
        let content = asst["content"].as_array().unwrap();
        assert_eq!(content.len(), 2); // text + tool_use
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[1]["type"], "tool_use");

        // Tool result is a user message with tool_result content
        let tool = &msgs[2];
        assert_eq!(tool["role"], "user");
        let tool_content = tool["content"].as_array().unwrap();
        assert_eq!(tool_content[0]["type"], "tool_result");
        assert_eq!(tool_content[0]["tool_use_id"], "toolu_123");

        // Tools in body
        let tools = body["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "get_weather");
        assert_eq!(tools[0]["input_schema"]["type"], "object");
    }

    #[test]
    fn test_anthropic_list_models() {
        let provider = AnthropicProvider::new(None);
        let cred = Credential::ApiKey {
            key: "test".to_string(),
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let models = rt.block_on(provider.list_models(&cred)).unwrap();
        assert!(models.len() >= 3);
        assert!(models.iter().any(|m| m.id.contains("opus")));
        assert!(models.iter().any(|m| m.id.contains("sonnet")));
        assert!(models.iter().any(|m| m.id.contains("haiku")));
    }

    #[test]
    fn test_convert_anthropic_text_delta() {
        let mut state = AnthropicStreamState::default();
        let data = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
        let result = convert_anthropic_event("content_block_delta", data, &mut state);
        match result {
            Some(Ok(ChatChunk::TextDelta(text))) => assert_eq!(text, "Hello"),
            _ => panic!("Expected TextDelta"),
        }
    }

    #[test]
    fn test_convert_anthropic_tool_use_start() {
        let mut state = AnthropicStreamState::default();
        let data = r#"{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"toolu_1","name":"get_weather"}}"#;
        let result = convert_anthropic_event("content_block_start", data, &mut state);
        match result {
            Some(Ok(ChatChunk::ToolCallDelta {
                index,
                id,
                name,
                arguments_delta,
            })) => {
                assert_eq!(index, 1);
                assert_eq!(id, Some("toolu_1".to_string()));
                assert_eq!(name, Some("get_weather".to_string()));
                assert!(arguments_delta.is_empty());
            }
            _ => panic!("Expected ToolCallDelta"),
        }
    }

    #[test]
    fn test_convert_anthropic_message_stop() {
        let mut state = AnthropicStreamState::default();
        let result = convert_anthropic_event("message_stop", "{}", &mut state);
        assert!(matches!(result, Some(Ok(ChatChunk::Done))));
    }

    #[test]
    fn test_count_tokens() {
        let provider = AnthropicProvider::new(None);
        let messages = vec![ChatMessage::user("Hello world")];
        let count = provider.count_tokens(&messages).unwrap();
        assert!(count > 0);
    }

    #[test]
    fn test_anthropic_build_api_body_adaptive_thinking() {
        use crate::types::{ThinkingMode, ThinkingEffort};

        let provider = AnthropicProvider::new(None);
        let request = ChatRequest {
            model: "claude-opus-4-6".to_string(),
            messages: vec![ChatMessage::user("Think about this")],
            tools: vec![],
            max_tokens: Some(16000),
            temperature: Some(0.7), // Should be stripped when thinking is enabled
            stream: true,
            thinking: Some(ThinkingMode::Adaptive),
            effort: Some(ThinkingEffort::Medium),
        };

        let body = provider.build_api_body(&request);
        assert_eq!(body["thinking"]["type"], "adaptive");
        assert_eq!(body["output_config"]["effort"], "medium");
        // Temperature should NOT be set when thinking is enabled
        assert!(body.get("temperature").is_none());
    }

    #[test]
    fn test_anthropic_build_api_body_enabled_thinking() {
        use crate::types::ThinkingMode;

        let provider = AnthropicProvider::new(None);
        let request = ChatRequest {
            model: "claude-sonnet-4-6".to_string(),
            messages: vec![ChatMessage::user("Think hard")],
            tools: vec![],
            max_tokens: Some(16000),
            temperature: None,
            stream: true,
            thinking: Some(ThinkingMode::Enabled { budget_tokens: 8000 }),
            effort: None,
        };

        let body = provider.build_api_body(&request);
        assert_eq!(body["thinking"]["type"], "enabled");
        assert_eq!(body["thinking"]["budget_tokens"], 8000);
        assert!(body.get("output_config").is_none());
    }

    #[test]
    fn test_anthropic_build_api_body_no_thinking() {
        let provider = AnthropicProvider::new(None);
        let request = ChatRequest {
            model: "claude-haiku-4-5-20251001".to_string(),
            messages: vec![ChatMessage::user("Quick answer")],
            tools: vec![],
            max_tokens: Some(1024),
            temperature: Some(0.3),
            stream: true,
            thinking: None,
            effort: None,
        };

        let body = provider.build_api_body(&request);
        assert!(body.get("thinking").is_none());
        assert!((body["temperature"].as_f64().unwrap() - 0.3).abs() < 0.01);
    }

    #[test]
    fn test_convert_anthropic_thinking_delta() {
        let mut state = AnthropicStreamState::default();
        let data = r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Let me reason..."}}"#;
        let result = convert_anthropic_event("content_block_delta", data, &mut state);
        match result {
            Some(Ok(ChatChunk::ThinkingDelta(text))) => assert_eq!(text, "Let me reason..."),
            _ => panic!("Expected ThinkingDelta"),
        }
    }

    #[test]
    fn test_convert_anthropic_thinking_block_start() {
        let mut state = AnthropicStreamState::default();
        let data = r#"{"type":"content_block_start","index":0,"content_block":{"type":"thinking"}}"#;
        let result = convert_anthropic_event("content_block_start", data, &mut state);
        match result {
            Some(Ok(ChatChunk::ThinkingDelta(text))) => {
                assert!(text.is_empty()); // Start signal
                assert!(state.in_thinking_block);
            }
            _ => panic!("Expected ThinkingDelta start signal"),
        }
    }

    #[test]
    fn test_anthropic_thinking_blocks_in_assistant_message() {
        use crate::types::{ThinkingBlock, ThinkingContent, RedactedThinkingBlock};

        let provider = AnthropicProvider::new(None);
        let thinking = vec![
            ThinkingContent::Thinking(ThinkingBlock {
                thinking: "Step 1: consider...".to_string(),
                signature: "sig123".to_string(),
            }),
            ThinkingContent::Redacted(RedactedThinkingBlock {
                data: "encrypted_data".to_string(),
            }),
        ];

        let msg = ChatMessage::assistant_with_thinking("The answer is 42.", thinking);
        let request = ChatRequest {
            model: "claude-opus-4-6".to_string(),
            messages: vec![ChatMessage::user("Question"), msg],
            tools: vec![],
            max_tokens: Some(4096),
            temperature: None,
            stream: true,
            thinking: Some(ThinkingMode::Adaptive),
            effort: None,
        };

        let body = provider.build_api_body(&request);
        let msgs = body["messages"].as_array().unwrap();
        let asst = &msgs[1];
        let content = asst["content"].as_array().unwrap();

        // First block should be thinking
        assert_eq!(content[0]["type"], "thinking");
        assert_eq!(content[0]["thinking"], "Step 1: consider...");
        assert_eq!(content[0]["signature"], "sig123");

        // Second block should be redacted thinking
        assert_eq!(content[1]["type"], "redacted_thinking");
        assert_eq!(content[1]["data"], "encrypted_data");

        // Third block should be text
        assert_eq!(content[2]["type"], "text");
        assert_eq!(content[2]["text"], "The answer is 42.");
    }

    #[test]
    fn test_convert_anthropic_signature_delta() {
        let mut state = AnthropicStreamState::default();
        let data = r#"{"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"sig_abc123"}}"#;
        let result = convert_anthropic_event("content_block_delta", data, &mut state);
        match result {
            Some(Ok(ChatChunk::SignatureDelta(sig))) => assert_eq!(sig, "sig_abc123"),
            _ => panic!("Expected SignatureDelta, got {:?}", result),
        }
    }

    #[test]
    fn test_convert_anthropic_signature_delta_missing_field() {
        let mut state = AnthropicStreamState::default();
        // Missing signature field should produce empty string
        let data = r#"{"type":"content_block_delta","index":0,"delta":{"type":"signature_delta"}}"#;
        let result = convert_anthropic_event("content_block_delta", data, &mut state);
        match result {
            Some(Ok(ChatChunk::SignatureDelta(sig))) => assert!(sig.is_empty()),
            _ => panic!("Expected SignatureDelta with empty string"),
        }
    }
}
