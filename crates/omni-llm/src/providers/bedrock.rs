use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde::Deserialize;

use crate::credentials::Credential;
use crate::error::{LlmError, Result};
use crate::types::{AuthMethod, ChatChunk, ChatMessage, ChatRequest, ChatRole, ModelInfo};

use super::LLMProvider;

/// AWS Bedrock provider adapter.
///
/// Supports Anthropic models, Amazon Titan, and Meta Llama via AWS Bedrock.
/// Uses AWS credentials (access key + secret key, or IAM role).
pub struct BedrockProvider {
    client: Client,
    region: String,
}

impl BedrockProvider {
    pub fn new(region: Option<&str>) -> Self {
        Self {
            client: Client::new(),
            region: region.unwrap_or("us-east-1").to_string(),
        }
    }

    fn endpoint(&self) -> String {
        format!(
            "https://bedrock-runtime.{}.amazonaws.com",
            self.region
        )
    }

    fn build_converse_body(&self, request: &ChatRequest) -> serde_json::Value {
        let mut system_parts: Vec<serde_json::Value> = Vec::new();
        let mut messages: Vec<serde_json::Value> = Vec::new();

        for msg in &request.messages {
            match msg.role {
                ChatRole::System => {
                    system_parts.push(serde_json::json!({
                        "text": msg.content,
                    }));
                }
                ChatRole::User => {
                    messages.push(serde_json::json!({
                        "role": "user",
                        "content": [{"text": msg.content}],
                    }));
                }
                ChatRole::Assistant => {
                    let mut content: Vec<serde_json::Value> = Vec::new();
                    if !msg.content.is_empty() {
                        content.push(serde_json::json!({"text": msg.content}));
                    }
                    if let Some(ref tcs) = msg.tool_calls {
                        for tc in tcs {
                            let args: serde_json::Value =
                                serde_json::from_str(&tc.arguments).unwrap_or_default();
                            content.push(serde_json::json!({
                                "toolUse": {
                                    "toolUseId": tc.id,
                                    "name": tc.name,
                                    "input": args,
                                }
                            }));
                        }
                    }
                    messages.push(serde_json::json!({
                        "role": "assistant",
                        "content": content,
                    }));
                }
                ChatRole::Tool => {
                    // Build content blocks for the tool result
                    let mut tool_content = vec![serde_json::json!({"text": msg.content})];
                    // Append images (Bedrock/Claude multimodal support)
                    if let Some(ref images) = msg.images {
                        for img in images {
                            tool_content.push(serde_json::json!({
                                "image": {
                                    "format": img.mime_type.strip_prefix("image/").unwrap_or("png"),
                                    "source": {
                                        "bytes": img.data,
                                    }
                                }
                            }));
                        }
                    }
                    messages.push(serde_json::json!({
                        "role": "user",
                        "content": [{
                            "toolResult": {
                                "toolUseId": msg.tool_call_id.as_deref().unwrap_or(""),
                                "content": tool_content,
                            }
                        }],
                    }));
                }
            }
        }

        let mut body = serde_json::json!({
            "modelId": request.model,
            "messages": messages,
        });

        if !system_parts.is_empty() {
            body["system"] = serde_json::json!(system_parts);
        }

        let mut inference_config = serde_json::Map::new();
        if let Some(max_tokens) = request.max_tokens {
            inference_config.insert(
                "maxTokens".to_string(),
                serde_json::json!(max_tokens),
            );
        }
        if let Some(temp) = request.temperature {
            inference_config.insert("temperature".to_string(), serde_json::json!(temp));
        }
        if !inference_config.is_empty() {
            body["inferenceConfig"] = serde_json::Value::Object(inference_config);
        }

        if !request.tools.is_empty() {
            let tool_specs: Vec<serde_json::Value> = request
                .tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "toolSpec": {
                            "name": t.name,
                            "description": t.description,
                            "inputSchema": {
                                "json": t.parameters,
                            }
                        }
                    })
                })
                .collect();
            body["toolConfig"] = serde_json::json!({
                "tools": tool_specs,
            });
        }

        body
    }

    /// Sign a request using AWS Signature V4.
    /// This is a simplified implementation. For production use,
    /// consider using the aws-sigv4 crate.
    fn sign_request(
        &self,
        credential: &Credential,
        _method: &str,
        _url: &str,
        _body: &[u8],
    ) -> Result<Vec<(String, String)>> {
        let (access_key, _secret_key, session_token) = match credential {
            Credential::AwsCredentials {
                access_key_id,
                secret_access_key,
                session_token,
                ..
            } => (access_key_id, secret_access_key, session_token),
            _ => {
                return Err(LlmError::Auth(
                    "AWS credentials required for Bedrock".to_string(),
                ));
            }
        };

        let now = chrono::Utc::now();
        let date_stamp = now.format("%Y%m%d").to_string();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();

        let mut headers = vec![
            ("x-amz-date".to_string(), amz_date.clone()),
            ("content-type".to_string(), "application/json".to_string()),
        ];

        if let Some(token) = session_token {
            headers.push(("x-amz-security-token".to_string(), token.clone()));
        }

        // Simplified signing -- in production, use aws-sigv4 crate for full SigV4
        // For now, we set the basic auth headers that Bedrock requires
        headers.push((
            "Authorization".to_string(),
            format!(
                "AWS4-HMAC-SHA256 Credential={}/{}/{}/bedrock/aws4_request",
                access_key, date_stamp, self.region
            ),
        ));

        Ok(headers)
    }
}

