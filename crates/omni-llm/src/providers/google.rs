use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde::Deserialize;

use crate::credentials::Credential;
use crate::error::{LlmError, Result};
use crate::types::{AuthMethod, ChatChunk, ChatMessage, ChatRequest, ChatRole, ModelInfo};

use super::LLMProvider;

const DEFAULT_ENDPOINT: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Google Gemini provider adapter.
///
/// Supports Gemini 2.0 Flash, Pro, Ultra models.
pub struct GoogleProvider {
    client: Client,
    endpoint: String,
}

impl GoogleProvider {
    pub fn new(endpoint: Option<&str>) -> Self {
        Self {
            client: Client::new(),
            endpoint: endpoint.unwrap_or(DEFAULT_ENDPOINT).trim_end_matches('/').to_string(),
        }
    }

    fn build_api_body(&self, request: &ChatRequest) -> serde_json::Value {
        let mut contents: Vec<serde_json::Value> = Vec::new();
        let mut system_instruction = None;

        for msg in &request.messages {
            match msg.role {
                ChatRole::System => {
                    system_instruction = Some(serde_json::json!({
                        "parts": [{"text": msg.content}]
                    }));
                }
                ChatRole::User | ChatRole::Tool => {
                    // Gemini maps both user and tool_result to "user" role
                    let mut parts = vec![serde_json::json!({"text": msg.content})];
                    if msg.role == ChatRole::Tool {
                        if let Some(ref tc_id) = msg.tool_call_id {
                            parts = vec![serde_json::json!({
                                "functionResponse": {
                                    "name": tc_id,
                                    "response": {"result": msg.content}
                                }
                            })];
                        }
                        // Append images as inline_data parts (Gemini multimodal)
                        if let Some(ref images) = msg.images {
                            for img in images {
                                parts.push(serde_json::json!({
                                    "inline_data": {
                                        "mime_type": img.mime_type,
                                        "data": img.data,
                                    }
                                }));
                            }
                        }
                    }
                    contents.push(serde_json::json!({
                        "role": "user",
                        "parts": parts,
                    }));
                }
                ChatRole::Assistant => {
                    let mut parts: Vec<serde_json::Value> = Vec::new();
                    if !msg.content.is_empty() {
                        parts.push(serde_json::json!({"text": msg.content}));
                    }
                    if let Some(ref tcs) = msg.tool_calls {
                        for tc in tcs {
                            let args: serde_json::Value =
                                serde_json::from_str(&tc.arguments).unwrap_or_default();
                            parts.push(serde_json::json!({
                                "functionCall": {
                                    "name": tc.name,
                                    "args": args,
                                }
                            }));
                        }
                    }
                    contents.push(serde_json::json!({
                        "role": "model",
                        "parts": parts,
                    }));
                }
            }
        }

        let mut body = serde_json::json!({
            "contents": contents,
        });

        if let Some(si) = system_instruction {
            body["systemInstruction"] = si;
        }

        let mut generation_config = serde_json::Map::new();
        if let Some(max_tokens) = request.max_tokens {
            generation_config.insert(
                "maxOutputTokens".to_string(),
                serde_json::json!(max_tokens),
            );
        }
        if let Some(temp) = request.temperature {
            generation_config.insert("temperature".to_string(), serde_json::json!(temp));
        }
        if !generation_config.is_empty() {
            body["generationConfig"] = serde_json::Value::Object(generation_config);
        }

        if !request.tools.is_empty() {
            let function_declarations: Vec<serde_json::Value> = request
                .tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    })
                })
                .collect();
            body["tools"] = serde_json::json!([{
                "functionDeclarations": function_declarations,
            }]);
        }

        body
    }
}

#[async_trait]
impl LLMProvider for GoogleProvider {
    fn id(&self) -> &str {
        "google"
    }

    fn display_name(&self) -> &str {
        "Google Gemini"
    }

