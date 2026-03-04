//! Native git tool providing structured version control operations.
//!
//! Single multi-action tool (like `app_interact`) wrapping the git CLI
//! with parsed, structured JSON output for reliable LLM consumption.
//!
//! Gated by `vcs.operations` permission.

use async_trait::async_trait;
use omni_permissions::capability::Capability;
use regex::RegexSet;
use serde_json::{json, Value};
use tokio::process::Command;

use super::NativeTool;
use crate::error::{LlmError, Result};

/// Maximum output bytes from git commands before truncation.
const MAX_OUTPUT: usize = 50 * 1024;
/// Maximum number of log entries to return.
const MAX_LOG_COUNT: u32 = 100;

/// Secret patterns to scan for before committing.
/// Catches common API keys, tokens, private keys, and credentials.
fn secret_patterns() -> RegexSet {
    RegexSet::new([
        // AWS
        r"(?i)AKIA[0-9A-Z]{16}",
        r"(?i)aws[_\-]?secret[_\-]?access[_\-]?key\s*[=:]\s*\S+",
        // Private keys
        r"-----BEGIN\s+(RSA|DSA|EC|OPENSSH|PGP)\s+PRIVATE\s+KEY-----",
        // Generic API keys/tokens
        r#"(?i)(api[_\-]?key|api[_\-]?secret|access[_\-]?token|auth[_\-]?token|secret[_\-]?key)\s*[=:"]\s*[A-Za-z0-9/+=]{20,}"#,
        // GitHub/GitLab tokens
        r"gh[pousr]_[A-Za-z0-9_]{36,}",
        r"glpat-[A-Za-z0-9\-_]{20,}",
        // Slack tokens
        r"xox[baprs]-[A-Za-z0-9\-]+",
        // Generic password assignments
        r#"(?i)(password|passwd|pwd)\s*[=:"]\s*[^\s"']{8,}"#,
    ])
    .expect("secret patterns should compile")
}

/// Scan staged diff content for potential secrets.
fn scan_for_secrets(diff_content: &str) -> Vec<String> {
    let patterns = secret_patterns();
    let mut findings = Vec::new();

    for line in diff_content.lines() {
        // Only check added lines (lines starting with '+')
        if !line.starts_with('+') || line.starts_with("+++") {
            continue;
        }

        let matches: Vec<usize> = patterns.matches(line).into_iter().collect();
        if !matches.is_empty() {
            // Truncate the line to avoid leaking the secret in the error message
            let preview = if line.len() > 60 {
                format!("{}...", &line[..60])
            } else {
                line.to_string()
            };
            findings.push(preview);
        }
    }

    findings
}

pub struct GitTool;

impl GitTool {
    pub fn new() -> Self {
        Self
    }

    async fn run_git(args: &[&str], repo_path: Option<&str>) -> Result<String> {
        let mut cmd = Command::new("git");
        if let Some(path) = repo_path {
            cmd.arg("-C").arg(path);
        }
        cmd.args(args);

        let output = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to run git: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            return Err(LlmError::ToolCall(format!(
                "git {} failed (exit {}): {}",
                args.first().unwrap_or(&""),
                output.status.code().unwrap_or(-1),
                stderr.trim()
            )));
        }

        let mut result = stdout.to_string();
        if result.len() > MAX_OUTPUT {
            result.truncate(MAX_OUTPUT);
            result.push_str("\n... (output truncated)");
        }

