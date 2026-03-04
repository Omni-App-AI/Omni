//! Memory tools for persistent knowledge storage and retrieval.
//!
//! Gated by `storage.persistent` permission.

use async_trait::async_trait;
use omni_permissions::capability::Capability;
use tokio::io::AsyncWriteExt;

use super::util::floor_char_boundary;
use super::NativeTool;
use crate::error::{LlmError, Result};

/// Default memory directory relative to current working directory.
const MEMORY_DIR: &str = "memory";
const MEMORY_FILE: &str = "MEMORY.md";

fn memory_base_dir() -> std::path::PathBuf {
    std::env::var("OMNI_MEMORY_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}

/// Native tool for saving information to persistent memory.
pub struct MemorySaveTool {
    /// Override base directory for testing. If None, uses OMNI_MEMORY_DIR env var.
    pub(crate) base_dir_override: Option<std::path::PathBuf>,
}

impl MemorySaveTool {
    fn base_dir(&self) -> std::path::PathBuf {
        self.base_dir_override.clone().unwrap_or_else(memory_base_dir)
    }
}

#[async_trait]
impl NativeTool for MemorySaveTool {
    fn name(&self) -> &str {
        "memory_save"
    }

    fn description(&self) -> &str {
        "Save information to persistent memory. Content is appended to memory files \
         (MEMORY.md or memory/<category>.md) with timestamps. Use this to remember \
         important facts, decisions, preferences, or context across sessions."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "The information to save to memory"
                },
                "tags": {
                    "type": "string",
                    "description": "Comma-separated tags for categorization"
                },
                "category": {
                    "type": "string",
                    "description": "Category file name (saves to memory/<category>.md instead of MEMORY.md)"
                }
            },
            "required": ["content"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::StoragePersistent(None)
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let content = params["content"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'content' parameter is required".to_string()))?;
        let tags = params["tags"].as_str().unwrap_or("");
        let category = params["category"].as_str();

        let base = self.base_dir();
        let file_path = if let Some(cat) = category {
            let dir = base.join(MEMORY_DIR);
            tokio::fs::create_dir_all(&dir)
                .await
                .map_err(|e| LlmError::ToolCall(format!("Failed to create memory dir: {e}")))?;
            dir.join(format!("{}.md", cat))
        } else {
            base.join(MEMORY_FILE)
        };

        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        let entry_id = uuid::Uuid::new_v4().to_string()[..8].to_string();

        let mut entry = format!("\n## [{entry_id}] {timestamp}\n\n{content}\n");
        if !tags.is_empty() {
            entry.push_str(&format!("\n_Tags: {tags}_\n"));
        }

        let header = if let Some(cat) = category {
            format!("# Memory: {}\n", cat)
        } else {
            "# Memory\n".to_string()
        };

        // Atomic create-if-not-exists + append -- no TOCTOU race.
        // OpenOptions::create(true).append(true) atomically creates the file
        // if it doesn't exist. We check if the file is empty to write the header.
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to open memory file: {e}")))?;

        let meta = file.metadata()
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to stat memory file: {e}")))?;
        if meta.len() == 0 {
            file.write_all(header.as_bytes())
                .await
                .map_err(|e| LlmError::ToolCall(format!("Failed to write memory header: {e}")))?;
        }

        file.write_all(entry.as_bytes())
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to write memory: {e}")))?;

        Ok(serde_json::json!({
            "success": true,
            "path": file_path.display().to_string(),
            "entry_id": entry_id,
        }))
    }
}

/// Native tool for searching persistent memory.
pub struct MemorySearchTool {
    /// Override base directory for testing. If None, uses OMNI_MEMORY_DIR env var.
    pub(crate) base_dir_override: Option<std::path::PathBuf>,
}

impl MemorySearchTool {
    fn base_dir(&self) -> std::path::PathBuf {
        self.base_dir_override.clone().unwrap_or_else(memory_base_dir)
    }
}

#[async_trait]
impl NativeTool for MemorySearchTool {
    fn name(&self) -> &str {
        "memory_search"
    }

    fn description(&self) -> &str {
        "Search persistent memory files for relevant information. Uses keyword matching \
         to find entries across MEMORY.md and memory/*.md files."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query (keywords)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum results to return (default: 10)"
                },
                "category": {
                    "type": "string",
                    "description": "Limit search to a specific category file"
                }
            },
            "required": ["query"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::StoragePersistent(None)
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let query = params["query"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'query' parameter is required".to_string()))?;
        let max_results = params["max_results"].as_u64().unwrap_or(10) as usize;
        let category = params["category"].as_str();

        let base = self.base_dir();
        let keywords: Vec<&str> = query.split_whitespace().collect();

        let mut all_results = Vec::new();

        // Search specific category or all files (all checks use async I/O)
        if let Some(cat) = category {
            let path = base.join(MEMORY_DIR).join(format!("{}.md", cat));
            if tokio::fs::try_exists(&path).await.unwrap_or(false) {
                search_memory_file(&path, &keywords, &mut all_results).await;
            }
        } else {
            // Search MEMORY.md
            let main_path = base.join(MEMORY_FILE);
            if tokio::fs::try_exists(&main_path).await.unwrap_or(false) {
                search_memory_file(&main_path, &keywords, &mut all_results).await;
            }

            // Search memory/*.md
            let mem_dir = base.join(MEMORY_DIR);
            let is_dir = tokio::fs::metadata(&mem_dir)
                .await
                .map(|m| m.is_dir())
                .unwrap_or(false);
            if is_dir {
                if let Ok(mut entries) = tokio::fs::read_dir(&mem_dir).await {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let p = entry.path();
                        if p.extension().map(|e| e == "md").unwrap_or(false) {
                            search_memory_file(&p, &keywords, &mut all_results).await;
                        }
                    }
                }
            }
        }

        // Sort by score (descending)
        all_results.sort_by(|a, b| {
            let sa = a["score"].as_f64().unwrap_or(0.0);
            let sb = b["score"].as_f64().unwrap_or(0.0);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });

        all_results.truncate(max_results);
        let total = all_results.len();

        Ok(serde_json::json!({
            "results": all_results,
            "total": total,
            "query": query,
        }))
    }
}

async fn search_memory_file(
    path: &std::path::Path,
    keywords: &[&str],
    results: &mut Vec<serde_json::Value>,
) {
    let content = match tokio::fs::read_to_string(path).await {
        Ok(c) => c,
        Err(_) => return,
    };

    // Split into sections (by ## headers)
    let mut current_section = String::new();
    let mut section_start = 1;

    for (line_num, line) in content.lines().enumerate() {
        if line.starts_with("## ") && !current_section.is_empty() {
            score_section(
                path,
                section_start,
                &current_section,
                keywords,
                results,
            );
            current_section.clear();
            section_start = line_num + 1;
        }
        current_section.push_str(line);
        current_section.push('\n');
    }

    // Don't forget the last section
    if !current_section.is_empty() {
        score_section(path, section_start, &current_section, keywords, results);
    }
}

fn score_section(
    path: &std::path::Path,
    line: usize,
    section: &str,
    keywords: &[&str],
    results: &mut Vec<serde_json::Value>,
) {
    let lower = section.to_lowercase();
    let mut score = 0.0_f64;
    let mut matched = 0;

    for keyword in keywords {
        let kw_lower = keyword.to_lowercase();
        let count = lower.matches(&kw_lower).count();
        if count > 0 {
            matched += 1;
            score += count as f64;
        }
    }

    if matched == 0 {
        return;
    }

    // Bonus for matching more keywords
    score *= matched as f64 / keywords.len() as f64;

    // Truncate section preview (UTF-8 safe)
    let preview = if section.len() > 300 {
        let end = floor_char_boundary(section, 300);
        format!("{}...", &section[..end])
    } else {
        section.trim().to_string()
    };

    results.push(serde_json::json!({
        "path": path.display().to_string(),
        "line": line,
        "content": preview,
        "score": (score * 100.0).round() / 100.0,
    }));
}

/// Native tool for reading a specific memory file.
pub struct MemoryGetTool {
    /// Override base directory for testing. If None, uses OMNI_MEMORY_DIR env var.
    pub(crate) base_dir_override: Option<std::path::PathBuf>,
}

impl MemoryGetTool {
    fn base_dir(&self) -> std::path::PathBuf {
        self.base_dir_override.clone().unwrap_or_else(memory_base_dir)
    }
}

#[async_trait]
impl NativeTool for MemoryGetTool {
    fn name(&self) -> &str {
        "memory_get"
    }

    fn description(&self) -> &str {
        "Read the contents of a memory file (MEMORY.md or memory/<name>.md). \
         Optionally specify line range for large files."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Memory file path (e.g. 'MEMORY.md' or 'memory/projects.md'). Defaults to MEMORY.md"
                },
                "from": {
                    "type": "integer",
                    "description": "Starting line number (1-based)"
                },
                "lines": {
                    "type": "integer",
                    "description": "Number of lines to read"
                }
            },
            "required": []
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::StoragePersistent(None)
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let rel_path = params["path"].as_str().unwrap_or(MEMORY_FILE);
        let from = params["from"].as_u64().map(|f| f.saturating_sub(1) as usize);
        let line_count = params["lines"].as_u64().map(|l| l as usize);

        let base = self.base_dir();
        let file_path = base.join(rel_path);

        let content = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to read '{}': {}", rel_path, e)))?;

        let all_lines: Vec<&str> = content.lines().collect();
        let start = from.unwrap_or(0);
        let end = line_count
            .map(|l| (start + l).min(all_lines.len()))
            .unwrap_or(all_lines.len());

        if start >= all_lines.len() {
            return Ok(serde_json::json!({
                "content": "",
                "total_lines": all_lines.len(),
                "note": format!("Offset {} exceeds file length ({} lines)", start + 1, all_lines.len()),
            }));
        }

        let selected: String = all_lines[start..end].join("\n");

        Ok(serde_json::json!({
            "content": selected,
            "total_lines": all_lines.len(),
            "lines_shown": end - start,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_save_schema() {
        let tool = MemorySaveTool { base_dir_override: None };
        assert_eq!(tool.name(), "memory_save");
        assert_eq!(tool.required_capability().capability_key(), "storage.persistent");
    }

    #[test]
    fn test_memory_search_schema() {
        let tool = MemorySearchTool { base_dir_override: None };
        assert_eq!(tool.name(), "memory_search");
        assert_eq!(tool.required_capability().capability_key(), "storage.persistent");
    }

    #[test]
    fn test_memory_get_schema() {
        let tool = MemoryGetTool { base_dir_override: None };
        assert_eq!(tool.name(), "memory_get");
        assert_eq!(tool.required_capability().capability_key(), "storage.persistent");
    }

    #[tokio::test]
    async fn test_memory_save_and_get() {
        let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let tmp = std::env::temp_dir().join(format!("omni_mem_test_{id}"));
        tokio::fs::create_dir_all(&tmp).await.ok();

        let save = MemorySaveTool {
            base_dir_override: Some(tmp.clone()),
        };
        let result = save
            .execute(serde_json::json!({
                "content": "Test memory entry",
                "tags": "test,unit",
            }))
            .await
            .unwrap();
        assert_eq!(result["success"], true);

        // Verify by reading the file directly
        let mem_path = tmp.join("MEMORY.md");
        let content = tokio::fs::read_to_string(&mem_path).await.unwrap();
        assert!(content.contains("Test memory entry"));

        tokio::fs::remove_dir_all(&tmp).await.ok();
    }

    #[tokio::test]
    async fn test_memory_search() {
        let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let tmp = std::env::temp_dir().join(format!("omni_mem_search_{id}"));
        tokio::fs::create_dir_all(&tmp).await.ok();

        // Write test memory file
        let mem_file = tmp.join("MEMORY.md");
        tokio::fs::write(
            &mem_file,
            "# Memory\n\n## Entry 1\n\nRust is great for systems programming\n\n## Entry 2\n\nPython is popular for data science\n",
        )
        .await
        .unwrap();

        let search = MemorySearchTool {
            base_dir_override: Some(tmp.clone()),
        };
        let result = search
            .execute(serde_json::json!({"query": "Rust systems"}))
            .await
            .unwrap();

        let total = result["total"].as_u64().unwrap();
        assert!(total >= 1, "Expected at least 1 result, got {total}");

        tokio::fs::remove_dir_all(&tmp).await.ok();
    }
}