    async fn list_models(&self, _credential: &Credential) -> Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo {
                id: "gemini-2.5-flash".to_string(),
                name: "Gemini 2.5 Flash".to_string(),
                context_window: 1048576,
                max_output_tokens: Some(65536),
            },
            ModelInfo {
                id: "gemini-2.5-pro".to_string(),
                name: "Gemini 2.5 Pro".to_string(),
                context_window: 1048576,
                max_output_tokens: Some(65536),
            },
            ModelInfo {
                id: "gemini-2.5-flash-lite".to_string(),
                name: "Gemini 2.5 Flash-Lite".to_string(),
                context_window: 1048576,
                max_output_tokens: Some(65536),
            },
        ])
    }

    async fn chat_stream(
        &self,
        request: &ChatRequest,
        credential: &Credential,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>> {
        let api_key = credential
            .api_key()
            .ok_or_else(|| LlmError::Auth("No API key available".to_string()))?;

        let body = self.build_api_body(request);

        let url = format!(
            "{}/models/{}:streamGenerateContent?alt=sse&key={}",
            self.endpoint, request.model, api_key
        );

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::Provider(format!(
                "Google Gemini chat failed ({}): {}",
                status, body
            )));
        }

        let stream = parse_gemini_sse_stream(resp);
        Ok(Box::pin(stream))
    }

    fn count_tokens(&self, messages: &[ChatMessage]) -> Result<usize> {
        // Rough estimation for Gemini: ~4 characters per token
        let total_chars: usize = messages.iter().map(|m| m.content.len()).sum();
        Ok(total_chars / 4 + messages.len() * 4)
    }

    fn auth_methods(&self) -> Vec<AuthMethod> {
        vec![
            AuthMethod::ApiKey {
                env_var_hint: Some("GOOGLE_API_KEY".to_string()),
            },
            AuthMethod::OAuth {
                authorize_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
                token_url: "https://oauth2.googleapis.com/token".to_string(),
                scopes: vec!["https://www.googleapis.com/auth/generative-language".to_string()],
            },
        ]
    }
}

