//! Minimal LSP protocol types for the subset we use.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// LSP Position (0-based line and column).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// LSP Range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// LSP Location.
#[derive(Debug, Clone, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

/// LSP Diagnostic severity.
#[derive(Debug, Clone, Copy, Deserialize)]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

/// LSP Diagnostic.
#[derive(Debug, Clone, Deserialize)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: Option<u32>,
    pub message: String,
    pub source: Option<String>,
}

/// LSP SymbolKind (subset).
pub fn symbol_kind_name(kind: u32) -> &'static str {
    match kind {
        1 => "File",
        2 => "Module",
        3 => "Namespace",
        4 => "Package",
        5 => "Class",
        6 => "Method",
        7 => "Property",
        8 => "Field",
        9 => "Constructor",
        10 => "Enum",
        11 => "Interface",
        12 => "Function",
        13 => "Variable",
        14 => "Constant",
        15 => "String",
        16 => "Number",
        17 => "Boolean",
        18 => "Array",
        19 => "Object",
        22 => "Struct",
        23 => "Event",
        24 => "Operator",
        25 => "TypeParameter",
        _ => "Unknown",
    }
}

/// LSP SymbolInformation.
#[derive(Debug, Clone, Deserialize)]
pub struct SymbolInformation {
    pub name: String,
    pub kind: u32,
    pub location: Location,
    #[serde(rename = "containerName")]
    pub container_name: Option<String>,
}

/// LSP DocumentSymbol (newer format).
#[derive(Debug, Clone, Deserialize)]
pub struct DocumentSymbol {
    pub name: String,
    pub kind: u32,
    pub range: Range,
    #[serde(rename = "selectionRange")]
    pub selection_range: Range,
    pub children: Option<Vec<DocumentSymbol>>,
}

/// LSP Hover result.
#[derive(Debug, Clone, Deserialize)]
pub struct Hover {
    pub contents: Value, // Can be string, MarkupContent, or array
}

/// LSP TextEdit (for rename).
#[derive(Debug, Clone, Deserialize)]
pub struct TextEdit {
    pub range: Range,
    #[serde(rename = "newText")]
    pub new_text: String,
}

/// LSP WorkspaceEdit.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceEdit {
    pub changes: Option<std::collections::HashMap<String, Vec<TextEdit>>>,
}

/// Simplified LSP JSON-RPC request.
#[derive(Debug, Serialize)]
pub struct LspRequest {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: String,
    pub params: Value,
}

impl LspRequest {
    pub fn new(id: u64, method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            method: method.into(),
            params,
        }
    }
}

/// LSP notification (no id).
#[derive(Debug, Serialize)]
pub struct LspNotification {
    pub jsonrpc: &'static str,
    pub method: String,
    pub params: Value,
}

impl LspNotification {
    pub fn new(method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            method: method.into(),
            params,
        }
    }
}