        Ok(result)
    }

    async fn action_status(&self, params: &Value) -> Result<Value> {
        let repo_path = params.get("repo_path").and_then(|v| v.as_str());
        let output = Self::run_git(&["status", "--porcelain=v1", "-b"], repo_path).await?;

        let mut branch = String::new();
        let mut modified = Vec::new();
        let mut added = Vec::new();
        let mut deleted = Vec::new();
        let mut renamed = Vec::new();
        let mut untracked = Vec::new();
        let mut conflicted = Vec::new();

        for line in output.lines() {
            if line.starts_with("## ") {
                branch = line[3..].split("...").next().unwrap_or("").to_string();
                continue;
            }
            if line.len() < 3 {
                continue;
            }
            let status = &line[..2];
            let file = line[3..].trim().to_string();

            match status {
                "M " | " M" | "MM" => modified.push(file),
                "A " | "AM" => added.push(file),
                "D " | " D" => deleted.push(file),
                "R " => renamed.push(file),
                "??" => untracked.push(file),
                "UU" | "AA" | "DD" | "AU" | "UA" => conflicted.push(file),
                _ => modified.push(file),
            }
        }

        Ok(json!({
            "branch": branch,
            "modified": modified,
            "added": added,
            "deleted": deleted,
            "renamed": renamed,
            "untracked": untracked,
            "conflicted": conflicted,
            "clean": modified.is_empty() && added.is_empty() && deleted.is_empty()
                && renamed.is_empty() && untracked.is_empty() && conflicted.is_empty()
        }))
    }

    async fn action_diff(&self, params: &Value) -> Result<Value> {
        let repo_path = params.get("repo_path").and_then(|v| v.as_str());
        let staged = params.get("staged").and_then(|v| v.as_bool()).unwrap_or(false);
        let file = params.get("file").and_then(|v| v.as_str());

        let mut args = vec!["diff", "--stat", "--patch"];
        if staged {
            args.push("--cached");
        }
        if let Some(f) = file {
            args.push("--");
            args.push(f);
        }

        let output = Self::run_git(&args, repo_path).await?;
        Ok(json!({
            "diff": output,
            "staged": staged
        }))
    }

    async fn action_log(&self, params: &Value) -> Result<Value> {
        let repo_path = params.get("repo_path").and_then(|v| v.as_str());
        let count = params
            .get("count")
            .and_then(|v| v.as_u64())
            .unwrap_or(10)
            .min(MAX_LOG_COUNT as u64);
        let since = params.get("since").and_then(|v| v.as_str());
        let author = params.get("author").and_then(|v| v.as_str());

        let count_flag = format!("-{}", count);
        let mut args = vec!["log", count_flag.as_str(), "--format=%H%n%an%n%ae%n%aI%n%s%n---END---"];

        let since_flag;
        if let Some(s) = since {
            since_flag = format!("--since={}", s);
            args.push(&since_flag);
        }
        let author_flag;
        if let Some(a) = author {
            author_flag = format!("--author={}", a);
            args.push(&author_flag);
        }

        let output = Self::run_git(&args, repo_path).await?;

        let mut commits = Vec::new();
        let mut lines = output.lines().peekable();

        while lines.peek().is_some() {
            let hash = match lines.next() {
                Some(h) if !h.is_empty() => h.to_string(),
                _ => break,
            };
            let author_name = lines.next().unwrap_or("").to_string();
            let author_email = lines.next().unwrap_or("").to_string();
            let date = lines.next().unwrap_or("").to_string();
            let subject = lines.next().unwrap_or("").to_string();
            // consume the ---END--- separator
            let _ = lines.next();

            commits.push(json!({
                "hash": hash,
                "author": author_name,
                "email": author_email,
                "date": date,
                "subject": subject,
                "short_hash": &hash[..hash.len().min(8)]
            }));
        }

        Ok(json!({ "commits": commits, "count": commits.len() }))
    }

    async fn action_commit(&self, params: &Value) -> Result<Value> {
        let repo_path = params.get("repo_path").and_then(|v| v.as_str());
        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'message' is required for commit".to_string()))?;
        let files = params.get("files").and_then(|v| v.as_array());

        // Stage files
        if let Some(file_list) = files {
            for f in file_list {
                if let Some(path) = f.as_str() {
                    Self::run_git(&["add", path], repo_path).await?;
                }
            }
        } else {
            // Stage all tracked changes
            Self::run_git(&["add", "-u"], repo_path).await?;
        }

        // Scan staged diff for secrets before committing
        let staged_diff = Self::run_git(&["diff", "--cached"], repo_path).await.unwrap_or_default();
        let secrets = scan_for_secrets(&staged_diff);
        if !secrets.is_empty() {
            // Unstage to prevent accidental commit with secrets
            Self::run_git(&["reset", "HEAD"], repo_path).await.ok();
            return Err(LlmError::ToolCall(format!(
                "Commit blocked: potential secrets detected in staged changes. {} suspicious pattern(s) found. \
                 Review and remove secrets before committing. Previews: {}",
                secrets.len(),
                secrets.iter().take(3).cloned().collect::<Vec<_>>().join("; ")
            )));
        }

        // Commit
        let output = Self::run_git(&["commit", "-m", message], repo_path).await?;

        // Get the resulting commit hash
        let hash = Self::run_git(&["rev-parse", "HEAD"], repo_path)
            .await
            .unwrap_or_default()
            .trim()
            .to_string();

        Ok(json!({
            "success": true,
            "hash": hash,
            "message": message,
            "output": output.trim()
        }))
    }

    async fn action_branch(&self, params: &Value) -> Result<Value> {
        let repo_path = params.get("repo_path").and_then(|v| v.as_str());
        let name = params.get("name").and_then(|v| v.as_str());
        let delete = params.get("delete").and_then(|v| v.as_bool()).unwrap_or(false);
        let list = params.get("list").and_then(|v| v.as_bool()).unwrap_or(false);

        if list || name.is_none() {
            let output = Self::run_git(&["branch", "-a", "--format=%(refname:short) %(objectname:short) %(upstream:short)"], repo_path).await?;
            let current_output = Self::run_git(&["branch", "--show-current"], repo_path).await.unwrap_or_default();
            let current = current_output.trim().to_string();

            let branches: Vec<Value> = output
                .lines()
                .filter(|l| !l.is_empty())
                .map(|line| {
                    let parts: Vec<&str> = line.splitn(3, ' ').collect();
                    let name = parts.first().unwrap_or(&"").to_string();
                    let hash = parts.get(1).unwrap_or(&"").to_string();
                    let upstream = parts.get(2).unwrap_or(&"").to_string();
                    json!({
                        "name": name,
                        "hash": hash,
                        "upstream": if upstream.is_empty() { Value::Null } else { Value::String(upstream) },
                        "current": name == current
                    })
                })
                .collect();

            return Ok(json!({ "branches": branches, "current": current }));
        }

        let branch_name = name.unwrap();
        if delete {
            let output = Self::run_git(&["branch", "-d", branch_name], repo_path).await?;
            Ok(json!({ "deleted": branch_name, "output": output.trim() }))
        } else {
            let output = Self::run_git(&["branch", branch_name], repo_path).await?;
            Ok(json!({ "created": branch_name, "output": output.trim() }))
        }
    }

    async fn action_checkout(&self, params: &Value) -> Result<Value> {
        let repo_path = params.get("repo_path").and_then(|v| v.as_str());
        let branch = params
            .get("branch")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'branch' is required for checkout".to_string()))?;
        let create = params.get("create").and_then(|v| v.as_bool()).unwrap_or(false);

        let mut args = vec!["checkout"];
        if create {
            args.push("-b");
        }
        args.push(branch);

        let output = Self::run_git(&args, repo_path).await?;
        Ok(json!({
            "branch": branch,
            "created": create,
            "output": output.trim()
        }))
    }

    async fn action_stash(&self, params: &Value) -> Result<Value> {
        let repo_path = params.get("repo_path").and_then(|v| v.as_str());
        let pop = params.get("pop").and_then(|v| v.as_bool()).unwrap_or(false);
        let list = params.get("list").and_then(|v| v.as_bool()).unwrap_or(false);

        if list {
            let output = Self::run_git(&["stash", "list"], repo_path).await?;
            let stashes: Vec<Value> = output
                .lines()
                .filter(|l| !l.is_empty())
                .enumerate()
                .map(|(i, line)| json!({ "index": i, "description": line.trim() }))
                .collect();
            return Ok(json!({ "stashes": stashes }));
        }

        if pop {
            let output = Self::run_git(&["stash", "pop"], repo_path).await?;
            Ok(json!({ "action": "pop", "output": output.trim() }))
        } else {
            let output = Self::run_git(&["stash", "push"], repo_path).await?;
            Ok(json!({ "action": "push", "output": output.trim() }))
        }
    }

    async fn action_merge(&self, params: &Value) -> Result<Value> {
        let repo_path = params.get("repo_path").and_then(|v| v.as_str());
        let branch = params
            .get("branch")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'branch' is required for merge".to_string()))?;

        match Self::run_git(&["merge", branch], repo_path).await {
            Ok(output) => Ok(json!({
                "success": true,
                "merged": branch,
                "output": output.trim(),
                "conflicts": false
            })),
            Err(e) => {
                // Check if it's a merge conflict
                let status = Self::run_git(&["status", "--porcelain=v1"], repo_path)
                    .await
                    .unwrap_or_default();
                let conflict_files: Vec<String> = status
                    .lines()
                    .filter(|l| l.starts_with("UU") || l.starts_with("AA") || l.starts_with("DD"))
                    .map(|l| l[3..].trim().to_string())
                    .collect();

                if !conflict_files.is_empty() {
                    Ok(json!({
                        "success": false,
                        "merged": branch,
                        "conflicts": true,
                        "conflict_files": conflict_files,
                        "hint": "Use 'show_conflict' to see conflict details, then 'resolve' to fix each file"
                    }))
                } else {
                    Err(e)
                }
            }
        }
    }

    async fn action_show_conflict(&self, params: &Value) -> Result<Value> {
        let repo_path = params.get("repo_path").and_then(|v| v.as_str());
        let file = params
            .get("file")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'file' is required for show_conflict".to_string()))?;

        // Read the file with conflict markers
        let full_path = if let Some(repo) = repo_path {
            std::path::PathBuf::from(repo).join(file)
        } else {
            std::path::PathBuf::from(file)
        };

        let content = tokio::fs::read_to_string(&full_path)
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to read {}: {e}", full_path.display())))?;

        // Parse conflict markers
        let mut conflicts = Vec::new();
        let mut in_conflict = false;
        let mut ours = Vec::new();
        let mut theirs = Vec::new();
        let mut in_theirs = false;
        let mut start_line = 0;

        for (i, line) in content.lines().enumerate() {
            if line.starts_with("<<<<<<< ") {
                in_conflict = true;
                in_theirs = false;
                ours.clear();
                theirs.clear();
                start_line = i + 1;
            } else if line.starts_with("=======") && in_conflict {
                in_theirs = true;
            } else if line.starts_with(">>>>>>> ") && in_conflict {
                conflicts.push(json!({
                    "start_line": start_line,
                    "end_line": i + 1,
                    "ours": ours.join("\n"),
                    "theirs": theirs.join("\n")
                }));
                in_conflict = false;
                in_theirs = false;
                ours.clear();
                theirs.clear();
            } else if in_conflict {
                if in_theirs {
                    theirs.push(line.to_string());
                } else {
                    ours.push(line.to_string());
                }
            }
        }

        Ok(json!({
            "file": file,
            "conflicts": conflicts,
            "conflict_count": conflicts.len()
        }))
    }

    async fn action_resolve(&self, params: &Value) -> Result<Value> {
        let repo_path = params.get("repo_path").and_then(|v| v.as_str());
        let file = params
            .get("file")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'file' is required for resolve".to_string()))?;
        let content = params
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'content' is required for resolve".to_string()))?;

        // Write resolved content
        let full_path = if let Some(repo) = repo_path {
            std::path::PathBuf::from(repo).join(file)
        } else {
            std::path::PathBuf::from(file)
        };

        tokio::fs::write(&full_path, content)
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to write {}: {e}", full_path.display())))?;

        // Stage the resolved file
        Self::run_git(&["add", file], repo_path).await?;

        Ok(json!({
            "resolved": file,
            "success": true
        }))
    }
}

