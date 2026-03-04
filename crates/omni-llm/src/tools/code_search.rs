//! Code search tool -- symbol-aware code search and indexing.
//!
//! Provides fast symbol extraction and search across codebases using
//! regex-based pattern matching (lightweight ctags approach). Supports
//! Rust, TypeScript/JavaScript, Python, Go, C/C++, Java, and C#.
//!
//! 4 actions: `index` (build symbol index), `search` (query symbols/text),
//! `symbols` (list symbols in a file), `dependencies` (show imports for a file).
//!
//! Gated by `filesystem.read` permission.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use omni_permissions::capability::Capability;
use regex::Regex;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use super::NativeTool;
use crate::error::{LlmError, Result};

/// Maximum files to index in a single `index` call.
const MAX_INDEX_FILES: usize = 10_000;
/// Maximum results per search query.
const MAX_SEARCH_RESULTS: usize = 100;
/// Maximum file size to index (256KB).
const MAX_FILE_SIZE: u64 = 256 * 1024;

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub file: String,
    pub line: u32,
    pub parent: Option<String>,
    pub signature: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    Trait,
    Constant,
    Variable,
    Type,
    Module,
    Import,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolKind::Function => write!(f, "function"),
            SymbolKind::Method => write!(f, "method"),
            SymbolKind::Class => write!(f, "class"),
            SymbolKind::Struct => write!(f, "struct"),
            SymbolKind::Enum => write!(f, "enum"),
            SymbolKind::Interface => write!(f, "interface"),
            SymbolKind::Trait => write!(f, "trait"),
            SymbolKind::Constant => write!(f, "constant"),
            SymbolKind::Variable => write!(f, "variable"),
            SymbolKind::Type => write!(f, "type"),
            SymbolKind::Module => write!(f, "module"),
            SymbolKind::Import => write!(f, "import"),
        }
    }
}

#[derive(Debug, Clone)]
struct ImportInfo {
    module: String,
    items: Vec<String>,
    line: u32,
}

#[derive(Default)]
struct SymbolIndex {
    symbols: Vec<SymbolInfo>,
    _root_path: String,
    _file_count: usize,
}

pub struct CodeSearchTool {
    index: Arc<Mutex<Option<SymbolIndex>>>,
}

impl CodeSearchTool {
    pub fn new() -> Self {
        Self {
            index: Arc::new(Mutex::new(None)),
        }
    }