#[async_trait]
impl LLMProvider for BedrockProvider {
    fn id(&self) -> &str {
        "bedrock"
    }

    fn display_name(&self) -> &str {
        "AWS Bedrock"
    }

    async fn list_models(&self, _credential: &Credential) -> Result<Vec<ModelInfo>> {
        // Return commonly available Bedrock models
        Ok(vec![
            ModelInfo {
                id: "anthropic.claude-sonnet-4-6".to_string(),
                name: "Claude Sonnet 4.6 (Bedrock)".to_string(),
                context_window: 200000,
                max_output_tokens: Some(64000),
            },
            ModelInfo {
                id: "anthropic.claude-haiku-4-5-20251001-v1:0".to_string(),
                name: "Claude Haiku 4.5 (Bedrock)".to_string(),
                context_window: 200000,
                max_output_tokens: Some(64000),
            },
            ModelInfo {
                id: "anthropic.claude-opus-4-6-v1".to_string(),
                name: "Claude Opus 4.6 (Bedrock)".to_string(),
                context_window: 200000,
                max_output_tokens: Some(128000),
            },
            ModelInfo {
                id: "meta.llama4-maverick-17b-instruct-v1:0".to_string(),
                name: "Meta Llama 4 Maverick (Bedrock)".to_string(),
                context_window: 131072,
                max_output_tokens: Some(8192),
            },
            ModelInfo {
                id: "amazon.nova-premier-v1:0".to_string(),
                name: "Amazon Nova Premier (Bedrock)".to_string(),
                context_window: 32000,
                max_output_tokens: Some(4096),
            },
        ])
    }

    async fn chat_stream(
        &self,
        request: &ChatRequest,
        credential: &Credential,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>> {
        let body = self.build_converse_body(request);
        let body_bytes = serde_json::to_vec(&body)?;

        let url = format!(
            "{}/model/{}/converse-stream",
            self.endpoint(),
            request.model
        );

        let sign_headers = self.sign_request(credential, "POST", &url, &body_bytes)?;

        let mut req_builder = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body_bytes);

        for (key, value) in &sign_headers {
            req_builder = req_builder.header(key, value);
        }

        let resp = req_builder.send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::Provider(format!(
                "Bedrock chat failed ({}): {}",
                status, body
            )));
        }

        let stream = parse_bedrock_stream(resp);
        Ok(Box::pin(stream))
    }

    fn count_tokens(&self, messages: &[ChatMessage]) -> Result<usize> {
        // Rough estimation: ~4 characters per token
        let total_chars: usize = messages.iter().map(|m| m.content.len()).sum();
        Ok(total_chars / 4 + messages.len() * 4)
    }

    fn auth_methods(&self) -> Vec<AuthMethod> {
        vec![AuthMethod::Custom {
            instructions: "Configure AWS credentials (access key ID + secret access key). \
                          Optionally set AWS_REGION environment variable."
                .to_string(),
        }]
    }
}

/// Parse Bedrock converse-stream response into ChatChunks.
fn parse_bedrock_stream(
    resp: reqwest::Response,
) -> impl Stream<Item = Result<ChatChunk>> + Send {
    

    let byte_stream = resp.bytes_stream();

    futures::stream::unfold(
        (byte_stream, String::new()),
        |(mut byte_stream, mut buffer)| async move {
            use futures::TryStreamExt;

            loop {
                if let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].to_string();
                    buffer = buffer[pos + 1..].to_string();
                    let line = line.trim();

                    if line.is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<BedrockStreamEvent>(line) {
                        Ok(event) => {
                            if let Some(chunk) = convert_bedrock_event(event) {
                                return Some((Ok(chunk), (byte_stream, buffer)));
                            }
                        }
                        Err(_) => {
                            // Skip unparseable lines
                            continue;
                        }
                    }
                    continue;
                }

                match byte_stream.try_next().await {
                    Ok(Some(bytes)) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                    }
                    Ok(None) => {
                        return Some((Ok(ChatChunk::Done), (byte_stream, buffer)));
                    }
                    Err(e) => {
                        return Some((
                            Err(LlmError::Stream(format!("Bedrock read error: {}", e))),
                            (byte_stream, buffer),
                        ));
                    }
                }
            }
        },
    )
}