#[async_trait]
impl NativeTool for GitTool {
    fn name(&self) -> &str {
        "git"
    }

    fn description(&self) -> &str {
        "Version control operations returning structured JSON. Actions: status (parsed repo state), \
         diff (show changes), log (commit history), commit (stage and commit with automatic secret \
         scanning), branch (list/create/delete), checkout (switch branches), stash (save/restore work), \
         merge (merge branches), show_conflict (parse conflict markers), resolve (write resolution). \
         Prefer this over exec with raw git commands."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["status", "diff", "log", "commit", "branch", "checkout",
                             "stash", "merge", "show_conflict", "resolve"],
                    "description": "The git action to perform"
                },
                "repo_path": {
                    "type": "string",
                    "description": "Path to the repository (defaults to current directory)"
                },
                "message": {
                    "type": "string",
                    "description": "Commit message (for 'commit' action)"
                },
                "files": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Files to stage (for 'commit' action, omit to stage all tracked changes)"
                },
                "branch": {
                    "type": "string",
                    "description": "Branch name (for 'checkout' and 'merge' actions)"
                },
                "name": {
                    "type": "string",
                    "description": "Branch name (for 'branch' action to create/delete)"
                },
                "create": {
                    "type": "boolean",
                    "description": "Create new branch (for 'checkout' action)"
                },
                "delete": {
                    "type": "boolean",
                    "description": "Delete branch (for 'branch' action)"
                },
                "list": {
                    "type": "boolean",
                    "description": "List items (for 'branch' and 'stash' actions)"
                },
                "staged": {
                    "type": "boolean",
                    "description": "Show staged diff (for 'diff' action)"
                },
                "file": {
                    "type": "string",
                    "description": "File path (for 'diff', 'show_conflict', 'resolve' actions)"
                },
                "content": {
                    "type": "string",
                    "description": "Resolved file content (for 'resolve' action)"
                },
                "count": {
                    "type": "integer",
                    "description": "Number of log entries (for 'log' action, default 10, max 100)"
                },
                "since": {
                    "type": "string",
                    "description": "Date filter for log (e.g. '2024-01-01', '1 week ago')"
                },
                "author": {
                    "type": "string",
                    "description": "Author filter for log (for 'log' action)"
                },
                "pop": {
                    "type": "boolean",
                    "description": "Pop latest stash (for 'stash' action)"
                }
            },
            "required": ["action"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::VersionControl(None)
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let action = params
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'action' is required".to_string()))?;

        match action {
            "status" => self.action_status(&params).await,
            "diff" => self.action_diff(&params).await,
            "log" => self.action_log(&params).await,
            "commit" => self.action_commit(&params).await,
            "branch" => self.action_branch(&params).await,
            "checkout" => self.action_checkout(&params).await,
            "stash" => self.action_stash(&params).await,
            "merge" => self.action_merge(&params).await,
            "show_conflict" => self.action_show_conflict(&params).await,
            "resolve" => self.action_resolve(&params).await,
            _ => Err(LlmError::ToolCall(format!(
                "Unknown git action: '{action}'. Valid actions: status, diff, log, commit, \
                 branch, checkout, stash, merge, show_conflict, resolve"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn init_test_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_str().unwrap();

        // Init repo
        GitTool::run_git(&["init"], Some(path)).await.unwrap();
        GitTool::run_git(&["config", "user.email", "test@test.com"], Some(path))
            .await
            .unwrap();
        GitTool::run_git(&["config", "user.name", "Test User"], Some(path))
            .await
            .unwrap();

        // Create initial commit
        let file_path = dir.path().join("README.md");
        tokio::fs::write(&file_path, "# Test\n").await.unwrap();
        GitTool::run_git(&["add", "."], Some(path)).await.unwrap();
        GitTool::run_git(&["commit", "-m", "Initial commit"], Some(path))
            .await
            .unwrap();

        dir
    }

    #[tokio::test]
    async fn test_status_clean() {
        let dir = init_test_repo().await;
        let tool = GitTool::new();
        let result = tool
            .execute(json!({
                "action": "status",
                "repo_path": dir.path().to_str().unwrap()
            }))
            .await
            .unwrap();

        let branch = result["branch"].as_str().unwrap();
        assert!(
            branch == "main" || branch == "master",
            "Expected main or master, got: {branch}"
        );
        assert_eq!(result["clean"], true);
    }

    #[tokio::test]
    async fn test_status_modified() {
        let dir = init_test_repo().await;
        tokio::fs::write(dir.path().join("README.md"), "# Changed\n")
            .await
            .unwrap();

        let tool = GitTool::new();
        let result = tool
            .execute(json!({
                "action": "status",
                "repo_path": dir.path().to_str().unwrap()
            }))
            .await
            .unwrap();

        assert_eq!(result["clean"], false);
        let modified = result["modified"].as_array().unwrap();
        assert!(modified.iter().any(|v| v.as_str() == Some("README.md")));
    }

    #[tokio::test]
    async fn test_status_untracked() {
        let dir = init_test_repo().await;
        tokio::fs::write(dir.path().join("new_file.txt"), "new content")
            .await
            .unwrap();

        let tool = GitTool::new();
        let result = tool
            .execute(json!({
                "action": "status",
                "repo_path": dir.path().to_str().unwrap()
            }))
            .await
            .unwrap();

        let untracked = result["untracked"].as_array().unwrap();
        assert!(untracked.iter().any(|v| v.as_str() == Some("new_file.txt")));
    }

    #[tokio::test]
    async fn test_diff() {
        let dir = init_test_repo().await;
        tokio::fs::write(dir.path().join("README.md"), "# Changed\n")
            .await
            .unwrap();

        let tool = GitTool::new();
        let result = tool
            .execute(json!({
                "action": "diff",
                "repo_path": dir.path().to_str().unwrap()
            }))
            .await
            .unwrap();

        let diff = result["diff"].as_str().unwrap();
        assert!(diff.contains("Changed"));
        assert_eq!(result["staged"], false);
    }

    #[tokio::test]
    async fn test_log() {
        let dir = init_test_repo().await;
        let tool = GitTool::new();
        let result = tool
            .execute(json!({
                "action": "log",
                "repo_path": dir.path().to_str().unwrap(),
                "count": 5
            }))
            .await
            .unwrap();

        let commits = result["commits"].as_array().unwrap();
        assert!(!commits.is_empty());
        assert_eq!(commits[0]["subject"], "Initial commit");
        assert_eq!(commits[0]["author"], "Test User");
    }

    #[tokio::test]
    async fn test_commit() {
        let dir = init_test_repo().await;
        tokio::fs::write(dir.path().join("new.txt"), "content")
            .await
            .unwrap();

        let tool = GitTool::new();
        let result = tool
            .execute(json!({
                "action": "commit",
                "repo_path": dir.path().to_str().unwrap(),
                "message": "Add new file",
                "files": ["new.txt"]
            }))
            .await
            .unwrap();

        assert_eq!(result["success"], true);
        assert!(!result["hash"].as_str().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_branch_list() {
        let dir = init_test_repo().await;
        let tool = GitTool::new();
        let result = tool
            .execute(json!({
                "action": "branch",
                "repo_path": dir.path().to_str().unwrap(),
                "list": true
            }))
            .await
            .unwrap();

        let branches = result["branches"].as_array().unwrap();
        assert!(!branches.is_empty());
    }

    #[tokio::test]
    async fn test_branch_create_and_checkout() {
        let dir = init_test_repo().await;
        let path = dir.path().to_str().unwrap();
        let tool = GitTool::new();

        // Create branch
        let result = tool
            .execute(json!({
                "action": "branch",
                "repo_path": path,
                "name": "feature-test"
            }))
            .await
            .unwrap();
        assert_eq!(result["created"], "feature-test");

        // Checkout
        let result = tool
            .execute(json!({
                "action": "checkout",
                "repo_path": path,
                "branch": "feature-test"
            }))
            .await
            .unwrap();
        assert_eq!(result["branch"], "feature-test");
    }

    #[tokio::test]
    async fn test_stash() {
        let dir = init_test_repo().await;
        let path = dir.path().to_str().unwrap();
        tokio::fs::write(dir.path().join("README.md"), "# Changed\n")
            .await
            .unwrap();

        let tool = GitTool::new();

        // Stash
        let result = tool
            .execute(json!({ "action": "stash", "repo_path": path }))
            .await
            .unwrap();
        assert_eq!(result["action"], "push");

        // List
        let result = tool
            .execute(json!({ "action": "stash", "repo_path": path, "list": true }))
            .await
            .unwrap();
        let stashes = result["stashes"].as_array().unwrap();
        assert!(!stashes.is_empty());

        // Pop
        let result = tool
            .execute(json!({ "action": "stash", "repo_path": path, "pop": true }))
            .await
            .unwrap();
        assert_eq!(result["action"], "pop");
    }

    #[tokio::test]
    async fn test_show_conflict_parse() {
        let dir = TempDir::new().unwrap();
        let conflict_content = r#"line 1
<<<<<<< HEAD
our version
=======
their version
>>>>>>> feature
line 3
"#;
        let file_path = dir.path().join("conflict.txt");
        tokio::fs::write(&file_path, conflict_content).await.unwrap();

        let tool = GitTool::new();
        let result = tool
            .execute(json!({
                "action": "show_conflict",
                "file": file_path.to_str().unwrap()
            }))
            .await
            .unwrap();

        let conflicts = result["conflicts"].as_array().unwrap();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0]["ours"], "our version");
        assert_eq!(conflicts[0]["theirs"], "their version");
    }

    #[tokio::test]
    async fn test_unknown_action() {
        let tool = GitTool::new();
        let result = tool.execute(json!({ "action": "invalid" })).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown git action"));
    }

    #[tokio::test]
    async fn test_missing_action() {
        let tool = GitTool::new();
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_secret_scan_detects_aws_key() {
        let diff = "+AKIAIOSFODNN7EXAMPLE\n+some normal code\n";
        let findings = scan_for_secrets(diff);
        assert!(!findings.is_empty(), "Should detect AWS key");
    }

    #[test]
    fn test_secret_scan_detects_private_key() {
        let diff = "+-----BEGIN RSA PRIVATE KEY-----\n+MIIEpAIBAAK...\n";
        let findings = scan_for_secrets(diff);
        assert!(!findings.is_empty(), "Should detect private key header");
    }

    #[test]
    fn test_secret_scan_detects_github_token() {
        let diff = "+const token = \"ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijkl\";\n";
        let findings = scan_for_secrets(diff);
        assert!(!findings.is_empty(), "Should detect GitHub token");
    }

    #[test]
    fn test_secret_scan_clean_diff() {
        let diff = "+fn main() {\n+    println!(\"Hello, world!\");\n+}\n";
        let findings = scan_for_secrets(diff);
        assert!(findings.is_empty(), "Normal code should not trigger secrets: {:?}", findings);
    }

    #[test]
    fn test_secret_scan_ignores_removed_lines() {
        let diff = "-AKIAIOSFODNN7EXAMPLE\n some context\n";
        let findings = scan_for_secrets(diff);
        assert!(findings.is_empty(), "Removed lines should not be scanned");
    }
}
