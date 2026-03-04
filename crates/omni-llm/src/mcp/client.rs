//! MCP client -- manages a single MCP server connection.
//!
//! Handles initialization handshake, tool discovery, and tool invocation.

use std::collections::HashMap;

use serde_json::{json, Value};

use super::protocol::*;
use super::transport::StdioTransport;
use crate::error::{LlmError, Result};
use crate::types::ToolSchema;

/// Configuration for an MCP server connection.
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: Option<String>,
    pub auto_start: bool,
}

impl From<omni_core::config::McpServerEntry> for McpServerConfig {
    fn from(entry: omni_core::config::McpServerEntry) -> Self {
        Self {
            name: entry.name,
            command: entry.command,
            args: entry.args,
            env: entry.env,
            working_dir: entry.working_dir,
            auto_start: entry.auto_start,
        }
    }
}

/// A connected MCP server client.
pub struct McpClient {
    config: McpServerConfig,
    transport: StdioTransport,
    tools: Vec<McpToolDef>,
    server_info: Option<ServerInfo>,
}

impl McpClient {
    /// Connect to an MCP server: spawn process, initialize, discover tools.
    pub async fn connect(config: McpServerConfig) -> Result<Self> {
        let transport = StdioTransport::spawn(
            &config.command,
            &config.args,
            &config.env,
            config.working_dir.as_deref(),
        )
        .await?;

        let mut client = Self {
            config,
            transport,
            tools: Vec::new(),
            server_info: None,
        };

        client.initialize().await?;
        client.discover_tools().await?;

        Ok(client)
    }

    /// MCP initialize handshake.
    async fn initialize(&mut self) -> Result<()> {
        let params = InitializeParams {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ClientCapabilities {
                roots: Some(RootsCapability {
                    list_changed: false,
                }),
            },
            client_info: ClientInfo {
                name: "omni".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        let result = self
            .transport
            .request("initialize", Some(serde_json::to_value(params).unwrap()))
            .await?;

        let init_result: InitializeResult = serde_json::from_value(result)
            .map_err(|e| LlmError::ToolCall(format!("Invalid initialize response: {e}")))?;

        self.server_info = init_result.server_info;

        // Send initialized notification
        self.transport.notify("notifications/initialized", None).await?;

        tracing::info!(
            "MCP server '{}' initialized (protocol {})",
            self.config.name,
            init_result.protocol_version
        );

        Ok(())
    }

    /// Discover available tools from the server.
    async fn discover_tools(&mut self) -> Result<()> {
        let result = self.transport.request("tools/list", None).await?;

        let tools_result: ToolsListResult = serde_json::from_value(result)
            .map_err(|e| LlmError::ToolCall(format!("Invalid tools/list response: {e}")))?;

        tracing::info!(
            "MCP server '{}' provides {} tools",
            self.config.name,
            tools_result.tools.len()
        );
        for tool in &tools_result.tools {
            tracing::debug!("  tool: {} - {:?}", tool.name, tool.description);
        }

        self.tools = tools_result.tools;
        Ok(())
    }

    /// Get tool schemas formatted for the LLM, with namespaced names.
    pub fn get_tool_schemas(&self) -> Vec<ToolSchema> {
        self.tools
            .iter()
            .map(|tool| {
                let namespaced_name = format!("mcp_{}_{}", self.config.name, tool.name);
                ToolSchema {
                    name: namespaced_name,
                    description: format!(
                        "[MCP:{}] {}",
                        self.config.name,
                        tool.description.as_deref().unwrap_or(&tool.name)
                    ),
                    parameters: tool.input_schema.clone(),
                    required_permission: None,
                }
            })
            .collect()
    }

    /// Check if this client handles a given namespaced tool name.
    pub fn handles_tool(&self, namespaced_name: &str) -> bool {
        let prefix = format!("mcp_{}_", self.config.name);
        if let Some(tool_name) = namespaced_name.strip_prefix(&prefix) {
            self.tools.iter().any(|t| t.name == tool_name)
        } else {
            false
        }
    }

    /// Extract the original tool name from a namespaced name.
    pub fn extract_tool_name<'a>(&self, namespaced_name: &'a str) -> Option<&'a str> {
        let prefix = format!("mcp_{}_", self.config.name);
        namespaced_name.strip_prefix(&prefix)
    }

    /// Call a tool on this MCP server.
    pub async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        let params = ToolCallParams {
            name: tool_name.to_string(),
            arguments,
        };

        let result = self
            .transport
            .request("tools/call", Some(serde_json::to_value(params).unwrap()))
            .await?;

        let call_result: ToolCallResult = serde_json::from_value(result)
            .map_err(|e| LlmError::ToolCall(format!("Invalid tools/call response: {e}")))?;

        if call_result.is_error == Some(true) {
            let error_text = call_result
                .content
                .iter()
                .filter_map(|c| c.text.as_deref())
                .collect::<Vec<_>>()
                .join("\n");
            return Err(LlmError::ToolCall(format!(
                "MCP tool '{}' returned error: {}",
                tool_name, error_text
            )));
        }

        // Combine text content items
        let text_parts: Vec<&str> = call_result
            .content
            .iter()
            .filter_map(|c| c.text.as_deref())
            .collect();

        if text_parts.len() == 1 {
            // Try to parse as JSON, fall back to string
            if let Ok(parsed) = serde_json::from_str::<Value>(text_parts[0]) {
                return Ok(parsed);
            }
            Ok(json!({ "result": text_parts[0] }))
        } else {
            Ok(json!({
                "result": text_parts.join("\n"),
                "content_count": call_result.content.len()
            }))
        }
    }

    /// Get the server name.
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Get a clone of the server config (for restart).
    pub fn config(&self) -> McpServerConfig {
        self.config.clone()
    }

    /// Get the number of tools available.
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    /// Get tool definitions (raw MCP format).
    pub fn tool_defs(&self) -> &[McpToolDef] {
        &self.tools
    }

    /// Refresh the tool list from the server.
    pub async fn refresh_tools(&mut self) -> Result<()> {
        self.discover_tools().await
    }

    /// Shutdown the MCP server connection.
    pub async fn shutdown(&mut self) {
        self.transport.shutdown().await;
    }
}