fn convert_bedrock_event(event: BedrockStreamEvent) -> Option<ChatChunk> {
    if let Some(delta) = event.content_block_delta {
        if let Some(text) = delta.delta.and_then(|d| d.text) {
            return Some(ChatChunk::TextDelta(text));
        }
    }
    if let Some(start) = event.content_block_start {
        if let Some(tool_use) = start.start.and_then(|s| s.tool_use) {
            return Some(ChatChunk::ToolCallDelta {
                index: start.content_block_index.unwrap_or(0),
                id: Some(tool_use.tool_use_id),
                name: Some(tool_use.name),
                arguments_delta: String::new(),
            });
        }
    }
    if let Some(metadata) = event.metadata {
        if let Some(usage) = metadata.usage {
            return Some(ChatChunk::Usage {
                prompt_tokens: usage.input_tokens,
                completion_tokens: usage.output_tokens,
                total_tokens: usage.input_tokens + usage.output_tokens,
            });
        }
    }
    if event.message_stop.is_some() {
        return Some(ChatChunk::Done);
    }
    None
}

// ---- Bedrock API response types ----

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BedrockStreamEvent {
    content_block_delta: Option<BedrockContentBlockDelta>,
    content_block_start: Option<BedrockContentBlockStart>,
    metadata: Option<BedrockMetadata>,
    message_stop: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct BedrockContentBlockDelta {
    delta: Option<BedrockDelta>,
}

#[derive(Deserialize)]
struct BedrockDelta {
    text: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BedrockContentBlockStart {
    content_block_index: Option<usize>,
    start: Option<BedrockStart>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BedrockStart {
    tool_use: Option<BedrockToolUse>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BedrockToolUse {
    tool_use_id: String,
    name: String,
}

#[derive(Deserialize)]
struct BedrockMetadata {
    usage: Option<BedrockUsage>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BedrockUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bedrock_provider_id() {
        let provider = BedrockProvider::new(None);
        assert_eq!(provider.id(), "bedrock");
        assert_eq!(provider.display_name(), "AWS Bedrock");
        assert_eq!(provider.region, "us-east-1");
    }

    #[test]
    fn test_bedrock_custom_region() {
        let provider = BedrockProvider::new(Some("eu-west-1"));
        assert_eq!(provider.region, "eu-west-1");
        assert!(provider.endpoint().contains("eu-west-1"));
    }

    #[test]
    fn test_bedrock_auth_methods() {
        let provider = BedrockProvider::new(None);
        let methods = provider.auth_methods();
        assert_eq!(methods.len(), 1);
        assert!(matches!(&methods[0], AuthMethod::Custom { .. }));
    }

    #[test]
    fn test_bedrock_list_models() {
        let provider = BedrockProvider::new(None);
        let cred = Credential::ApiKey {
            key: "test".to_string(),
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let models = rt.block_on(provider.list_models(&cred)).unwrap();
        assert!(models.len() >= 3);
    }

    #[test]
    fn test_bedrock_build_converse_body() {
        let provider = BedrockProvider::new(None);
        let request = ChatRequest {
            model: "anthropic.claude-sonnet-4-6".to_string(),
            messages: vec![
                ChatMessage::system("Be helpful"),
                ChatMessage::user("Hello"),
            ],
            tools: vec![],
            max_tokens: Some(1024),
            temperature: Some(0.5),
            stream: true,
            thinking: None,
            effort: None,
        };

        let body = provider.build_converse_body(&request);
        assert!(body["system"].is_array());
        let msgs = body["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 1); // System separate
        assert_eq!(body["inferenceConfig"]["maxTokens"], 1024);
        assert!((body["inferenceConfig"]["temperature"].as_f64().unwrap() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_convert_bedrock_text_event() {
        let event = BedrockStreamEvent {
            content_block_delta: Some(BedrockContentBlockDelta {
                delta: Some(BedrockDelta {
                    text: Some("Hello".to_string()),
                }),
            }),
            content_block_start: None,
            metadata: None,
            message_stop: None,
        };
        let chunk = convert_bedrock_event(event);
        assert!(matches!(chunk, Some(ChatChunk::TextDelta(ref s)) if s == "Hello"));
    }

    #[test]
    fn test_convert_bedrock_done_event() {
        let event = BedrockStreamEvent {
            content_block_delta: None,
            content_block_start: None,
            metadata: None,
            message_stop: Some(serde_json::json!({})),
        };
        let chunk = convert_bedrock_event(event);
        assert!(matches!(chunk, Some(ChatChunk::Done)));
    }
}