    fn detect_language(path: &Path) -> Option<&'static str> {
        match path.extension()?.to_str()? {
            "rs" => Some("rust"),
            "ts" | "tsx" => Some("typescript"),
            "js" | "jsx" | "mjs" => Some("javascript"),
            "py" => Some("python"),
            "go" => Some("go"),
            "c" | "h" => Some("c"),
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some("cpp"),
            "java" => Some("java"),
            "cs" => Some("csharp"),
            _ => None,
        }
    }

    fn extract_symbols(content: &str, language: &str, file_path: &str) -> Vec<SymbolInfo> {
        let mut symbols = Vec::new();

        // Language-specific regex patterns
        let patterns: Vec<(Regex, SymbolKind)> = match language {
            "rust" => vec![
                (Regex::new(r"(?m)^\s*(?:pub(?:\(.*?\))?\s+)?(?:async\s+)?fn\s+(\w+)").unwrap(), SymbolKind::Function),
                (Regex::new(r"(?m)^\s*(?:pub(?:\(.*?\))?\s+)?struct\s+(\w+)").unwrap(), SymbolKind::Struct),
                (Regex::new(r"(?m)^\s*(?:pub(?:\(.*?\))?\s+)?enum\s+(\w+)").unwrap(), SymbolKind::Enum),
                (Regex::new(r"(?m)^\s*(?:pub(?:\(.*?\))?\s+)?trait\s+(\w+)").unwrap(), SymbolKind::Trait),
                (Regex::new(r"(?m)^\s*(?:pub(?:\(.*?\))?\s+)?type\s+(\w+)").unwrap(), SymbolKind::Type),
                (Regex::new(r"(?m)^\s*(?:pub(?:\(.*?\))?\s+)?mod\s+(\w+)").unwrap(), SymbolKind::Module),
                (Regex::new(r"(?m)^\s*(?:pub(?:\(.*?\))?\s+)?const\s+(\w+)").unwrap(), SymbolKind::Constant),
                (Regex::new(r"(?m)^\s*(?:pub(?:\(.*?\))?\s+)?static\s+(\w+)").unwrap(), SymbolKind::Constant),
                (Regex::new(r"(?m)^\s*impl(?:<.*?>)?\s+(?:(\w+)\s+for\s+)?(\w+)").unwrap(), SymbolKind::Struct),
            ],
            "typescript" | "javascript" => vec![
                (Regex::new(r"(?m)^\s*(?:export\s+)?(?:async\s+)?function\s+(\w+)").unwrap(), SymbolKind::Function),
                (Regex::new(r"(?m)^\s*(?:export\s+)?class\s+(\w+)").unwrap(), SymbolKind::Class),
                (Regex::new(r"(?m)^\s*(?:export\s+)?interface\s+(\w+)").unwrap(), SymbolKind::Interface),
                (Regex::new(r"(?m)^\s*(?:export\s+)?type\s+(\w+)").unwrap(), SymbolKind::Type),
                (Regex::new(r"(?m)^\s*(?:export\s+)?enum\s+(\w+)").unwrap(), SymbolKind::Enum),
                (Regex::new(r"(?m)^\s*(?:export\s+)?const\s+(\w+)").unwrap(), SymbolKind::Constant),
                (Regex::new(r"(?m)^\s*(?:export\s+)?(?:let|var)\s+(\w+)").unwrap(), SymbolKind::Variable),
                (Regex::new(r"(?m)^\s*(\w+)\s*[=(]\s*(?:async\s+)?\(").unwrap(), SymbolKind::Function),
            ],
            "python" => vec![
                (Regex::new(r"(?m)^(?:\s*)(?:async\s+)?def\s+(\w+)").unwrap(), SymbolKind::Function),
                (Regex::new(r"(?m)^class\s+(\w+)").unwrap(), SymbolKind::Class),
                (Regex::new(r"(?m)^(\w+)\s*=").unwrap(), SymbolKind::Variable),
            ],
            "go" => vec![
                (Regex::new(r"(?m)^func\s+(?:\(\w+\s+\*?\w+\)\s+)?(\w+)").unwrap(), SymbolKind::Function),
                (Regex::new(r"(?m)^type\s+(\w+)\s+struct").unwrap(), SymbolKind::Struct),
                (Regex::new(r"(?m)^type\s+(\w+)\s+interface").unwrap(), SymbolKind::Interface),
                (Regex::new(r"(?m)^type\s+(\w+)").unwrap(), SymbolKind::Type),
                (Regex::new(r"(?m)^const\s+(\w+)").unwrap(), SymbolKind::Constant),
                (Regex::new(r"(?m)^var\s+(\w+)").unwrap(), SymbolKind::Variable),
            ],
            "java" | "csharp" => vec![
                (Regex::new(r"(?m)^\s*(?:public|private|protected|static|final|abstract|override|virtual|async)*\s*(?:void|int|string|bool|float|double|long|char|byte|var|Task|[\w<>\[\]]+)\s+(\w+)\s*\(").unwrap(), SymbolKind::Function),
                (Regex::new(r"(?m)^\s*(?:public|private|protected|static|abstract|sealed|final)?\s*class\s+(\w+)").unwrap(), SymbolKind::Class),
                (Regex::new(r"(?m)^\s*(?:public|private|protected)?\s*interface\s+(\w+)").unwrap(), SymbolKind::Interface),
                (Regex::new(r"(?m)^\s*(?:public|private|protected)?\s*enum\s+(\w+)").unwrap(), SymbolKind::Enum),
            ],
            "c" | "cpp" => vec![
                (Regex::new(r"(?m)^(?:\w+[\s*]+)+(\w+)\s*\(").unwrap(), SymbolKind::Function),
                (Regex::new(r"(?m)^\s*(?:typedef\s+)?struct\s+(\w+)").unwrap(), SymbolKind::Struct),
                (Regex::new(r"(?m)^\s*(?:typedef\s+)?enum\s+(\w+)").unwrap(), SymbolKind::Enum),
                (Regex::new(r"(?m)^\s*class\s+(\w+)").unwrap(), SymbolKind::Class),
                (Regex::new(r"(?m)^\s*#define\s+(\w+)").unwrap(), SymbolKind::Constant),
                (Regex::new(r"(?m)^\s*typedef\s+.*\s+(\w+)\s*;").unwrap(), SymbolKind::Type),
            ],
            _ => return symbols,
        };

        let lines: Vec<&str> = content.lines().collect();

        for (pattern, kind) in &patterns {
            for cap in pattern.captures_iter(content) {
                // Get the last capture group (handles impl patterns)
                let name = cap
                    .get(cap.len() - 1)
                    .or_else(|| cap.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();

                if name.is_empty() || name == "_" {
                    continue;
                }

                // Find line number
                let match_start = cap.get(0).unwrap().start();
                let line = content[..match_start].chars().filter(|c| *c == '\n').count() as u32 + 1;

                // Get signature (the full line)
                let sig = lines
                    .get((line as usize).saturating_sub(1))
                    .unwrap_or(&"")
                    .trim()
                    .to_string();

                symbols.push(SymbolInfo {
                    name,
                    kind: *kind,
                    file: file_path.to_string(),
                    line,
                    parent: None,
                    signature: sig,
                });
            }
        }

        symbols
    }

    fn extract_imports(content: &str, language: &str) -> Vec<ImportInfo> {
        let mut imports = Vec::new();

        let pattern = match language {
            "rust" => r"(?m)^\s*use\s+([\w:]+(?:::\{[^}]+\})?(?:::\*)?)\s*;",
            "typescript" | "javascript" => r#"(?m)^\s*import\s+(?:\{([^}]+)\}\s+from\s+)?['"]([^'"]+)['"]"#,
            "python" => r"(?m)^\s*(?:from\s+([\w.]+)\s+import\s+([\w, *]+)|import\s+([\w., ]+))",
            "go" => r#"(?m)^\s*(?:import\s+(?:\([\s\S]*?\)|"([^"]+)"))"#,
            "java" | "csharp" => r"(?m)^\s*(?:using|import)\s+([\w.*]+)\s*;",
            _ => return imports,
        };

        if let Ok(re) = Regex::new(pattern) {
            for cap in re.captures_iter(content) {
                let match_start = cap.get(0).unwrap().start();
                let line = content[..match_start].chars().filter(|c| *c == '\n').count() as u32 + 1;

                let full_match = cap.get(0).unwrap().as_str().trim().to_string();

                // Extract module and items based on language
                let (module, items) = match language {
                    "rust" => {
                        let m = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                        (m.to_string(), vec![m.to_string()])
                    }
                    "typescript" | "javascript" => {
                        let items_str = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                        let module = cap.get(2).map(|m| m.as_str()).unwrap_or(&full_match);
                        let items: Vec<String> = if items_str.is_empty() {
                            vec![module.to_string()]
                        } else {
                            items_str.split(',').map(|s| s.trim().to_string()).collect()
                        };
                        (module.to_string(), items)
                    }
                    "python" => {
                        if let Some(from_mod) = cap.get(1) {
                            let items_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                            let items: Vec<String> = items_str.split(',').map(|s| s.trim().to_string()).collect();
                            (from_mod.as_str().to_string(), items)
                        } else if let Some(imp) = cap.get(3) {
                            let modules: Vec<String> = imp.as_str().split(',').map(|s| s.trim().to_string()).collect();
                            (modules.join(", "), modules)
                        } else {
                            continue;
                        }
                    }
                    _ => {
                        let m = cap.get(1).map(|m| m.as_str()).unwrap_or(&full_match);
                        (m.to_string(), vec![m.to_string()])
                    }
                };

                imports.push(ImportInfo { module, items, line });
            }
        }

        imports
    }

    fn should_ignore(path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        let ignore_patterns = [
            "node_modules", "target", ".git", "__pycache__", ".tox",
            "dist", "build", ".next", "vendor", ".venv", "venv",
            ".eggs", "*.min.js", "*.min.css", "*.map",
        ];
        ignore_patterns.iter().any(|pat| {
            if pat.starts_with('*') {
                path_str.ends_with(&pat[1..])
            } else {
                path_str.contains(pat)
            }
        })
    }

    async fn collect_files(root: &Path, languages: &Option<Vec<String>>) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let mut stack = vec![root.to_path_buf()];

        while let Some(dir) = stack.pop() {
            if Self::should_ignore(&dir) {
                continue;
            }

            let mut entries = tokio::fs::read_dir(&dir)
                .await
                .map_err(|e| LlmError::ToolCall(format!("Failed to read dir: {e}")))?;

            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if Self::should_ignore(&path) {
                    continue;
                }

                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    // Check extension
                    if let Some(lang) = Self::detect_language(&path) {
                        if let Some(ref allowed) = languages {
                            if !allowed.iter().any(|l| l == lang) {
                                continue;
                            }
                        }

                        // Check file size
                        if let Ok(meta) = tokio::fs::metadata(&path).await {
                            if meta.len() <= MAX_FILE_SIZE {
                                files.push(path);
                            }
                        }
                    }
                }

                if files.len() >= MAX_INDEX_FILES {
                    break;
                }
            }

            if files.len() >= MAX_INDEX_FILES {
                break;
            }
        }

        Ok(files)
    }

    async fn action_index(&self, params: &Value) -> Result<Value> {
        let root_path = params
            .get("root_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'root_path' is required".to_string()))?;

        let languages: Option<Vec<String>> = params
            .get("languages")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect());

        let root = Path::new(root_path);
        if !root.is_dir() {
            return Err(LlmError::ToolCall(format!("'{}' is not a directory", root_path)));
        }

        let files = Self::collect_files(root, &languages).await?;

        let mut all_symbols = Vec::new();
        let mut file_count = 0;
        let mut lang_counts: HashMap<&str, usize> = HashMap::new();

        for file_path in &files {
            if let Ok(content) = tokio::fs::read_to_string(file_path).await {
                if let Some(lang) = Self::detect_language(file_path) {
                    let rel_path = file_path
                        .strip_prefix(root)
                        .unwrap_or(file_path)
                        .to_string_lossy()
                        .replace('\\', "/");

                    let symbols = Self::extract_symbols(&content, lang, &rel_path);
                    *lang_counts.entry(lang).or_insert(0) += 1;
                    all_symbols.extend(symbols);
                    file_count += 1;
                }
            }
        }

        let symbol_count = all_symbols.len();

        let mut index = self.index.lock().await;
        *index = Some(SymbolIndex {
            symbols: all_symbols,
            _root_path: root_path.to_string(),
            _file_count: file_count,
        });

        Ok(json!({
            "status": "indexed",
            "root_path": root_path,
            "files_indexed": file_count,
            "symbols_found": symbol_count,
            "languages": lang_counts,
        }))
    }

    async fn action_search(&self, params: &Value) -> Result<Value> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'query' is required".to_string()))?;

        let kind_filter = params.get("type").and_then(|v| v.as_str());
        let language_filter = params.get("language").and_then(|v| v.as_str());
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(MAX_SEARCH_RESULTS as u64) as usize;

        let index = self.index.lock().await;
        let idx = index
            .as_ref()
            .ok_or_else(|| LlmError::ToolCall(
                "No index built. Run action='index' first with root_path.".to_string(),
            ))?;

        let query_lower = query.to_lowercase();

        let kind_match = kind_filter.map(|k| match k {
            "function" | "fn" => SymbolKind::Function,
            "method" => SymbolKind::Method,
            "class" => SymbolKind::Class,
            "struct" => SymbolKind::Struct,
            "enum" => SymbolKind::Enum,
            "interface" => SymbolKind::Interface,
            "trait" => SymbolKind::Trait,
            "constant" | "const" => SymbolKind::Constant,
            "variable" | "var" => SymbolKind::Variable,
            "type" => SymbolKind::Type,
            "module" | "mod" => SymbolKind::Module,
            _ => SymbolKind::Function, // default
        });

        let mut results: Vec<&SymbolInfo> = idx
            .symbols
            .iter()
            .filter(|s| {
                let name_match = s.name.to_lowercase().contains(&query_lower);
                let kind_ok = kind_match.map_or(true, |k| s.kind == k);
                let lang_ok = language_filter.map_or(true, |l| {
                    let file_lang = Path::new(&s.file)
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|ext| match ext {
                            "rs" => "rust",
                            "ts" | "tsx" => "typescript",
                            "js" | "jsx" => "javascript",
                            "py" => "python",
                            "go" => "go",
                            _ => ext,
                        })
                        .unwrap_or("");
                    file_lang == l
                });
                name_match && kind_ok && lang_ok
            })
            .collect();

        // Sort: exact matches first, then by name length (shorter = more relevant)
        results.sort_by(|a, b| {
            let a_exact = a.name.to_lowercase() == query_lower;
            let b_exact = b.name.to_lowercase() == query_lower;
            b_exact.cmp(&a_exact).then(a.name.len().cmp(&b.name.len()))
        });

        results.truncate(limit);

        let json_results: Vec<Value> = results
            .iter()
            .map(|s| {
                json!({
                    "name": s.name,
                    "kind": s.kind.to_string(),
                    "file": s.file,
                    "line": s.line,
                    "signature": s.signature,
                })
            })
            .collect();

        Ok(json!({
            "query": query,
            "results": json_results,
            "count": json_results.len(),
            "total_symbols": idx.symbols.len(),
        }))
    }

    async fn action_symbols(&self, params: &Value) -> Result<Value> {
        let file = params
            .get("file")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'file' is required".to_string()))?;

        let path = Path::new(file);
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to read '{}': {e}", file)))?;

        let language = Self::detect_language(path)
            .ok_or_else(|| LlmError::ToolCall(format!("Unsupported file type: {file}")))?;

        let symbols = Self::extract_symbols(&content, language, file);

        let json_symbols: Vec<Value> = symbols
            .iter()
            .map(|s| {
                json!({
                    "name": s.name,
                    "kind": s.kind.to_string(),
                    "line": s.line,
                    "signature": s.signature,
                })
            })
            .collect();

        Ok(json!({
            "file": file,
            "language": language,
            "symbols": json_symbols,
            "count": json_symbols.len(),
        }))
    }

    async fn action_dependencies(&self, params: &Value) -> Result<Value> {
        let file = params
            .get("file")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'file' is required".to_string()))?;

        let path = Path::new(file);
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to read '{}': {e}", file)))?;

        let language = Self::detect_language(path)
            .ok_or_else(|| LlmError::ToolCall(format!("Unsupported file type: {file}")))?;

        let imports = Self::extract_imports(&content, language);

        let json_imports: Vec<Value> = imports
            .iter()
            .map(|i| {
                json!({
                    "module": i.module,
                    "items": i.items,
                    "line": i.line,
                })
            })
            .collect();

        Ok(json!({
            "file": file,
            "language": language,
            "imports": json_imports,
            "count": json_imports.len(),
        }))
    }
}

