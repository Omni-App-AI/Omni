//! LSP (Language Server Protocol) integration tool.
//!
//! Manages connections to language servers and exposes structured
//! code intelligence: go-to-definition, find-references, hover,
//! diagnostics, symbol search, and rename preview.
//!
//! Gated by `code.intelligence` permission.

pub mod client;
pub mod protocol;

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use omni_permissions::capability::Capability;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use super::NativeTool;
use crate::error::{LlmError, Result};

use client::LspClient;

pub struct LspTool {
    clients: Arc<Mutex<HashMap<String, LspClient>>>,
}

impl LspTool {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn detect_server(language: &str) -> Result<(String, Vec<String>)> {
        match language.to_lowercase().as_str() {
            "rust" => Ok(("rust-analyzer".to_string(), Vec::new())),
            "typescript" | "javascript" | "ts" | "js" => Ok((
                "typescript-language-server".to_string(),
                vec!["--stdio".to_string()],
            )),
            "python" | "py" => {
                // Try pyright first, fall back to pylsp
                Ok(("pyright-langserver".to_string(), vec!["--stdio".to_string()]))
            }
            "go" => Ok(("gopls".to_string(), vec!["serve".to_string()])),
            _ => Err(LlmError::ToolCall(format!(
                "No language server configured for '{}'. Supported: rust, typescript, javascript, python, go",
                language
            ))),
        }
    }

    async fn action_start(&self, params: &Value) -> Result<Value> {
        let language = params
            .get("language")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'language' is required".to_string()))?;
        let root_path = params
            .get("root_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'root_path' is required".to_string()))?;

        let mut clients = self.clients.lock().await;
        if clients.contains_key(language) {
            return Ok(json!({
                "language": language,
                "status": "already_running",
                "message": "Language server already running. Use 'stop' first to restart."
            }));
        }

        let (cmd, args) = Self::detect_server(language)?;
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        let client = LspClient::start(&cmd, &args_refs, root_path, language).await?;

        clients.insert(language.to_string(), client);