/// Parse Google Gemini SSE stream into ChatChunks.
fn parse_gemini_sse_stream(
    resp: reqwest::Response,
) -> impl Stream<Item = Result<ChatChunk>> + Send {
    

    let byte_stream = resp.bytes_stream();

    futures::stream::unfold(
        (byte_stream, String::new()),
        |(mut byte_stream, mut buffer)| async move {
            use futures::TryStreamExt;

            loop {
                if let Some(pos) = buffer.find("\n\n") {
                    let event = buffer[..pos].to_string();
                    buffer = buffer[pos + 2..].to_string();

                    for line in event.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            match serde_json::from_str::<GeminiStreamResponse>(data) {
                                Ok(resp) => {
                                    if let Some(chunk) = convert_gemini_response(resp) {
                                        return Some((Ok(chunk), (byte_stream, buffer)));
                                    }
                                }
                                Err(e) => {
                                    return Some((
                                        Err(LlmError::Stream(format!(
                                            "Gemini parse error: {}",
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

                match byte_stream.try_next().await {
                    Ok(Some(bytes)) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                    }
                    Ok(None) => {
                        // Stream ended -- emit Done
                        if !buffer.is_empty() {
                            buffer.clear();
                        }
                        return Some((Ok(ChatChunk::Done), (byte_stream, buffer)));
                    }
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

fn convert_gemini_response(resp: GeminiStreamResponse) -> Option<ChatChunk> {
    let candidate = resp.candidates.into_iter().next()?;
    let content = candidate.content?;

    for part in content.parts {
        if let Some(text) = part.text {
            return Some(ChatChunk::TextDelta(text));
        }
        if let Some(fc) = part.function_call {
            let args = serde_json::to_string(&fc.args).unwrap_or_default();
            return Some(ChatChunk::ToolCallDelta {
                index: 0,
                id: Some(format!("gemini_tc_{}", fc.name)),
                name: Some(fc.name),
                arguments_delta: args,
            });
        }
    }

    if let Some(usage) = resp.usage_metadata {
        return Some(ChatChunk::Usage {
            prompt_tokens: usage.prompt_token_count.unwrap_or(0),
            completion_tokens: usage.candidates_token_count.unwrap_or(0),
            total_tokens: usage.total_token_count.unwrap_or(0),
        });
    }

    None
}

// ---- Gemini API response types ----

#[derive(Deserialize)]
struct GeminiStreamResponse {
    #[serde(default)]
    candidates: Vec<GeminiCandidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContent>,
}

#[derive(Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Deserialize)]
struct GeminiPart {
    text: Option<String>,
    #[serde(rename = "functionCall")]
    function_call: Option<GeminiFunctionCall>,
}

#[derive(Deserialize)]
struct GeminiFunctionCall {
    name: String,
    args: serde_json::Value,
}

#[derive(Deserialize)]
struct GeminiUsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: Option<u32>,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: Option<u32>,
    #[serde(rename = "totalTokenCount")]
    total_token_count: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolSchema;

    #[test]
    fn test_google_provider_id() {
        let provider = GoogleProvider::new(None);
        assert_eq!(provider.id(), "google");
        assert_eq!(provider.display_name(), "Google Gemini");
    }

    #[test]
    fn test_google_auth_methods() {
        let provider = GoogleProvider::new(None);
        let methods = provider.auth_methods();
        assert_eq!(methods.len(), 2);
        assert!(matches!(&methods[0], AuthMethod::ApiKey { .. }));
        assert!(matches!(&methods[1], AuthMethod::OAuth { .. }));
    }

    #[test]
    fn test_google_list_models() {
        let provider = GoogleProvider::new(None);
        let cred = Credential::ApiKey {
            key: "test".to_string(),
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let models = rt.block_on(provider.list_models(&cred)).unwrap();
        assert!(models.len() >= 3);
        assert!(models.iter().any(|m| m.id.contains("flash")));
    }

    #[test]
    fn test_google_build_api_body() {
        let provider = GoogleProvider::new(None);
        let request = ChatRequest {
            model: "gemini-2.5-flash".to_string(),
            messages: vec![
                ChatMessage::system("Be helpful"),
                ChatMessage::user("Hello"),
            ],
            tools: vec![],
            max_tokens: Some(2048),
            temperature: Some(0.7),
            stream: true,
            thinking: None,
            effort: None,
        };

        let body = provider.build_api_body(&request);
        assert!(body["systemInstruction"].is_object());
        let contents = body["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 1); // Only user message
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(body["generationConfig"]["maxOutputTokens"], 2048);
        assert!((body["generationConfig"]["temperature"].as_f64().unwrap() - 0.7).abs() < 0.01);
    }

    #[test]
    fn test_google_build_api_body_with_tools() {
        let provider = GoogleProvider::new(None);
        let request = ChatRequest {
            model: "gemini-2.5-flash".to_string(),
            messages: vec![ChatMessage::user("Weather?")],
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
        let tools = body["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        let decls = tools[0]["functionDeclarations"].as_array().unwrap();
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0]["name"], "get_weather");
    }

    #[test]
    fn test_convert_gemini_text_response() {
        let resp = GeminiStreamResponse {
            candidates: vec![GeminiCandidate {
                content: Some(GeminiContent {
                    parts: vec![GeminiPart {
                        text: Some("Hello!".to_string()),
                        function_call: None,
                    }],
                }),
            }],
            usage_metadata: None,
        };
        let chunk = convert_gemini_response(resp);
        assert!(matches!(chunk, Some(ChatChunk::TextDelta(ref s)) if s == "Hello!"));
    }

    #[test]
    fn test_convert_gemini_function_call() {
        let resp = GeminiStreamResponse {
            candidates: vec![GeminiCandidate {
                content: Some(GeminiContent {
                    parts: vec![GeminiPart {
                        text: None,
                        function_call: Some(GeminiFunctionCall {
                            name: "get_weather".to_string(),
                            args: serde_json::json!({"city": "London"}),
                        }),
                    }],
                }),
            }],
            usage_metadata: None,
        };
        let chunk = convert_gemini_response(resp);
        match chunk {
            Some(ChatChunk::ToolCallDelta { name, .. }) => {
                assert_eq!(name, Some("get_weather".to_string()));
            }
            _ => panic!("Expected ToolCallDelta"),
        }
    }
}
