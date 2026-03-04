//! OpenAI Responses API WebSocket transport.
//!
//! Provides persistent WebSocket connections to `wss://api.openai.com/v1/responses`
//! for lower-latency streaming, especially in multi-tool-call agent workflows.
//! Falls back to SSE when WebSocket connection fails.

use futures::{SinkExt, Stream, StreamExt};
use serde::Deserialize;
use tokio_tungstenite::tungstenite;

use crate::error::{LlmError, Result};
use crate::types::{ChatChunk, ChatRequest, ChatRole};

const WS_ENDPOINT: &str = "wss://api.openai.com/v1/responses";

/// Maximum idle time before we consider the connection dead (60 minutes per OpenAI docs).
const WS_IDLE_TIMEOUT_SECS: u64 = 60 * 60;

/// Interval for sending WebSocket ping frames to keep the connection alive.
#[allow(dead_code)]
const WS_PING_INTERVAL_SECS: u64 = 30;

/// Transport mode for OpenAI provider.
#[derive(Debug, Clone, Default)]
pub enum OpenAITransport {
    /// Try WebSocket first, fall back to SSE on connection failure.
    Auto,
    /// WebSocket only (fails if WS connection cannot be established).
    WebSocket,
    /// SSE only (current default behavior for backward compatibility).
    #[default]
    Sse,
}

/// Connect to the OpenAI Responses API via WebSocket and stream ChatChunks.
///
/// Includes keepalive pings and a 60-minute idle timeout per OpenAI's session limits.
/// The optional `previous_response_id` enables conversation continuity across
/// reconnections and multi-turn tool-call flows.
pub async fn ws_chat_stream(
    request: &ChatRequest,
    token: &str,
    endpoint: Option<&str>,
    previous_response_id: Option<&str>,
) -> Result<impl Stream<Item = Result<ChatChunk>> + Send> {
    let ws_url = endpoint.unwrap_or(WS_ENDPOINT);

    // Build a full WebSocket handshake request. When using Request::builder()
    // with connect_async, tungstenite does NOT auto-generate WebSocket headers,
    // so we must provide all required headers ourselves.
    let ws_request = tungstenite::http::Request::builder()
        .uri(ws_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("OpenAI-Beta", "realtime=v1")
        .header("Sec-WebSocket-Version", "13")
        .header(
            "Sec-WebSocket-Key",
            tungstenite::handshake::client::generate_key(),
        )
        .header(
            "Host",
            url::Url::parse(ws_url)
                .map(|u| u.host_str().unwrap_or("api.openai.com").to_string())
                .unwrap_or_else(|_| "api.openai.com".to_string()),
        )
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .body(())
        .map_err(|e| LlmError::Stream(format!("Failed to build WS request: {}", e)))?;

    // Connect with a 30-second timeout to avoid hanging on network issues
    let (ws_stream, _response) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio_tungstenite::connect_async(ws_request),
    )
    .await
    .map_err(|_| LlmError::Stream("WebSocket connection timed out after 30s".to_string()))?
    .map_err(|e| LlmError::Stream(format!("WebSocket connection failed: {}", e)))?;

    // Send response.create message
    let create_msg = build_response_create(request, previous_response_id);
    let create_json = serde_json::to_string(&create_msg)
        .map_err(|e| LlmError::Stream(format!("Failed to serialize: {}", e)))?;

    let (mut write, read) = ws_stream.split();

    write
        .send(tungstenite::Message::Text(create_json.into()))
        .await
        .map_err(|e| LlmError::Stream(format!("Failed to send WS message: {}", e)))?;

    // Drop the write half -- we're done sending. We do NOT call write.close()
    // because that sends a WebSocket Close frame, which would tell the server
    // to shut down the connection and kill our response stream prematurely.
    // Dropping the SplitSink releases resources without sending a Close frame.
    drop(write);

    // Track the current function call ID so argument deltas can inherit it
    let current_call_id = std::sync::Arc::new(tokio::sync::Mutex::new(Option::<String>::None));
    let call_id_clone = current_call_id.clone();

    // Convert WebSocket message stream into ChatChunk stream.
    // Each read is wrapped in a per-message idle timeout matching OpenAI's
    // 60-minute session lifetime. If no message arrives within that window,
    // we treat the connection as dead and yield Done.
    let idle_timeout = std::time::Duration::from_secs(WS_IDLE_TIMEOUT_SECS);

    Ok(futures::stream::unfold(
        (read, call_id_clone, idle_timeout),
        |(mut read, call_id, timeout)| async move {
            // Wait for the next message with a 60-minute idle timeout
            let msg = match tokio::time::timeout(timeout, read.next()).await {
                Ok(Some(msg)) => msg,
                Ok(None) => {
                    // Stream ended normally
                    return None;
                }
                Err(_) => {
                    // Idle timeout -- no data received within the session window
                    tracing::warn!("WebSocket idle timeout ({timeout:?}), closing stream");
                    return Some((Ok(ChatChunk::Done), (read, call_id, timeout)));
                }
            };

            let chunk = match msg {
                Ok(tungstenite::Message::Text(text)) => {
                    parse_ws_event(&text, &call_id).await
                }
                Ok(tungstenite::Message::Ping(_)) | Ok(tungstenite::Message::Pong(_)) => {
                    // Keepalive frames -- skip and read the next message
                    None
                }
                Ok(tungstenite::Message::Close(frame)) => {
                    if let Some(ref f) = frame {
                        tracing::debug!(
                            code = %f.code,
                            reason = %f.reason,
                            "WebSocket closed by server"
                        );
                    }
                    Some(Ok(ChatChunk::Done))
                }
                Err(e) => {
                    tracing::warn!(error = %e, "WebSocket stream error");
                    Some(Err(LlmError::Stream(format!("WebSocket error: {}", e))))
                }
                _ => None, // Ignore binary
            };

            match chunk {
                Some(Ok(ChatChunk::Done)) => {
                    // Terminal -- emit Done then stop the unfold
                    Some((Ok(ChatChunk::Done), (read, call_id, timeout)))
                }
                Some(result) => Some((result, (read, call_id, timeout))),
                None => {
                    // Filtered out (ping/pong/unrecognized) -- continue reading.
                    // We recurse via unfold state; the next iteration reads again.
                    Some((Ok(ChatChunk::TextDelta(String::new())), (read, call_id, timeout)))
                }
            }
        },
    )
    // Filter out empty sentinel deltas produced by skipped frames
    .filter(|chunk| {
        std::future::ready(!matches!(chunk, Ok(ChatChunk::TextDelta(t)) if t.is_empty()))
    }))
}

