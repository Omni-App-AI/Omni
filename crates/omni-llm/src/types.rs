use serde::{Deserialize, Serialize};

/// A single chunk from the LLM stream.
#[derive(Debug, Clone)]
pub enum ChatChunk {
    /// Text content delta.
    TextDelta(String),

    /// Thinking content delta (extended/adaptive thinking).
    ThinkingDelta(String),

    /// Signature delta for thinking block verification (Anthropic-specific).
    /// Accumulated alongside thinking text for multi-turn preservation.
    SignatureDelta(String),

    /// Tool/function call being assembled.
    ToolCallDelta {
        index: usize,
        id: Option<String>,
        name: Option<String>,
        arguments_delta: String,
    },

    /// Usage statistics (sent at end of stream).
    Usage {
        prompt_tokens: u32,
        completion_tokens: u32,
        total_tokens: u32,
    },

    /// Stream complete.
    Done,
}

/// A thinking block from Claude's extended/adaptive thinking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingBlock {
    /// The (possibly summarized) thinking text.
    pub thinking: String,
    /// Opaque signature for verification when passing back to the API.
    pub signature: String,
}

/// A redacted thinking block (encrypted, must pass back verbatim).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactedThinkingBlock {
    /// Encrypted opaque data.
    pub data: String,
}

/// A thinking content item -- either visible or redacted.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ThinkingContent {
    #[serde(rename = "thinking")]
    Thinking(ThinkingBlock),
    #[serde(rename = "redacted_thinking")]
    Redacted(RedactedThinkingBlock),
}

/// Thinking mode for the LLM request.
#[derive(Debug, Clone)]
pub enum ThinkingMode {
    /// Claude dynamically determines when and how much to think.
    /// Recommended for Opus 4.6 and Sonnet 4.6.
    Adaptive,
    /// Fixed thinking budget (deprecated on Opus/Sonnet 4.6, still supported on older models).
    Enabled { budget_tokens: u32 },
}

/// Effort level for adaptive thinking.
#[derive(Debug, Clone, Copy)]
pub enum ThinkingEffort {
    /// Minimize thinking. Skips thinking for simple tasks.
    Low,
    /// Moderate thinking. May skip for very simple queries.
    Medium,
    /// Deep reasoning (default). Claude always thinks.
    High,
    /// No constraints on thinking depth. Opus 4.6 only.
    Max,
}

/// An image attached to a message (for multimodal tool results).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    /// MIME type (e.g., "image/png", "image/jpeg").
    pub mime_type: String,
    /// Base64-encoded image data.
    pub data: String,
}

/// A message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Optional images attached to this message (for multimodal tool results).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ImageContent>>,
    /// Thinking blocks from extended/adaptive thinking (for multi-turn preservation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_content: Option<Vec<ThinkingContent>>,
}

impl ChatMessage {
    pub fn system(content: &str) -> Self {
        Self {
            role: ChatRole::System,
            content: content.to_string(),
            tool_call_id: None,
            tool_calls: None,
            images: None,
            thinking_content: None,
        }
    }

    pub fn user(content: &str) -> Self {
        Self {
            role: ChatRole::User,
            content: content.to_string(),
            tool_call_id: None,
            tool_calls: None,
            images: None,
            thinking_content: None,
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.to_string(),
            tool_call_id: None,
            tool_calls: None,
            images: None,
            thinking_content: None,
        }
    }

    pub fn assistant_with_tool_calls(content: &str, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.to_string(),
            tool_call_id: None,
            tool_calls: Some(tool_calls),
            images: None,
            thinking_content: None,
        }
    }

    /// Create an assistant message with thinking blocks for multi-turn preservation.
    pub fn assistant_with_thinking(
        content: &str,
        thinking_content: Vec<ThinkingContent>,
    ) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.to_string(),
            tool_call_id: None,
            tool_calls: None,
            images: None,
            thinking_content: if thinking_content.is_empty() { None } else { Some(thinking_content) },
        }
    }

    /// Create an assistant message with both tool calls and thinking blocks.
    pub fn assistant_with_tool_calls_and_thinking(
        content: &str,
        tool_calls: Vec<ToolCall>,
        thinking_content: Vec<ThinkingContent>,
    ) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.to_string(),
            tool_call_id: None,
            tool_calls: Some(tool_calls),
            images: None,
            thinking_content: if thinking_content.is_empty() { None } else { Some(thinking_content) },
        }
    }

    pub fn tool_result(tool_call_id: &str, content: &str) -> Self {
        Self {
            role: ChatRole::Tool,
            content: content.to_string(),
            tool_call_id: Some(tool_call_id.to_string()),
            tool_calls: None,
            images: None,
            thinking_content: None,
        }
    }

    /// Create a tool result with attached images for multimodal responses.
    pub fn tool_result_with_images(
        tool_call_id: &str,
        content: &str,
        images: Vec<ImageContent>,
    ) -> Self {
        Self {
            role: ChatRole::Tool,
            content: content.to_string(),
            tool_call_id: Some(tool_call_id.to_string()),
            tool_calls: None,
            images: if images.is_empty() { None } else { Some(images) },
            thinking_content: None,
        }
    }
}

/// Role of a message participant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

impl ChatRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChatRole::System => "system",
            ChatRole::User => "user",
            ChatRole::Assistant => "assistant",
            ChatRole::Tool => "tool",
        }
    }
}

