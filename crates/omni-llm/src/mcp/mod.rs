//! MCP (Model Context Protocol) client -- a tool multiplexer.
//!
//! Connects to external MCP servers, discovers their tools, and exposes
//! them as callable tools in the agent loop. This is NOT a native tool
//! itself -- it dynamically injects tools from external servers.
//!
//! Architecture:
//! - `McpManager` holds a map of `McpClient` connections
//! - Each `McpClient` wraps a single MCP server (stdio transport)
//! - Tool names are namespaced: `mcp_<server>_<tool>`
//! - The AgentLoop collects schemas from McpManager and routes calls

pub mod client;
pub mod protocol;
pub mod transport;

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::error::{LlmError, Result};
use crate::types::ToolSchema;

pub use client::{McpClient, McpServerConfig};

/// Manages all MCP server connections.
pub struct McpManager {
    clients: Arc<RwLock<HashMap<String, McpClient>>>,
}

impl McpManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add and connect to an MCP server.
    pub async fn add_server(&self, config: McpServerConfig) -> Result<()> {
        let name = config.name.clone();

        // Check for duplicate
        {
            let clients = self.clients.read().await;
            if clients.contains_key(&name) {
                return Err(LlmError::ToolCall(format!(
                    "MCP server '{}' already connected",
                    name
                )));
            }
        }

        let client = McpClient::connect(config).await?;
        tracing::info!(
            "MCP server '{}' connected with {} tools",
            name,
            client.tool_count()
        );

        let mut clients = self.clients.write().await;
        clients.insert(name, client);
        Ok(())
    }

    /// Remove and disconnect an MCP server.
    pub async fn remove_server(&self, name: &str) -> Result<()> {
        let mut clients = self.clients.write().await;
        if let Some(mut client) = clients.remove(name) {
            client.shutdown().await;
            tracing::info!("MCP server '{}' disconnected", name);
            Ok(())
        } else {
            Err(LlmError::ToolCall(format!(
                "MCP server '{}' not found",
                name
            )))
        }
    }

    /// Restart an MCP server (disconnect + reconnect).
    pub async fn restart_server(&self, name: &str) -> Result<()> {
        let config = {
            let mut clients = self.clients.write().await;
            if let Some(mut client) = clients.remove(name) {
                let config = client.config();
                client.shutdown().await;
                config
            } else {
                return Err(LlmError::ToolCall(format!(
                    "MCP server '{}' not found",
                    name
                )));
            }
        };

        self.add_server(config).await
    }

    /// Get all tool schemas from all connected MCP servers.
    pub async fn get_all_schemas(&self) -> Vec<ToolSchema> {
        let clients = self.clients.read().await;
        let mut schemas = Vec::new();
        for client in clients.values() {
            schemas.extend(client.get_tool_schemas());
        }
        schemas
    }

    /// Check if any connected server handles a given tool name.
    pub async fn handles_tool(&self, namespaced_name: &str) -> bool {
        let clients = self.clients.read().await;
        clients.values().any(|c| c.handles_tool(namespaced_name))
    }

    /// Execute a tool call, routing to the appropriate MCP server.
    pub async fn execute_tool(
        &self,
        namespaced_name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let clients = self.clients.read().await;

        for client in clients.values() {
            if client.handles_tool(namespaced_name) {
                let tool_name = client
                    .extract_tool_name(namespaced_name)
                    .ok_or_else(|| {
                        LlmError::ToolCall(format!("Failed to extract tool name from '{namespaced_name}'"))
                    })?;
                return client.call_tool(tool_name, arguments).await;
            }
        }

        Err(LlmError::ToolNotFound(namespaced_name.to_string()))
    }

    /// List all connected servers with their tool counts.
    pub async fn list_servers(&self) -> Vec<McpServerInfo> {
        let clients = self.clients.read().await;
        clients
            .values()
            .map(|c| McpServerInfo {
                name: c.name().to_string(),
                tool_count: c.tool_count(),
                tools: c
                    .tool_defs()
                    .iter()
                    .map(|t| t.name.clone())
                    .collect(),
            })
            .collect()
    }

    /// Get the total number of connected servers.
    pub async fn server_count(&self) -> usize {
        self.clients.read().await.len()
    }

    /// Refresh tool lists from all connected servers.
    pub async fn refresh_all_tools(&self) -> Result<()> {
        let mut clients = self.clients.write().await;
        for client in clients.values_mut() {
            client.refresh_tools().await?;
        }
        Ok(())
    }

    /// Shutdown all MCP server connections.
    pub async fn shutdown_all(&self) {
        let mut clients = self.clients.write().await;
        for (name, mut client) in clients.drain() {
            tracing::info!("Shutting down MCP server '{}'", name);
            client.shutdown().await;
        }
    }
}

/// Summary info about a connected MCP server.
#[derive(Debug, Clone, serde::Serialize)]
pub struct McpServerInfo {
    pub name: String,
    pub tool_count: usize,
    pub tools: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_new_empty() {
        let manager = McpManager::new();
        assert_eq!(manager.server_count().await, 0);
        assert!(manager.get_all_schemas().await.is_empty());
        assert!(manager.list_servers().await.is_empty());
    }

    #[tokio::test]
    async fn test_manager_handles_unknown_tool() {
        let manager = McpManager::new();
        assert!(!manager.handles_tool("mcp_foo_bar").await);
    }

    #[tokio::test]
    async fn test_manager_execute_unknown_tool() {
        let manager = McpManager::new();
        let result = manager
            .execute_tool("mcp_foo_bar", serde_json::json!({}))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_manager_remove_nonexistent() {
        let manager = McpManager::new();
        let result = manager.remove_server("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_manager_shutdown_empty() {
        let manager = McpManager::new();
        manager.shutdown_all().await;
        // Should not panic
    }
}