/// Build a `response.create` message for the Responses API.
fn build_response_create(
    request: &ChatRequest,
    previous_response_id: Option<&str>,
) -> serde_json::Value {
    let mut input: Vec<serde_json::Value> = Vec::new();

    // Convert ChatMessages to Responses API input format
    for msg in &request.messages {
        match msg.role {
            ChatRole::System => {
                input.push(serde_json::json!({
                    "type": "message",
                    "role": "system",
                    "content": [{"type": "input_text", "text": msg.content}],
                }));
            }
            ChatRole::User => {
                input.push(serde_json::json!({
                    "type": "message",
                    "role": "user",
                    "content": [{"type": "input_text", "text": msg.content}],
                }));
            }
            ChatRole::Assistant => {
                input.push(serde_json::json!({
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": msg.content}],
                }));
            }
            ChatRole::Tool => {
                if let Some(ref tool_call_id) = msg.tool_call_id {
                    input.push(serde_json::json!({
                        "type": "function_call_output",
                        "call_id": tool_call_id,
                        "output": msg.content,
                    }));
                }
            }
        }
    }

    // Build tools array
    let tools: Vec<serde_json::Value> = request
        .tools
        .iter()
        .map(|t| {
            serde_json::json!({
                "type": "function",
                "name": t.name,
                "description": t.description,
                "parameters": t.parameters,
            })
        })
        .collect();

    let mut msg = serde_json::json!({
        "type": "response.create",
        "model": request.model,
        "input": input,
    });

    if !tools.is_empty() {
        msg["tools"] = serde_json::json!(tools);
    }

    if let Some(max_tokens) = request.max_tokens {
        msg["max_output_tokens"] = serde_json::json!(max_tokens);
    }

    if let Some(temp) = request.temperature {
        msg["temperature"] = serde_json::json!(temp);
    }

    if let Some(prev_id) = previous_response_id {
        msg["previous_response_id"] = serde_json::json!(prev_id);
    }

    msg
}