/// A tool call from the LLM.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Schema for a tool exposed to the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    /// The permission capability key required to use this tool (e.g., "messaging.chat").
    /// Used by the system prompt builder to inform the LLM about access requirements.
    /// Not sent to LLM providers in the tool definition -- purely informational.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub required_permission: Option<String>,
}

/// A chat completion request.
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub tools: Vec<ToolSchema>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: bool,
    /// Thinking mode (adaptive or fixed budget). Only used by providers that support it.
    pub thinking: Option<ThinkingMode>,
    /// Effort level for adaptive thinking.
    pub effort: Option<ThinkingEffort>,
}

/// Information about an available model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub context_window: u32,
    pub max_output_tokens: Option<u32>,
}

/// Authentication methods supported by a provider.
#[derive(Debug, Clone)]
pub enum AuthMethod {
    ApiKey {
        env_var_hint: Option<String>,
    },
    OAuth {
        authorize_url: String,
        token_url: String,
        scopes: Vec<String>,
    },
    DeviceCode {
        device_auth_url: String,
    },
    Custom {
        instructions: String,
    },
}

/// Result of the agent loop.
#[derive(Debug)]
pub struct AgentResult {
    pub text: String,
    pub iterations: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_constructors() {
        let sys = ChatMessage::system("You are helpful.");
        assert_eq!(sys.role, ChatRole::System);
        assert_eq!(sys.content, "You are helpful.");
        assert!(sys.tool_calls.is_none());

        let user = ChatMessage::user("Hello");
        assert_eq!(user.role, ChatRole::User);

        let asst = ChatMessage::assistant("Hi there");
        assert_eq!(asst.role, ChatRole::Assistant);

        let tc = ToolCall {
            id: "tc-1".to_string(),
            name: "get_weather".to_string(),
            arguments: "{}".to_string(),
        };
        let asst_tc = ChatMessage::assistant_with_tool_calls("", vec![tc]);
        assert!(asst_tc.tool_calls.is_some());
        assert_eq!(asst_tc.tool_calls.as_ref().unwrap().len(), 1);

        let tool = ChatMessage::tool_result("tc-1", "sunny");
        assert_eq!(tool.role, ChatRole::Tool);
        assert_eq!(tool.tool_call_id, Some("tc-1".to_string()));
        assert!(tool.images.is_none());

        let img = ImageContent {
            mime_type: "image/png".to_string(),
            data: "iVBOR...".to_string(),
        };
        let tool_img = ChatMessage::tool_result_with_images("tc-2", "screenshot", vec![img]);
        assert_eq!(tool_img.role, ChatRole::Tool);
        assert!(tool_img.images.is_some());
        assert_eq!(tool_img.images.as_ref().unwrap().len(), 1);
        assert_eq!(tool_img.images.as_ref().unwrap()[0].mime_type, "image/png");
    }

    #[test]
    fn test_chat_role_serialize() {
        let json = serde_json::to_string(&ChatRole::Assistant).unwrap();
        assert_eq!(json, "\"assistant\"");

        let role: ChatRole = serde_json::from_str("\"user\"").unwrap();
        assert_eq!(role, ChatRole::User);
    }

    #[test]
    fn test_tool_call_default() {
        let tc = ToolCall::default();
        assert!(tc.id.is_empty());
        assert!(tc.name.is_empty());
        assert!(tc.arguments.is_empty());
    }

    #[test]
    fn test_chat_message_serialization() {
        let msg = ChatMessage::user("Hello");
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["role"], "user");
        assert_eq!(json["content"], "Hello");
        // tool_call_id and tool_calls should be absent (skip_serializing_if)
        assert!(json.get("tool_call_id").is_none());
        assert!(json.get("tool_calls").is_none());
    }

    #[test]
    fn test_thinking_content_serialization() {
        let thinking = ThinkingContent::Thinking(ThinkingBlock {
            thinking: "Let me consider...".to_string(),
            signature: "sig_abc".to_string(),
        });
        let json = serde_json::to_value(&thinking).unwrap();
        assert_eq!(json["type"], "thinking");
        assert_eq!(json["thinking"], "Let me consider...");
        assert_eq!(json["signature"], "sig_abc");

        let redacted = ThinkingContent::Redacted(RedactedThinkingBlock {
            data: "encrypted".to_string(),
        });
        let json = serde_json::to_value(&redacted).unwrap();
        assert_eq!(json["type"], "redacted_thinking");
        assert_eq!(json["data"], "encrypted");
    }

    #[test]
    fn test_assistant_with_thinking_constructors() {
        let thinking = vec![ThinkingContent::Thinking(ThinkingBlock {
            thinking: "reasoning".to_string(),
            signature: "sig".to_string(),
        })];
        let msg = ChatMessage::assistant_with_thinking("answer", thinking);
        assert_eq!(msg.role, ChatRole::Assistant);
        assert_eq!(msg.content, "answer");
        assert!(msg.thinking_content.is_some());
        assert_eq!(msg.thinking_content.as_ref().unwrap().len(), 1);

        // Empty thinking should produce None
        let msg_empty = ChatMessage::assistant_with_thinking("answer", vec![]);
        assert!(msg_empty.thinking_content.is_none());
    }

    #[test]
    fn test_model_info() {
        let model = ModelInfo {
            id: "gpt-4o".to_string(),
            name: "GPT-4o".to_string(),
            context_window: 128000,
            max_output_tokens: Some(4096),
        };
        let json = serde_json::to_string(&model).unwrap();
        let deserialized: ModelInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "gpt-4o");
        assert_eq!(deserialized.context_window, 128000);
    }
}
