use std::sync::Arc;

use async_trait::async_trait;
use omni_guardian::policy::{ToolInfo, ToolRegistry};
use tokio::sync::RwLock;

/// Bridges the `ExtensionHost` to Guardian's `ToolRegistry` trait,
/// avoiding a direct dependency from omni-guardian on omni-extensions.
///
/// Because `ExtensionHost` contains wasmtime types that are `!Sync`,
/// this bridge caches the tool list in a thread-safe `RwLock`.
/// The cache is refreshed by calling `refresh()` before tool validation.
#[derive(Clone)]
pub struct ExtensionToolRegistry {
    cached_tools: Arc<RwLock<Vec<(String, ToolInfo)>>>,
}

impl Default for ExtensionToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtensionToolRegistry {
    /// Create an empty registry. Call `refresh()` to populate.
    pub fn new() -> Self {
        Self {
            cached_tools: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Refresh the cached tool list from an ExtensionHost.
    /// Called from the agent loop where ExtensionHost is accessible.
    pub async fn refresh_from(
        &self,
        tools: Vec<(String, omni_extensions::manifest::ToolDefinition)>,
    ) {
        let converted: Vec<(String, ToolInfo)> = tools
            .into_iter()
            .map(|(ext_id, tool_def)| {
                (
                    ext_id,
                    ToolInfo {
                        name: tool_def.name,
                        description: tool_def.description,
                        parameters: tool_def.parameters,
                    },
                )
            })
            .collect();
        let mut cache = self.cached_tools.write().await;
        *cache = converted;
    }

    /// Append additional tools (e.g. flowchart tools) to the cached list.
    /// Called after `refresh_from()` to include non-extension tools in Guardian's
    /// SP-4 tool call validation.
    pub async fn append_from(
        &self,
        tools: Vec<(String, omni_extensions::manifest::ToolDefinition)>,
    ) {
        let additional: Vec<(String, ToolInfo)> = tools
            .into_iter()
            .map(|(id, tool_def)| {
                (
                    id,
                    ToolInfo {
                        name: tool_def.name,
                        description: tool_def.description,
                        parameters: tool_def.parameters,
                    },
                )
            })
            .collect();
        let mut cache = self.cached_tools.write().await;
        cache.extend(additional);
    }
}

#[async_trait]
impl ToolRegistry for ExtensionToolRegistry {
    async fn get_all_tools(&self) -> Vec<(String, ToolInfo)> {
        self.cached_tools.read().await.clone()
    }
}