/// Parse a WebSocket server event into a ChatChunk.
///
/// `current_call_id` tracks the active function call so that argument delta
/// events (which don't include the call_id) can inherit it from the preceding
/// `output_item.added` event.
async fn parse_ws_event(
    text: &str,
    current_call_id: &tokio::sync::Mutex<Option<String>>,
) -> Option<Result<ChatChunk>> {
    let event: std::result::Result<WsEvent, _> = serde_json::from_str(text);
    match event {
        Ok(ev) => match ev.r#type.as_str() {
            "response.output_text.delta" => ev.delta.map(|d| Ok(ChatChunk::TextDelta(d))),
            "response.function_call_arguments.delta" => {
                // Inherit the call_id from the most recent output_item.added
                let inherited_id = current_call_id.lock().await.clone();
                Some(Ok(ChatChunk::ToolCallDelta {
                    index: ev.output_index.unwrap_or(0),
                    id: inherited_id,
                    name: None,
                    arguments_delta: ev.delta.unwrap_or_default(),
                }))
            }
            "response.output_item.added" => {
                // Check if it's a function call
                if let Some(ref item) = ev.item {
                    if item.r#type.as_deref() == Some("function_call") {
                        // Store the call_id so subsequent argument deltas can use it
                        *current_call_id.lock().await = item.call_id.clone();
                        return Some(Ok(ChatChunk::ToolCallDelta {
                            index: ev.output_index.unwrap_or(0),
                            id: item.call_id.clone(),
                            name: item.name.clone(),
                            arguments_delta: String::new(),
                        }));
                    }
                }
                None
            }
            "response.completed" => {
                // Extract usage if available
                if let Some(ref response) = ev.response {
                    if let Some(ref usage) = response.usage {
                        return Some(Ok(ChatChunk::Usage {
                            prompt_tokens: usage.input_tokens.unwrap_or(0),
                            completion_tokens: usage.output_tokens.unwrap_or(0),
                            total_tokens: usage.total_tokens.unwrap_or(0),
                        }));
                    }
                }
                Some(Ok(ChatChunk::Done))
            }
            "response.failed" => {
                let reason = ev
                    .response
                    .and_then(|r| r.status_details)
                    .unwrap_or_else(|| "Unknown error".to_string());
                Some(Err(LlmError::Provider(format!(
                    "OpenAI WS error: {}",
                    reason
                ))))
            }
            other => {
                tracing::trace!(event_type = other, "Ignoring unrecognized WebSocket event");
                None
            }
        },
        Err(e) => {
            tracing::trace!(error = %e, raw = %text.chars().take(200).collect::<String>(), "Failed to parse WebSocket event");
            None
        }
    }
}

// ── WebSocket event types ───────────────────────────────────────────

#[derive(Deserialize)]
struct WsEvent {
    r#type: String,
    #[serde(default)]
    delta: Option<String>,
    #[serde(default)]
    output_index: Option<usize>,
    #[serde(default)]
    item: Option<WsItem>,
    #[serde(default)]
    response: Option<WsResponse>,
}

#[derive(Deserialize)]
struct WsItem {
    r#type: Option<String>,
    call_id: Option<String>,
    name: Option<String>,
}

#[derive(Deserialize)]
struct WsResponse {
    #[serde(default)]
    usage: Option<WsUsage>,
    #[serde(default)]
    status_details: Option<String>,
    /// The response ID for conversation continuity across tool-call turns.
    #[serde(default)]
    #[allow(dead_code)]
    pub id: Option<String>,
}

