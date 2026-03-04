//! Filesystem tools for reading, writing, editing, and listing files.
//!
//! Gated by `filesystem.read` and `filesystem.write` permissions.

use std::sync::OnceLock;

use async_trait::async_trait;
use omni_permissions::capability::Capability;
use tokio::io::{AsyncBufReadExt, BufReader};

use super::util::floor_char_boundary;
use super::NativeTool;
use crate::error::{LlmError, Result};

/// Maximum file size for read/edit/patch operations (10 MB).
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Check file size before reading, returning an error if too large.
async fn check_file_size(path: &str) -> Result<()> {
    let metadata = tokio::fs::metadata(path)
        .await
        .map_err(|e| LlmError::ToolCall(format!("Failed to stat '{}': {}", path, e)))?;
    let size = metadata.len();
    if size > MAX_FILE_SIZE {
        return Err(LlmError::ToolCall(format!(
            "File '{}' is too large ({} bytes, max {} bytes). Use offset/limit parameters for large files.",
            path, size, MAX_FILE_SIZE
        )));
    }
    Ok(())
}

/// Native tool for reading file contents.
pub struct ReadFileTool;

#[async_trait]
impl NativeTool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file. Returns the file content as text. \
         For binary files, content is returned as base64."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute or relative path to the file"
                },
                "offset": {
                    "type": "integer",
                    "description": "Line number to start reading from (1-based, optional)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to read (optional, defaults to all)"
                }
            },
            "required": ["path"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::FilesystemRead(None)
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let path = params["path"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'path' parameter is required".to_string()))?;

        let offset = params["offset"].as_u64().map(|o| o.saturating_sub(1) as usize);
        let limit = params["limit"].as_u64().map(|l| l as usize);

        let has_pagination = offset.is_some() || limit.is_some();

        if has_pagination {
            // Streaming pagination -- read only the lines we need, no full-file load.
            // This works on files of any size without OOM.
            let file = tokio::fs::File::open(path)
                .await
                .map_err(|e| LlmError::ToolCall(format!("Failed to open '{}': {}", path, e)))?;
            let reader = BufReader::new(file);
            let mut lines_stream = reader.lines();

            let start = offset.unwrap_or(0);
            let take = limit.unwrap_or(usize::MAX);

            // Skip lines before offset
            let mut line_count: usize = 0;
            let mut skipped: usize = 0;
            let mut selected = Vec::new();

            while let Some(line) = lines_stream.next_line().await.map_err(|e| {
                LlmError::ToolCall(format!("Failed to read '{}': {}", path, e))
            })? {
                if skipped < start {
                    skipped += 1;
                    line_count += 1;
                    continue;
                }
                if selected.len() < take {
                    selected.push(format!("{:>4}\t{}", line_count + 1, line));
                }
                line_count += 1;
                // Once we have enough lines, count remaining for total
                if selected.len() >= take {
                    // Count remaining lines without storing them
                    while lines_stream.next_line().await.map_err(|e| {
                        LlmError::ToolCall(format!("Failed to read '{}': {}", path, e))
                    })?.is_some() {
                        line_count += 1;
                    }
                    break;
                }
            }

            if selected.is_empty() && start > 0 {
                return Ok(serde_json::json!({
                    "content": "",
                    "total_lines": line_count,
                    "note": format!("Offset {} exceeds file length ({} lines)", start + 1, line_count)
                }));
            }

            let shown = selected.len();
            Ok(serde_json::json!({
                "content": selected.join("\n"),
                "total_lines": line_count,
                "lines_shown": shown,
            }))
        } else {
            // Full-file read with size check
            check_file_size(path).await?;

            let content = tokio::fs::read(path)
                .await
                .map_err(|e| LlmError::ToolCall(format!("Failed to read '{}': {}", path, e)))?;

            match String::from_utf8(content) {
                Ok(text) => {
                    let lines: Vec<&str> = text.lines().collect();
                    let selected: Vec<String> = lines
                        .iter()
                        .enumerate()
                        .map(|(i, line)| format!("{:>4}\t{}", i + 1, line))
                        .collect();

                    Ok(serde_json::json!({
                        "content": selected.join("\n"),
                        "total_lines": lines.len(),
                        "lines_shown": lines.len(),
                    }))
                }
                Err(e) => {
                    // Binary file -- recover the original bytes and return base64
                    let bytes = e.into_bytes();
                    let b64 = base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        &bytes,
                    );
                    Ok(serde_json::json!({
                        "content_base64": b64,
                        "size_bytes": bytes.len(),
                        "encoding": "base64",
                    }))
                }
            }
        }
    }
}