        Ok(json!({
            "language": language,
            "status": "started",
            "server": cmd,
            "root_path": root_path
        }))
    }

    async fn get_client<'a>(
        clients: &'a HashMap<String, LspClient>,
        file: &str,
    ) -> Result<&'a LspClient> {
        // Find the right client by checking which language server covers this file
        let ext = std::path::Path::new(file)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let language = match ext {
            "rs" => "rust",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" | "mjs" => "javascript",
            "py" => "python",
            "go" => "go",
            _ => "",
        };

        // Try exact language match, then try typescript for js files
        clients
            .get(language)
            .or_else(|| {
                if language == "javascript" {
                    clients.get("typescript")
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                LlmError::ToolCall(format!(
                    "No language server running for '{}' files. Start one with action='start'.",
                    ext
                ))
            })
    }

    async fn action_goto_definition(&self, params: &Value) -> Result<Value> {
        let file = params.get("file").and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'file' is required".to_string()))?;
        let line = params.get("line").and_then(|v| v.as_u64())
            .ok_or_else(|| LlmError::ToolCall("'line' is required".to_string()))? as u32;
        let column = params.get("column").and_then(|v| v.as_u64())
            .ok_or_else(|| LlmError::ToolCall("'column' is required".to_string()))? as u32;

        let clients = self.clients.lock().await;
        let client = Self::get_client(&clients, file).await?;
        let result = client.goto_definition(file, line, column).await?;

        Ok(json!({ "definition": result }))
    }

    async fn action_find_references(&self, params: &Value) -> Result<Value> {
        let file = params.get("file").and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'file' is required".to_string()))?;
        let line = params.get("line").and_then(|v| v.as_u64())
            .ok_or_else(|| LlmError::ToolCall("'line' is required".to_string()))? as u32;
        let column = params.get("column").and_then(|v| v.as_u64())
            .ok_or_else(|| LlmError::ToolCall("'column' is required".to_string()))? as u32;

        let clients = self.clients.lock().await;
        let client = Self::get_client(&clients, file).await?;
        let result = client.find_references(file, line, column).await?;

        Ok(json!({ "references": result }))
    }

    async fn action_hover(&self, params: &Value) -> Result<Value> {
        let file = params.get("file").and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'file' is required".to_string()))?;
        let line = params.get("line").and_then(|v| v.as_u64())
            .ok_or_else(|| LlmError::ToolCall("'line' is required".to_string()))? as u32;
        let column = params.get("column").and_then(|v| v.as_u64())
            .ok_or_else(|| LlmError::ToolCall("'column' is required".to_string()))? as u32;

        let clients = self.clients.lock().await;
        let client = Self::get_client(&clients, file).await?;
        let result = client.hover(file, line, column).await?;

        Ok(json!({ "hover": result }))
    }

    async fn action_symbols(&self, params: &Value) -> Result<Value> {
        let query = params.get("query").and_then(|v| v.as_str());
        let file = params.get("file").and_then(|v| v.as_str());

        let clients = self.clients.lock().await;

        if let Some(f) = file {
            // Document symbols
            let client = Self::get_client(&clients, f).await?;
            let result = client.document_symbols(f).await?;
            Ok(json!({ "symbols": result, "scope": "file" }))
        } else if let Some(q) = query {
            // Workspace symbols -- use any available client
            let client = clients
                .values()
                .next()
                .ok_or_else(|| LlmError::ToolCall("No language server running".to_string()))?;
            let result = client.workspace_symbols(q).await?;
            Ok(json!({ "symbols": result, "scope": "workspace" }))
        } else {
            Err(LlmError::ToolCall(
                "Either 'file' or 'query' is required for 'symbols' action".to_string(),
            ))
        }
    }

    async fn action_diagnostics(&self, params: &Value) -> Result<Value> {
        let file = params.get("file").and_then(|v| v.as_str());

        if let Some(f) = file {
            let clients = self.clients.lock().await;
            let client = Self::get_client(&clients, f).await?;

            // Ensure the file is open so the server produces diagnostics
            client.ensure_open(f).await?;

            // Allow the server some time to compute diagnostics
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            Ok(json!({
                "file": f,
                "note": "Diagnostics are delivered asynchronously by the language server. \
                         The file is now being tracked. Use 'test_runner' for synchronous error checking.",
                "status": "file_tracked"
            }))
        } else {
            let clients = self.clients.lock().await;
            let languages: Vec<&str> = clients.keys().map(|s| s.as_str()).collect();
            Ok(json!({
                "active_servers": languages,
                "note": "Specify 'file' to track diagnostics for a specific file."
            }))
        }
    }

    async fn action_rename_preview(&self, params: &Value) -> Result<Value> {
        let file = params.get("file").and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'file' is required".to_string()))?;
        let line = params.get("line").and_then(|v| v.as_u64())
            .ok_or_else(|| LlmError::ToolCall("'line' is required".to_string()))? as u32;
        let column = params.get("column").and_then(|v| v.as_u64())
            .ok_or_else(|| LlmError::ToolCall("'column' is required".to_string()))? as u32;
        let new_name = params.get("new_name").and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'new_name' is required".to_string()))?;

        let clients = self.clients.lock().await;
        let client = Self::get_client(&clients, file).await?;
        let result = client.rename_preview(file, line, column, new_name).await?;

        Ok(json!({
            "rename_preview": result,
            "note": "This is a preview only. Use edit_file to apply changes."
        }))
    }

    async fn action_stop(&self, params: &Value) -> Result<Value> {
        let language = params
            .get("language")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'language' is required".to_string()))?;

        let mut clients = self.clients.lock().await;
        if let Some(mut client) = clients.remove(language) {
            client.shutdown().await;
            Ok(json!({ "language": language, "status": "stopped" }))
        } else {
            Err(LlmError::ToolCall(format!(
                "No language server running for '{language}'"
            )))
        }
    }
}

#[async_trait]
impl NativeTool for LspTool {
    fn name(&self) -> &str {
        "lsp"
    }