#[derive(Deserialize)]
struct WsUsage {
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
    total_tokens: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChatMessage, ToolSchema};

    // Helper to run async parse_ws_event in tests
    async fn parse_event(text: &str) -> Option<Result<ChatChunk>> {
        let call_id = tokio::sync::Mutex::new(None);
        parse_ws_event(text, &call_id).await
    }

    #[test]
    fn test_build_response_create_basic() {
        let request = ChatRequest {
            model: "gpt-4.1".to_string(),
            messages: vec![
                ChatMessage::system("You are helpful"),
                ChatMessage::user("Hello"),
            ],
            tools: vec![],
            max_tokens: Some(4096),
            temperature: Some(0.7),
            stream: true,
            thinking: None,
            effort: None,
        };

        let msg = build_response_create(&request, None);
        assert_eq!(msg["type"], "response.create");
        assert_eq!(msg["model"], "gpt-4.1");
        assert_eq!(msg["max_output_tokens"], 4096);

        let input = msg["input"].as_array().unwrap();
        assert_eq!(input.len(), 2);
        assert_eq!(input[0]["role"], "system");
        assert_eq!(input[1]["role"], "user");
    }

    #[test]
    fn test_build_response_create_with_tools() {
        let request = ChatRequest {
            model: "gpt-4.1".to_string(),
            messages: vec![ChatMessage::user("Search")],
            tools: vec![ToolSchema {
                name: "search".to_string(),
                description: "Search the web".to_string(),
                parameters: serde_json::json!({"type": "object"}),
                required_permission: None,
            }],
            max_tokens: None,
            temperature: None,
            stream: true,
            thinking: None,
            effort: None,
        };

        let msg = build_response_create(&request, None);
        let tools = msg["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "search");
        assert_eq!(tools[0]["type"], "function");
    }

    #[test]
    fn test_build_response_create_tool_result() {
        let request = ChatRequest {
            model: "gpt-4.1".to_string(),
            messages: vec![
                ChatMessage::user("Search"),
                ChatMessage::tool_result("call_123", "result text"),
            ],
            tools: vec![],
            max_tokens: None,
            temperature: None,
            stream: true,
            thinking: None,
            effort: None,
        };

        let msg = build_response_create(&request, None);
        let input = msg["input"].as_array().unwrap();
        assert_eq!(input.len(), 2);
        assert_eq!(input[1]["type"], "function_call_output");
        assert_eq!(input[1]["call_id"], "call_123");
    }

    #[test]
    fn test_build_response_create_with_previous_id() {
        let request = ChatRequest {
            model: "gpt-4.1".to_string(),
            messages: vec![ChatMessage::user("Continue")],
            tools: vec![],
            max_tokens: None,
            temperature: None,
            stream: true,
            thinking: None,
            effort: None,
        };

        let msg = build_response_create(&request, Some("resp_abc123"));
        assert_eq!(msg["previous_response_id"], "resp_abc123");
    }

    #[tokio::test]
    async fn test_parse_ws_text_delta() {
        let event = r#"{"type":"response.output_text.delta","delta":"Hello"}"#;
        let result = parse_event(event).await;
        match result {
            Some(Ok(ChatChunk::TextDelta(text))) => assert_eq!(text, "Hello"),
            _ => panic!("Expected TextDelta"),
        }
    }

    #[tokio::test]
    async fn test_parse_ws_function_call_start() {
        let event = r#"{"type":"response.output_item.added","output_index":0,"item":{"type":"function_call","call_id":"call_1","name":"search"}}"#;
        let result = parse_event(event).await;
        match result {
            Some(Ok(ChatChunk::ToolCallDelta { index, id, name, .. })) => {
                assert_eq!(index, 0);
                assert_eq!(id, Some("call_1".to_string()));
                assert_eq!(name, Some("search".to_string()));
            }
            _ => panic!("Expected ToolCallDelta"),
        }
    }

    #[tokio::test]
    async fn test_parse_ws_function_call_args_inherits_id() {
        let call_id = tokio::sync::Mutex::new(None);

        // First event: function call start (sets the call_id)
        let start_event = r#"{"type":"response.output_item.added","output_index":0,"item":{"type":"function_call","call_id":"call_99","name":"search"}}"#;
        let _ = parse_ws_event(start_event, &call_id).await;

        // Second event: argument delta (should inherit call_id)
        let args_event =
            r#"{"type":"response.function_call_arguments.delta","output_index":0,"delta":"{\"q\":"}"#;
        let result = parse_ws_event(args_event, &call_id).await;
        match result {
            Some(Ok(ChatChunk::ToolCallDelta {
                id,
                arguments_delta,
                ..
            })) => {
                assert_eq!(id, Some("call_99".to_string()));
                assert_eq!(arguments_delta, "{\"q\":");
            }
            _ => panic!("Expected ToolCallDelta with inherited call_id"),
        }
    }

    #[tokio::test]
    async fn test_parse_ws_completed() {
        let event = r#"{"type":"response.completed","response":{"usage":{"input_tokens":100,"output_tokens":50,"total_tokens":150}}}"#;
        let result = parse_event(event).await;
        match result {
            Some(Ok(ChatChunk::Usage {
                prompt_tokens,
                completion_tokens,
                total_tokens,
            })) => {
                assert_eq!(prompt_tokens, 100);
                assert_eq!(completion_tokens, 50);
                assert_eq!(total_tokens, 150);
            }
            _ => panic!("Expected Usage"),
        }
    }

    #[tokio::test]
    async fn test_parse_ws_unknown_event() {
        let event = r#"{"type":"session.created"}"#;
        let result = parse_event(event).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_parse_ws_failed() {
        let event = r#"{"type":"response.failed","response":{"status_details":"Rate limit exceeded"}}"#;
        let result = parse_event(event).await;
        match result {
            Some(Err(e)) => {
                assert!(e.to_string().contains("Rate limit exceeded"));
            }
            _ => panic!("Expected error"),
        }
    }
}