/// Native tool for writing/creating files.
pub struct WriteFileTool;

#[async_trait]
impl NativeTool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Create a new file or completely overwrite an existing file. Creates parent directories \
         automatically. For partial modifications to existing files, prefer edit_file or apply_patch."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::FilesystemWrite(None)
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let path = params["path"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'path' parameter is required".to_string()))?;
        let content = params["content"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'content' parameter is required".to_string()))?;

        // Create parent directories if needed
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| LlmError::ToolCall(format!("Failed to create directories: {e}")))?;
            }
        }

        tokio::fs::write(path, content)
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to write '{}': {}", path, e)))?;

        Ok(serde_json::json!({
            "success": true,
            "path": path,
            "bytes_written": content.len(),
        }))
    }
}

/// Native tool for editing files with find/replace.
pub struct EditFileTool;

#[async_trait]
impl NativeTool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing an exact string match with new content. \
         The old_string must be unique in the file to avoid ambiguity."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "old_string": {
                    "type": "string",
                    "description": "The exact text to find and replace (must be unique in file)"
                },
                "new_string": {
                    "type": "string",
                    "description": "The replacement text"
                }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::FilesystemWrite(None)
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let path = params["path"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'path' parameter is required".to_string()))?;
        let old_string = params["old_string"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'old_string' parameter is required".to_string()))?;
        let new_string = params["new_string"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'new_string' parameter is required".to_string()))?;

        check_file_size(path).await?;

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to read '{}': {}", path, e)))?;

        let count = content.matches(old_string).count();
        if count == 0 {
            return Err(LlmError::ToolCall(format!(
                "old_string not found in '{}'",
                path
            )));
        }
        if count > 1 {
            return Err(LlmError::ToolCall(format!(
                "old_string matches {} times in '{}' -- must be unique. Provide more context.",
                count, path
            )));
        }

        let new_content = content.replacen(old_string, new_string, 1);

        tokio::fs::write(path, &new_content)
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to write '{}': {}", path, e)))?;

        Ok(serde_json::json!({
            "success": true,
            "path": path,
        }))
    }
}

/// Native tool for listing directory contents.
pub struct ListFilesTool;

#[async_trait]
impl NativeTool for ListFilesTool {
    fn name(&self) -> &str {
        "list_files"
    }

    fn description(&self) -> &str {
        "List files and directories in a given path. Returns names, sizes, and types."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to list"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "Whether to list recursively (default: false, max depth 3)"
                }
            },
            "required": ["path"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::FilesystemRead(None)
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let path = params["path"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'path' parameter is required".to_string()))?;
        let recursive = params["recursive"].as_bool().unwrap_or(false);

        let max_entries = 500;
        let max_depth = if recursive { 3 } else { 1 };
        let mut entries = Vec::new();

        list_dir_recursive(path, 0, max_depth, max_entries, &mut entries).await?;

        Ok(serde_json::json!({
            "path": path,
            "entries": entries,
            "total": entries.len(),
        }))
    }
}

async fn list_dir_recursive(
    path: &str,
    depth: usize,
    max_depth: usize,
    max_entries: usize,
    entries: &mut Vec<serde_json::Value>,
) -> Result<()> {
    if depth >= max_depth || entries.len() >= max_entries {
        return Ok(());
    }

    let mut dir = tokio::fs::read_dir(path)
        .await
        .map_err(|e| LlmError::ToolCall(format!("Failed to read directory '{}': {}", path, e)))?;

    while let Ok(Some(entry)) = dir.next_entry().await {
        if entries.len() >= max_entries {
            break;
        }

        let file_name = entry.file_name().to_string_lossy().to_string();
        let file_type = entry.file_type().await.ok();
        let metadata = entry.metadata().await.ok();

        let is_dir = file_type.as_ref().map(|ft| ft.is_dir()).unwrap_or(false);
        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

        let indent = "  ".repeat(depth);
        let type_str = if is_dir { "dir" } else { "file" };

        entries.push(serde_json::json!({
            "name": format!("{}{}", indent, file_name),
            "type": type_str,
            "size": size,
            "path": entry.path().display().to_string(),
        }));

        if is_dir && depth + 1 < max_depth {
            let sub_path = entry.path().display().to_string();
            // Box the future to avoid recursive async type issues
            Box::pin(list_dir_recursive(
                &sub_path,
                depth + 1,
                max_depth,
                max_entries,
                entries,
            ))
            .await?;
        }
    }

    Ok(())
}

/// Native tool for applying unified diff patches to files.
pub struct ApplyPatchTool;

#[async_trait]
impl NativeTool for ApplyPatchTool {
    fn name(&self) -> &str {
        "apply_patch"
    }

    fn description(&self) -> &str {
        "Apply a unified diff patch to a file. Supports multi-hunk patches for making \
         multiple changes in a single operation. More powerful than edit_file for complex edits."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to patch"
                },
                "patch": {
                    "type": "string",
                    "description": "Unified diff patch content (output of `diff -u` or similar)"
                }
            },
            "required": ["path", "patch"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::FilesystemWrite(None)
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let path = params["path"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'path' parameter is required".to_string()))?;
        let patch = params["patch"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'patch' parameter is required".to_string()))?;

        check_file_size(path).await?;

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to read '{}': {}", path, e)))?;

        let lines: Vec<&str> = content.lines().collect();
        let hunks = parse_unified_diff(patch)?;

        if hunks.is_empty() {
            return Err(LlmError::ToolCall("No valid hunks found in patch".to_string()));
        }

        let mut result_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        let mut offset: i64 = 0;

        for hunk in &hunks {
            let start = ((hunk.old_start as i64 - 1) + offset) as usize;

            // Verify context lines match
            let mut old_idx = start;
            for line in &hunk.old_lines {
                if old_idx >= result_lines.len() || result_lines[old_idx] != *line {
                    return Err(LlmError::ToolCall(format!(
                        "Context mismatch at line {}: expected '{}', got '{}'",
                        old_idx + 1,
                        line,
                        result_lines.get(old_idx).unwrap_or(&String::new())
                    )));
                }
                old_idx += 1;
            }

            // Remove old lines and insert new lines
            let remove_count = hunk.old_lines.len();
            result_lines.splice(
                start..start + remove_count,
                hunk.new_lines.iter().cloned(),
            );

            offset += hunk.new_lines.len() as i64 - hunk.old_lines.len() as i64;
        }

        let new_content = result_lines.join("\n");
        let new_content = if content.ends_with('\n') && !new_content.ends_with('\n') {
            format!("{}\n", new_content)
        } else {
            new_content
        };

        tokio::fs::write(path, &new_content)
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to write '{}': {}", path, e)))?;

        Ok(serde_json::json!({
            "success": true,
            "path": path,
            "hunks_applied": hunks.len(),
        }))
    }
}

struct DiffHunk {
    old_start: usize,
    old_lines: Vec<String>,
    new_lines: Vec<String>,
}

fn parse_unified_diff(patch: &str) -> Result<Vec<DiffHunk>> {
    let mut hunks = Vec::new();
    let lines: Vec<&str> = patch.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        if lines[i].starts_with("@@") {
            let old_start = parse_hunk_header(lines[i])?;
            i += 1;
            let mut old_lines = Vec::new();
            let mut new_lines = Vec::new();

            while i < lines.len() && !lines[i].starts_with("@@") {
                let line = lines[i];
                if let Some(stripped) = line.strip_prefix('-') {
                    old_lines.push(stripped.to_string());
                } else if let Some(stripped) = line.strip_prefix('+') {
                    new_lines.push(stripped.to_string());
                } else if let Some(stripped) = line.strip_prefix(' ') {
                    old_lines.push(stripped.to_string());
                    new_lines.push(stripped.to_string());
                } else if !line.starts_with('\\') && !line.starts_with("---") && !line.starts_with("+++") {
                    old_lines.push(line.to_string());
                    new_lines.push(line.to_string());
                }
                i += 1;
            }

            hunks.push(DiffHunk { old_start, old_lines, new_lines });
        } else {
            i += 1;
        }
    }
    Ok(hunks)
}

fn parse_hunk_header(header: &str) -> Result<usize> {
    static HUNK_RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = HUNK_RE.get_or_init(|| regex::Regex::new(r"@@ -(\d+)").unwrap());
    let caps = re
        .captures(header)
        .ok_or_else(|| LlmError::ToolCall(format!("Invalid hunk header: {}", header)))?;
    let line_num = caps[1]
        .parse::<usize>()
        .map_err(|e| LlmError::ToolCall(format!("Invalid line number in hunk: {e}")))?;
    if line_num == 0 {
        return Err(LlmError::ToolCall(
            "Invalid hunk header: line number must be >= 1".to_string(),
        ));
    }
    Ok(line_num)
}

/// Native tool for searching file contents with regex.
pub struct GrepSearchTool;

#[async_trait]
impl NativeTool for GrepSearchTool {
    fn name(&self) -> &str {
        "grep_search"
    }

    fn description(&self) -> &str {
        "Search file contents using a regex pattern. Returns matching lines with file paths \
         and line numbers. Useful for finding code, text patterns, and references across files."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search in (defaults to current directory)"
                },
                "glob": {
                    "type": "string",
                    "description": "File glob pattern to filter files (e.g. '*.rs', '*.py')"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Case-insensitive search (default: false)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results (default: 100)"
                }
            },
            "required": ["pattern"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::FilesystemRead(None)
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let pattern_str = params["pattern"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'pattern' parameter is required".to_string()))?;
        let search_path = params["path"].as_str().unwrap_or(".");
        let glob_pattern = params["glob"].as_str();
        let case_insensitive = params["case_insensitive"].as_bool().unwrap_or(false);
        let max_results = params["max_results"].as_u64().unwrap_or(100) as usize;

        let re = if case_insensitive {
            regex::RegexBuilder::new(pattern_str)
                .case_insensitive(true)
                .build()
        } else {
            regex::Regex::new(pattern_str)
        }
        .map_err(|e| LlmError::ToolCall(format!("Invalid regex pattern: {e}")))?;

        let glob_re = glob_pattern
            .map(|g| glob_to_regex(g))
            .transpose()
            .map_err(|e| LlmError::ToolCall(format!("Invalid glob pattern: {e}")))?;

        let mut matches = Vec::new();
        let path = std::path::Path::new(search_path);

        if path.is_file() {
            grep_file(path, &re, max_results, &mut matches).await?;
        } else if path.is_dir() {
            grep_dir_recursive(path, &re, glob_re.as_ref(), max_results, 0, 10, &mut matches)
                .await?;
        } else {
            return Err(LlmError::ToolCall(format!("Path '{}' not found", search_path)));
        }

        let total = matches.len();
        Ok(serde_json::json!({
            "matches": matches,
            "total": total,
            "pattern": pattern_str,
        }))
    }
}

async fn grep_file(
    path: &std::path::Path,
    re: &regex::Regex,
    max_results: usize,
    matches: &mut Vec<serde_json::Value>,
) -> Result<()> {
    if matches.len() >= max_results {
        return Ok(());
    }
    let content = match tokio::fs::read_to_string(path).await {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };
    for (line_num, line) in content.lines().enumerate() {
        if matches.len() >= max_results {
            break;
        }
        if re.is_match(line) {
            let display = if line.len() > 200 {
                let end = floor_char_boundary(line, 200);
                format!("{}...", &line[..end])
            } else {
                line.to_string()
            };
            matches.push(serde_json::json!({
                "file": path.display().to_string(),
                "line": line_num + 1,
                "content": display,
            }));
        }
    }
    Ok(())
}

async fn grep_dir_recursive(
    dir: &std::path::Path,
    re: &regex::Regex,
    glob_re: Option<&regex::Regex>,
    max_results: usize,
    depth: usize,
    max_depth: usize,
    matches: &mut Vec<serde_json::Value>,
) -> Result<()> {
    if depth >= max_depth || matches.len() >= max_results {
        return Ok(());
    }
    let mut entries = match tokio::fs::read_dir(dir).await {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        if matches.len() >= max_results {
            break;
        }
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.starts_with('.') || file_name == "node_modules" || file_name == "target" {
            continue;
        }
        if path.is_dir() {
            Box::pin(grep_dir_recursive(
                &path, re, glob_re, max_results, depth + 1, max_depth, matches,
            ))
            .await?;
        } else if path.is_file() {
            if let Some(glob_re) = glob_re {
                if !glob_re.is_match(&file_name) {
                    continue;
                }
            }
            grep_file(&path, re, max_results, matches).await?;
        }
    }
    Ok(())
}

fn glob_to_regex(glob: &str) -> std::result::Result<regex::Regex, String> {
    let mut regex_str = String::from("^");
    for ch in glob.chars() {
        match ch {
            '*' => regex_str.push_str(".*"),
            '?' => regex_str.push('.'),
            // Escape all regex metacharacters so they match literally
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                regex_str.push('\\');
                regex_str.push(ch);
            }
            c => regex_str.push(c),
        }
    }
    regex_str.push('$');
    regex::Regex::new(&regex_str).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_file_schema() {
        let tool = ReadFileTool;
        assert_eq!(tool.name(), "read_file");
        assert_eq!(tool.required_capability().capability_key(), "filesystem.read");
    }

    #[test]
    fn test_write_file_schema() {
        let tool = WriteFileTool;
        assert_eq!(tool.name(), "write_file");
        assert_eq!(tool.required_capability().capability_key(), "filesystem.write");
    }

    #[test]
    fn test_edit_file_schema() {
        let tool = EditFileTool;
        assert_eq!(tool.name(), "edit_file");
        assert_eq!(tool.required_capability().capability_key(), "filesystem.write");
    }

    #[test]
    fn test_list_files_schema() {
        let tool = ListFilesTool;
        assert_eq!(tool.name(), "list_files");
        assert_eq!(tool.required_capability().capability_key(), "filesystem.read");
    }

    #[tokio::test]
    async fn test_read_file() {
        let tool = ReadFileTool;
        let tmp = std::env::temp_dir().join("omni_test_read.txt");
        std::fs::write(&tmp, "line1\nline2\nline3\n").unwrap();

        let result = tool
            .execute(serde_json::json!({"path": tmp.display().to_string()}))
            .await
            .unwrap();

        assert_eq!(result["total_lines"], 3);
        let content = result["content"].as_str().unwrap();
        assert!(content.contains("line1"));
        assert!(content.contains("line2"));

        std::fs::remove_file(&tmp).ok();
    }

    #[tokio::test]
    async fn test_read_file_with_offset() {
        let tool = ReadFileTool;
        let tmp = std::env::temp_dir().join("omni_test_read_offset.txt");
        std::fs::write(&tmp, "line1\nline2\nline3\nline4\n").unwrap();

        let result = tool
            .execute(serde_json::json!({"path": tmp.display().to_string(), "offset": 2, "limit": 2}))
            .await
            .unwrap();

        assert_eq!(result["lines_shown"], 2);
        let content = result["content"].as_str().unwrap();
        assert!(content.contains("line2"));
        assert!(content.contains("line3"));
        assert!(!content.contains("line1"));

        std::fs::remove_file(&tmp).ok();
    }

    #[tokio::test]
    async fn test_write_and_read_file() {
        let tool_write = WriteFileTool;
        let tool_read = ReadFileTool;
        let tmp = std::env::temp_dir().join("omni_test_write.txt");

        let write_result = tool_write
            .execute(serde_json::json!({
                "path": tmp.display().to_string(),
                "content": "Hello, Omni!"
            }))
            .await
            .unwrap();
        assert_eq!(write_result["success"], true);

        let read_result = tool_read
            .execute(serde_json::json!({"path": tmp.display().to_string()}))
            .await
            .unwrap();
        let content = read_result["content"].as_str().unwrap();
        assert!(content.contains("Hello, Omni!"));

        std::fs::remove_file(&tmp).ok();
    }

    #[tokio::test]
    async fn test_edit_file() {
        let tool = EditFileTool;
        let tmp = std::env::temp_dir().join("omni_test_edit.txt");
        std::fs::write(&tmp, "Hello, World!").unwrap();

        let result = tool
            .execute(serde_json::json!({
                "path": tmp.display().to_string(),
                "old_string": "World",
                "new_string": "Omni"
            }))
            .await
            .unwrap();
        assert_eq!(result["success"], true);

        let content = std::fs::read_to_string(&tmp).unwrap();
        assert_eq!(content, "Hello, Omni!");

        std::fs::remove_file(&tmp).ok();
    }

    #[tokio::test]
    async fn test_edit_file_not_found() {
        let tool = EditFileTool;
        let tmp = std::env::temp_dir().join("omni_test_edit_notfound.txt");
        std::fs::write(&tmp, "Hello").unwrap();

        let result = tool
            .execute(serde_json::json!({
                "path": tmp.display().to_string(),
                "old_string": "MISSING_STRING",
                "new_string": "replacement"
            }))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));

        std::fs::remove_file(&tmp).ok();
    }

    #[tokio::test]
    async fn test_list_files() {
        let tool = ListFilesTool;
        let tmp_dir = std::env::temp_dir().join("omni_test_list");
        std::fs::create_dir_all(&tmp_dir).ok();
        std::fs::write(tmp_dir.join("a.txt"), "a").ok();
        std::fs::write(tmp_dir.join("b.txt"), "b").ok();

        let result = tool
            .execute(serde_json::json!({"path": tmp_dir.display().to_string()}))
            .await
            .unwrap();

        let total = result["total"].as_u64().unwrap();
        assert!(total >= 2, "Expected at least 2 entries, got {total}");

        std::fs::remove_dir_all(&tmp_dir).ok();
    }

    #[test]
    fn test_apply_patch_schema() {
        let tool = ApplyPatchTool;
        assert_eq!(tool.name(), "apply_patch");
        assert_eq!(tool.required_capability().capability_key(), "filesystem.write");
    }

    #[tokio::test]
    async fn test_apply_patch() {
        let tool = ApplyPatchTool;
        let tmp = std::env::temp_dir().join("omni_test_patch.txt");
        std::fs::write(&tmp, "line1\nline2\nline3\n").unwrap();

        let patch = "@@ -1,3 +1,3 @@\n line1\n-line2\n+line2_modified\n line3";

        let result = tool
            .execute(serde_json::json!({
                "path": tmp.display().to_string(),
                "patch": patch,
            }))
            .await
            .unwrap();

        assert_eq!(result["success"], true);
        assert_eq!(result["hunks_applied"], 1);

        let content = std::fs::read_to_string(&tmp).unwrap();
        assert!(content.contains("line2_modified"));
        assert!(!content.contains("\nline2\n"));

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_grep_search_schema() {
        let tool = GrepSearchTool;
        assert_eq!(tool.name(), "grep_search");
        assert_eq!(tool.required_capability().capability_key(), "filesystem.read");
    }

    #[tokio::test]
    async fn test_grep_search_file() {
        let tool = GrepSearchTool;
        let tmp = std::env::temp_dir().join("omni_test_grep.txt");
        std::fs::write(&tmp, "hello world\nfoo bar\nhello again\n").unwrap();

        let result = tool
            .execute(serde_json::json!({
                "pattern": "hello",
                "path": tmp.display().to_string(),
            }))
            .await
            .unwrap();

        assert_eq!(result["total"], 2);
        let matches = result["matches"].as_array().unwrap();
        assert_eq!(matches[0]["line"], 1);
        assert_eq!(matches[1]["line"], 3);

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_glob_to_regex() {
        let re = glob_to_regex("*.rs").unwrap();
        assert!(re.is_match("main.rs"));
        assert!(!re.is_match("main.py"));

        let re = glob_to_regex("test_?.txt").unwrap();
        assert!(re.is_match("test_a.txt"));
        assert!(!re.is_match("test_ab.txt"));
    }

    #[test]
    fn test_glob_to_regex_metacharacters() {
        // + should be escaped and match literally
        let re = glob_to_regex("foo+bar.txt").unwrap();
        assert!(re.is_match("foo+bar.txt"));
        assert!(!re.is_match("foobar.txt")); // without escaping, + makes 'o' optional

        // Parens should be escaped
        let re = glob_to_regex("file(1).txt").unwrap();
        assert!(re.is_match("file(1).txt"));

        // Brackets, braces, caret, dollar, pipe
        let re = glob_to_regex("a[b]{c}^d$e|f.txt").unwrap();
        assert!(re.is_match("a[b]{c}^d$e|f.txt"));
    }

    #[tokio::test]
    async fn test_apply_patch_zero_start_rejected() {
        let tool = ApplyPatchTool;
        let tmp = std::env::temp_dir().join("omni_test_patch_zero.txt");
        std::fs::write(&tmp, "line1\nline2\n").unwrap();

        // @@ -0 is invalid -- line numbers must be >= 1
        let patch = "@@ -0,0 +1,1 @@\n+inserted";
        let result = tool
            .execute(serde_json::json!({
                "path": tmp.display().to_string(),
                "patch": patch,
            }))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be >= 1"));

        std::fs::remove_file(&tmp).ok();
    }

    #[tokio::test]
    async fn test_read_file_streaming_pagination() {
        let tool = ReadFileTool;
        let tmp = std::env::temp_dir().join("omni_test_read_stream.txt");
        // Create a file with numbered lines
        let content: String = (1..=100).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");
        std::fs::write(&tmp, &content).unwrap();

        // Read lines 50-54 using streaming pagination
        let result = tool
            .execute(serde_json::json!({
                "path": tmp.display().to_string(),
                "offset": 50,
                "limit": 5,
            }))
            .await
            .unwrap();

        assert_eq!(result["lines_shown"], 5);
        assert_eq!(result["total_lines"], 100);
        let text = result["content"].as_str().unwrap();
        assert!(text.contains("line 50"));
        assert!(text.contains("line 54"));
        assert!(!text.contains("line 49"));
        assert!(!text.contains("line 55"));

        std::fs::remove_file(&tmp).ok();
    }
}
