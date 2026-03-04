//! Messaging tools that bridge the native tool system with channel plugins.
//!
//! Gated by `messaging.chat` permission.

use async_trait::async_trait;
use omni_permissions::capability::Capability;

use super::NativeTool;
use crate::error::{LlmError, Result};

/// Native tool for sending messages via connected channels.
pub struct SendMessageTool;

#[async_trait]
impl NativeTool for SendMessageTool {
    fn name(&self) -> &str {
        "send_message"
    }

    fn description(&self) -> &str {
        "Send a message to a recipient via a connected messaging channel. Requires a channel_id \
         from list_channels (compound key like 'discord:production') and a recipient identifier \
         whose format depends on the channel type (numeric ID for Discord/Telegram, phone number \
         for WhatsApp/Signal, channel ID for Slack, room ID for Matrix, etc.). Always call \
         list_channels first to verify the channel is connected, and confirm with the user \
         before sending."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "channel_id": {
                    "type": "string",
                    "description": "The channel instance compound key (e.g. 'discord:production', 'telegram:default', 'slack:workspace1'). Get this from list_channels -- never guess it."
                },
                "recipient": {
                    "type": "string",
                    "description": "Recipient identifier. Format varies by channel: Discord=numeric channel/user ID, Telegram=numeric chat ID, Slack=C/U-prefixed ID, WhatsApp/Signal=phone number with country code, Matrix=room ID (!room:server), IRC=channel name or nick, Teams=conversation ID, iMessage=chat GUID."
                },
                "text": {
                    "type": "string",
                    "description": "Message text to send. Long messages are automatically chunked per channel limits."
                }
            },
            "required": ["channel_id", "recipient", "text"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::MessagingChat(None)
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let channel_id = params["channel_id"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'channel_id' parameter is required".to_string()))?;
        let recipient = params["recipient"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'recipient' parameter is required".to_string()))?;
        let text = params["text"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'text' parameter is required".to_string()))?;

        // Return structured intent -- the agent loop's process_delegated_action()
        // routes this to the DelegatedActionHandler for actual delivery.
        Ok(serde_json::json!({
            "action": "send_message",
            "channel_id": channel_id,
            "recipient": recipient,
            "text": text,
            "status": "pending",
        }))
    }
}

/// Native tool for listing available messaging channels.
pub struct ListChannelsTool;

#[async_trait]
impl NativeTool for ListChannelsTool {
    fn name(&self) -> &str {
        "list_channels"
    }

    fn description(&self) -> &str {
        "List all messaging channel instances with connection status, features, and compound keys. \
         Returns each channel's id (compound key like 'discord:production'), channel_type, \
         instance_id, name, status (connected/disconnected/connecting/error), and features \
         (direct_messages, group_messages, media_attachments, reactions, read_receipts, \
         typing_indicators). Only channels with status 'connected' can send messages. \
         Always call this before send_message."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::MessagingChat(None)
    }

    async fn execute(&self, _params: serde_json::Value) -> Result<serde_json::Value> {
        // Returns info about available channels.
        // The actual channel enumeration happens at a higher level (ChannelManager).
        // This returns structured data for the agent loop to fill in.
        Ok(serde_json::json!({
            "action": "list_channels",
            "note": "Channel listing delegated to ChannelManager.",
            "channels": [],
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_message_schema() {
        let tool = SendMessageTool;
        assert_eq!(tool.name(), "send_message");
        assert_eq!(tool.required_capability().capability_key(), "messaging.chat");
        let schema = tool.parameters_schema();
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("channel_id")));
        assert!(required.contains(&serde_json::json!("recipient")));
        assert!(required.contains(&serde_json::json!("text")));
    }

    #[test]
    fn test_list_channels_schema() {
        let tool = ListChannelsTool;
        assert_eq!(tool.name(), "list_channels");
        assert_eq!(tool.required_capability().capability_key(), "messaging.chat");
    }

    #[tokio::test]
    async fn test_send_message_execution() {
        let tool = SendMessageTool;
        let result = tool
            .execute(serde_json::json!({
                "channel_id": "whatsapp",
                "recipient": "+1234567890",
                "text": "Hello!"
            }))
            .await
            .unwrap();
        assert_eq!(result["channel_id"], "whatsapp");
        assert_eq!(result["status"], "pending");
    }

    #[tokio::test]
    async fn test_list_channels_execution() {
        let tool = ListChannelsTool;
        let result = tool.execute(serde_json::json!({})).await.unwrap();
        assert!(result["channels"].is_array());
    }
}