    fn description(&self) -> &str {
        "Real-time code intelligence via Language Server Protocol. Requires starting a server first. \
         Actions: 'start' (launch server), 'goto_definition' (jump to definition), \
         'find_references' (find all usages), 'hover' (get type info/docs), \
         'diagnostics' (get compiler errors/warnings), 'symbols' (search/list symbols), \
         'rename_preview' (preview multi-file rename), 'stop' (shutdown server). \
         Supports: Rust (rust-analyzer), TypeScript/JS (tsserver), Python (pyright), Go (gopls). \
         Lines and columns are 1-based. For offline symbol search without a running server, \
         use code_search instead."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["start", "goto_definition", "find_references", "hover",
                             "diagnostics", "symbols", "rename_preview", "stop"],
                    "description": "LSP action to perform"
                },
                "language": {
                    "type": "string",
                    "enum": ["rust", "typescript", "javascript", "python", "go"],
                    "description": "Language (for 'start' and 'stop')"
                },
                "root_path": {
                    "type": "string",
                    "description": "Project root path (for 'start')"
                },
                "file": {
                    "type": "string",
                    "description": "Source file path (for navigation actions and 'symbols')"
                },
                "line": {
                    "type": "integer",
                    "description": "Line number, 1-based"
                },
                "column": {
                    "type": "integer",
                    "description": "Column number, 1-based"
                },
                "query": {
                    "type": "string",
                    "description": "Symbol search query (for 'symbols' workspace search)"
                },
                "new_name": {
                    "type": "string",
                    "description": "New name (for 'rename_preview')"
                }
            },
            "required": ["action"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::CodeIntelligence
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let action = params
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'action' is required".to_string()))?;

        match action {
            "start" => self.action_start(&params).await,
            "goto_definition" => self.action_goto_definition(&params).await,
            "find_references" => self.action_find_references(&params).await,
            "hover" => self.action_hover(&params).await,
            "diagnostics" => self.action_diagnostics(&params).await,
            "symbols" => self.action_symbols(&params).await,
            "rename_preview" => self.action_rename_preview(&params).await,
            "stop" => self.action_stop(&params).await,
            _ => Err(LlmError::ToolCall(format!(
                "Unknown LSP action: '{action}'. Valid: start, goto_definition, find_references, \
                 hover, diagnostics, symbols, rename_preview, stop"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_metadata() {
        let tool = LspTool::new();
        assert_eq!(tool.name(), "lsp");
        assert!(!tool.description().is_empty());
        assert!(matches!(tool.required_capability(), Capability::CodeIntelligence));
    }

    #[test]
    fn test_detect_server_rust() {
        let (cmd, _) = LspTool::detect_server("rust").unwrap();
        assert_eq!(cmd, "rust-analyzer");
    }

    #[test]
    fn test_detect_server_typescript() {
        let (cmd, _) = LspTool::detect_server("typescript").unwrap();
        assert_eq!(cmd, "typescript-language-server");
    }

    #[test]
    fn test_detect_server_python() {
        let (cmd, _) = LspTool::detect_server("python").unwrap();
        assert_eq!(cmd, "pyright-langserver");
    }

    #[test]
    fn test_detect_server_go() {
        let (cmd, _) = LspTool::detect_server("go").unwrap();
        assert_eq!(cmd, "gopls");
    }

    #[test]
    fn test_detect_server_unknown() {
        assert!(LspTool::detect_server("fortran").is_err());
    }

    #[tokio::test]
    async fn test_unknown_action() {
        let tool = LspTool::new();
        let result = tool.execute(json!({ "action": "invalid" })).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stop_nonexistent() {
        let tool = LspTool::new();
        let result = tool
            .execute(json!({ "action": "stop", "language": "rust" }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_goto_no_server() {
        let tool = LspTool::new();
        let result = tool
            .execute(json!({
                "action": "goto_definition",
                "file": "test.rs",
                "line": 1,
                "column": 1
            }))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No language server"));
    }
}