#[async_trait]
impl NativeTool for CodeSearchTool {
    fn name(&self) -> &str {
        "code_search"
    }

    fn description(&self) -> &str {
        "Offline code intelligence using syntax analysis. Index a project first, then search. \
         Actions: 'index' (build symbol index for a project), 'search' (query symbols by name, \
         with optional type/language filters), 'symbols' (list all symbols in a single file), \
         'dependencies' (show imports/uses for a file). \
         Supports: Rust, TypeScript, JavaScript, Python, Go, C/C++, Java, C#. \
         Works without a running language server -- for real-time type info, use lsp instead."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["index", "search", "symbols", "dependencies"],
                    "description": "Code search action to perform"
                },
                "root_path": {
                    "type": "string",
                    "description": "Project root to index (for 'index')"
                },
                "languages": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Filter to specific languages (for 'index')"
                },
                "query": {
                    "type": "string",
                    "description": "Symbol name to search for (for 'search')"
                },
                "type": {
                    "type": "string",
                    "enum": ["function", "method", "class", "struct", "enum",
                             "interface", "trait", "constant", "variable", "type", "module"],
                    "description": "Filter by symbol type (for 'search')"
                },
                "language": {
                    "type": "string",
                    "description": "Filter by language (for 'search')"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max results (default 100, for 'search')"
                },
                "file": {
                    "type": "string",
                    "description": "File path (for 'symbols' and 'dependencies')"
                }
            },
            "required": ["action"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::FilesystemRead(None)
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let action = params
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'action' is required".to_string()))?;

        match action {
            "index" => self.action_index(&params).await,
            "search" => self.action_search(&params).await,
            "symbols" => self.action_symbols(&params).await,
            "dependencies" => self.action_dependencies(&params).await,
            _ => Err(LlmError::ToolCall(format!(
                "Unknown code_search action: '{action}'. Valid: index, search, symbols, dependencies"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_metadata() {
        let tool = CodeSearchTool::new();
        assert_eq!(tool.name(), "code_search");
        assert!(!tool.description().is_empty());
        assert!(tool.parameters_schema().is_object());
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(CodeSearchTool::detect_language(Path::new("main.rs")), Some("rust"));
        assert_eq!(CodeSearchTool::detect_language(Path::new("app.ts")), Some("typescript"));
        assert_eq!(CodeSearchTool::detect_language(Path::new("index.js")), Some("javascript"));
        assert_eq!(CodeSearchTool::detect_language(Path::new("main.py")), Some("python"));
        assert_eq!(CodeSearchTool::detect_language(Path::new("main.go")), Some("go"));
        assert_eq!(CodeSearchTool::detect_language(Path::new("readme.md")), None);
    }

    #[test]
    fn test_extract_rust_symbols() {
        let code = r#"
pub struct MyStruct {
    field: String,
}

pub enum MyEnum {
    A, B,
}

pub trait MyTrait {
    fn do_thing(&self);
}

pub fn my_function() {}

pub async fn async_fn() {}

const MAX: usize = 10;

pub mod submodule {}
"#;
        let symbols = CodeSearchTool::extract_symbols(code, "rust", "lib.rs");
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"MyStruct"), "symbols: {:?}", names);
        assert!(names.contains(&"MyEnum"), "symbols: {:?}", names);
        assert!(names.contains(&"MyTrait"), "symbols: {:?}", names);
        assert!(names.contains(&"my_function"), "symbols: {:?}", names);
        assert!(names.contains(&"async_fn"), "symbols: {:?}", names);
        assert!(names.contains(&"MAX"), "symbols: {:?}", names);
        assert!(names.contains(&"submodule"), "symbols: {:?}", names);
    }

    #[test]
    fn test_extract_typescript_symbols() {
        let code = r#"
export function fetchData() {}
export class UserService {}
export interface Config {}
export type ID = string;
export enum Status { Active, Inactive }
export const MAX_RETRIES = 3;
"#;
        let symbols = CodeSearchTool::extract_symbols(code, "typescript", "api.ts");
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"fetchData"), "symbols: {:?}", names);
        assert!(names.contains(&"UserService"), "symbols: {:?}", names);
        assert!(names.contains(&"Config"), "symbols: {:?}", names);
        assert!(names.contains(&"ID"), "symbols: {:?}", names);
        assert!(names.contains(&"Status"), "symbols: {:?}", names);
        assert!(names.contains(&"MAX_RETRIES"), "symbols: {:?}", names);
    }

    #[test]
    fn test_extract_python_symbols() {
        let code = r#"
class MyClass:
    def method(self):
        pass

def standalone():
    pass

MAX_SIZE = 100
"#;
        let symbols = CodeSearchTool::extract_symbols(code, "python", "app.py");
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"MyClass"), "symbols: {:?}", names);
        assert!(names.contains(&"method"), "symbols: {:?}", names);
        assert!(names.contains(&"standalone"), "symbols: {:?}", names);
        assert!(names.contains(&"MAX_SIZE"), "symbols: {:?}", names);
    }

    #[test]
    fn test_extract_rust_imports() {
        let code = r#"
use std::collections::HashMap;
use serde_json::{json, Value};
use crate::error::Result;
"#;
        let imports = CodeSearchTool::extract_imports(code, "rust");
        assert_eq!(imports.len(), 3);
        assert!(imports[0].module.contains("std::collections::HashMap"));
    }

    #[test]
    fn test_extract_python_imports() {
        let code = r#"
from os import path
import sys, json
from typing import Optional, List
"#;
        let imports = CodeSearchTool::extract_imports(code, "python");
        assert!(imports.len() >= 2, "got {} imports", imports.len());
    }

    #[test]
    fn test_should_ignore() {
        assert!(CodeSearchTool::should_ignore(Path::new("/project/node_modules/pkg")));
        assert!(CodeSearchTool::should_ignore(Path::new("/project/target/debug")));
        assert!(CodeSearchTool::should_ignore(Path::new("/project/.git/objects")));
        assert!(!CodeSearchTool::should_ignore(Path::new("/project/src/main.rs")));
    }

    #[tokio::test]
    async fn test_search_no_index() {
        let tool = CodeSearchTool::new();
        let result = tool.execute(json!({ "action": "search", "query": "foo" })).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No index"));
    }

    #[tokio::test]
    async fn test_unknown_action() {
        let tool = CodeSearchTool::new();
        let result = tool.execute(json!({ "action": "invalid" })).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_symbols_nonexistent_file() {
        let tool = CodeSearchTool::new();
        let result = tool
            .execute(json!({ "action": "symbols", "file": "/nonexistent/file.rs" }))
            .await;
        assert!(result.is_err());
    }
}
