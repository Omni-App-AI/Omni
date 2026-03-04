//! Native test runner tool with framework auto-detection.
//!
//! Detects testing frameworks from project files, runs tests,
//! and parses results into structured JSON for reliable LLM consumption.
//!
//! Gated by `process.spawn` permission.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use omni_permissions::capability::Capability;
use serde_json::{json, Value};
use tokio::process::Command;

use super::NativeTool;
use crate::error::{LlmError, Result};

/// Maximum test output before truncation (100KB).
const MAX_OUTPUT: usize = 100 * 1024;

#[derive(Debug, Clone, PartialEq)]
enum Framework {
    CargoTest,
    Jest,
    Vitest,
    Mocha,
    Pytest,
    GoTest,
    DotnetTest,
}

impl Framework {
    fn name(&self) -> &str {
        match self {
            Self::CargoTest => "cargo-test",
            Self::Jest => "jest",
            Self::Vitest => "vitest",
            Self::Mocha => "mocha",
            Self::Pytest => "pytest",
            Self::GoTest => "go-test",
            Self::DotnetTest => "dotnet-test",
        }
    }
}

pub struct TestRunnerTool;

impl TestRunnerTool {
    pub fn new() -> Self {
        Self
    }

    /// Auto-detect the testing framework from project files.
    fn detect_framework(root: &Path) -> Option<Framework> {
        if root.join("Cargo.toml").exists() {
            return Some(Framework::CargoTest);
        }
        if root.join("go.mod").exists() {
            return Some(Framework::GoTest);
        }

        // Check package.json for JS frameworks
        if root.join("package.json").exists() {
            if let Ok(content) = std::fs::read_to_string(root.join("package.json")) {
                if content.contains("vitest") {
                    return Some(Framework::Vitest);
                }
                if content.contains("jest") {
                    return Some(Framework::Jest);
                }
                if content.contains("mocha") {
                    return Some(Framework::Mocha);
                }
            }
            // Default to jest if package.json exists but no framework detected
            return Some(Framework::Jest);
        }

        if root.join("pyproject.toml").exists()
            || root.join("pytest.ini").exists()
            || root.join("setup.py").exists()
            || root.join("setup.cfg").exists()
        {
            return Some(Framework::Pytest);
        }

        // .NET: check for .csproj or .sln files in the root directory
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "csproj" || ext == "sln" {
                        return Some(Framework::DotnetTest);
                    }
                }
            }
        }

        None
    }

    fn parse_framework(name: &str) -> Option<Framework> {
        match name.to_lowercase().as_str() {
            "cargo" | "cargo-test" | "rust" => Some(Framework::CargoTest),
            "jest" => Some(Framework::Jest),
            "vitest" => Some(Framework::Vitest),
            "mocha" => Some(Framework::Mocha),
            "pytest" | "python" => Some(Framework::Pytest),
            "go" | "go-test" | "gotest" => Some(Framework::GoTest),
            "dotnet" | "dotnet-test" => Some(Framework::DotnetTest),
            _ => None,
        }
    }

    fn build_test_command(
        framework: &Framework,
        file: Option<&str>,
        pattern: Option<&str>,
        coverage: bool,
    ) -> (String, Vec<String>) {
        match framework {
            Framework::CargoTest => {
                let cmd = "cargo".to_string();
                let mut args = vec!["test".to_string()];
                if let Some(p) = pattern {
                    args.push(p.to_string());
                }
                if let Some(f) = file {
                    args.push("--test".to_string());
                    args.push(f.to_string());
                }
                args.push("--".to_string());
                args.push("--format=terse".to_string());
                if coverage {
                    // For coverage, use cargo-llvm-cov if available
                    return ("cargo".to_string(), vec![
                        "llvm-cov".to_string(),
                        "test".to_string(),
                        "--lcov".to_string(),
                        "--output-path=lcov.info".to_string(),
                    ]);
                }
                (cmd, args)
            }
            Framework::Jest => {
                let cmd = if cfg!(windows) { "npx.cmd" } else { "npx" }.to_string();
                let mut args = vec!["jest".to_string(), "--no-color".to_string()];
                if let Some(p) = pattern {
                    args.push("-t".to_string());
                    args.push(p.to_string());
                }
                if let Some(f) = file {
                    args.push(f.to_string());
                }
                if coverage {
                    args.push("--coverage".to_string());
                }
                (cmd, args)
            }
            Framework::Vitest => {
                let cmd = if cfg!(windows) { "npx.cmd" } else { "npx" }.to_string();
                let mut args = vec!["vitest".to_string(), "run".to_string()];
                if let Some(p) = pattern {
                    args.push("-t".to_string());
                    args.push(p.to_string());
                }
                if let Some(f) = file {
                    args.push(f.to_string());
                }
                if coverage {
                    args.push("--coverage".to_string());
                }
                (cmd, args)
            }
            Framework::Mocha => {
                let cmd = if cfg!(windows) { "npx.cmd" } else { "npx" }.to_string();
                let mut args = vec!["mocha".to_string()];
                if let Some(p) = pattern {
                    args.push("--grep".to_string());
                    args.push(p.to_string());
                }
                if let Some(f) = file {
                    args.push(f.to_string());
                }
                (cmd, args)
            }
            Framework::Pytest => {
                let cmd = "python".to_string();
                let mut args = vec!["-m".to_string(), "pytest".to_string(), "-v".to_string()];
                if let Some(p) = pattern {
                    args.push("-k".to_string());
                    args.push(p.to_string());
                }
                if let Some(f) = file {
                    args.push(f.to_string());
                }
                if coverage {
                    args.push("--cov".to_string());
                    args.push("--cov-report=term-missing".to_string());
                }
                (cmd, args)
            }
            Framework::GoTest => {
                let cmd = "go".to_string();
                let mut args = vec!["test".to_string(), "-v".to_string()];
                if let Some(p) = pattern {
                    args.push(format!("-run={}", p));
                }
                if let Some(f) = file {
                    args.push(f.to_string());
                } else {
                    args.push("./...".to_string());
                }
                if coverage {
                    args.push("-coverprofile=coverage.out".to_string());
                }
                (cmd, args)
            }
            Framework::DotnetTest => {
                let cmd = "dotnet".to_string();
                let mut args = vec!["test".to_string(), "--verbosity".to_string(), "normal".to_string()];
                if let Some(p) = pattern {
                    args.push("--filter".to_string());
                    args.push(p.to_string());
                }
                if let Some(f) = file {
                    args.push(f.to_string());
                }
                if coverage {
                    args.push("--collect:\"XPlat Code Coverage\"".to_string());
                }
                (cmd, args)
            }
        }
    }

    /// Extract the first number found in a text fragment.
    fn extract_number(text: &str) -> Option<u32> {
        for word in text.split_whitespace() {
            if let Ok(n) = word.trim_matches(|c: char| !c.is_ascii_digit()).parse::<u32>() {
                return Some(n);
            }
        }
        None
    }

    fn parse_test_results(output: &str, stderr: &str, framework: &Framework) -> Value {
        let mut passed = 0u32;
        let mut failed = 0u32;
        let mut skipped = 0u32;
        let mut failures: Vec<Value> = Vec::new();

        let combined = format!("{}\n{}", output, stderr);

        match framework {
            Framework::CargoTest => {
                for line in combined.lines() {
                    if line.contains("test result:") {
                        // e.g., "test result: ok. 42 passed; 2 failed; 1 ignored; 0 measured"
                        // Split by ';' and find the number before each keyword
                        let after_result = line.split("test result:").nth(1).unwrap_or(line);
                        for part in after_result.split(';') {
                            if let Some(n) = Self::extract_number(part) {
                                if part.contains("passed") {
                                    passed += n;
                                } else if part.contains("failed") {
                                    failed += n;
                                } else if part.contains("ignored") {
                                    skipped += n;
                                }
                            }
                        }
                    }
                    if line.ends_with("FAILED") && line.starts_with("test ") {
                        let test_name = line
                            .strip_prefix("test ")
                            .and_then(|s| s.strip_suffix(" ... FAILED"))
                            .unwrap_or(line)
                            .to_string();
                        failures.push(json!({
                            "test": test_name,
                            "error": "FAILED (see output for details)"
                        }));
                    }
                }
            }
            Framework::Jest | Framework::Vitest => {
                // Jest/Vitest summary: "Tests: 2 failed, 42 passed, 44 total"
                for line in combined.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("Tests:") || trimmed.starts_with("Test Suites:") {
                        for part in trimmed.split(',') {
                            if let Some(n) = Self::extract_number(part) {
                                if part.contains("passed") {
                                    passed += n;
                                } else if part.contains("failed") {
                                    failed += n;
                                } else if part.contains("skipped") || part.contains("pending") {
                                    skipped += n;
                                }
                            }
                        }
                    }
                    // Failure indicators
                    if trimmed.starts_with("FAIL ") || trimmed.starts_with("✕") || trimmed.starts_with("×") {
                        failures.push(json!({
                            "test": trimmed,
                            "error": "FAILED"
                        }));
                    }
                }
            }
            Framework::Pytest => {
                // Pytest summary: "====== 5 passed, 2 failed, 1 skipped in 3.45s ======"
                for line in combined.lines() {
                    let trimmed = line.trim();
                    if trimmed.contains("passed") || trimmed.contains("failed") || trimmed.contains("error") {
                        for part in trimmed.split(',') {
                            if let Some(n) = Self::extract_number(part) {
                                if part.contains("passed") {
                                    passed += n;
                                } else if part.contains("failed") || part.contains("error") {
                                    failed += n;
                                } else if part.contains("skipped") || part.contains("deselected") {
                                    skipped += n;
                                }
                            }
                        }
                    }
                    if trimmed.starts_with("FAILED ") {
                        failures.push(json!({
                            "test": trimmed.strip_prefix("FAILED ").unwrap_or(trimmed),
                            "error": "FAILED"
                        }));
                    }
                }
            }
            Framework::GoTest => {
                for line in combined.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("--- PASS:") {
                        passed += 1;
                    } else if trimmed.starts_with("--- FAIL:") {
                        failed += 1;
                        let test_name = trimmed
                            .strip_prefix("--- FAIL: ")
                            .and_then(|s| s.split_whitespace().next())
                            .unwrap_or(trimmed)
                            .to_string();
                        failures.push(json!({
                            "test": test_name,
                            "error": "FAILED"
                        }));
                    } else if trimmed.starts_with("--- SKIP:") {
                        skipped += 1;
                    }
                }
            }
            _ => {
                // Generic parsing: count pass/fail keywords
                for line in combined.lines() {
                    let lower = line.to_lowercase();
                    if lower.contains("pass") {
                        passed += 1;
                    }
                    if lower.contains("fail") {
                        failed += 1;
                    }
                }
            }
        }

        json!({
            "passed": passed,
            "failed": failed,
            "skipped": skipped,
            "total": passed + failed + skipped,
            "success": failed == 0,
            "failures": failures,
        })
    }

    async fn action_run(&self, params: &Value) -> Result<Value> {
        let framework_name = params.get("framework").and_then(|v| v.as_str());
        let file = params.get("file").and_then(|v| v.as_str());
        let pattern = params.get("pattern").and_then(|v| v.as_str());
        let coverage = params.get("coverage").and_then(|v| v.as_bool()).unwrap_or(false);
        let working_dir = params.get("working_dir").and_then(|v| v.as_str());

        let root = PathBuf::from(working_dir.unwrap_or("."));

        let framework = if let Some(name) = framework_name {
            Self::parse_framework(name)
                .ok_or_else(|| LlmError::ToolCall(format!("Unknown framework: '{name}'")))?
        } else {
            Self::detect_framework(&root)
                .ok_or_else(|| LlmError::ToolCall(
                    "Could not auto-detect test framework. Set 'framework' explicitly.".to_string()
                ))?
        };

        let (cmd, args) = Self::build_test_command(&framework, file, pattern, coverage);

        let output = Command::new(&cmd)
            .args(&args)
            .current_dir(&root)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to run {cmd}: {e}")))?;

        let mut stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let mut stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if stdout.len() > MAX_OUTPUT {
            stdout.truncate(MAX_OUTPUT);
            stdout.push_str("\n... (output truncated)");
        }
        if stderr.len() > MAX_OUTPUT / 2 {
            stderr.truncate(MAX_OUTPUT / 2);
            stderr.push_str("\n... (stderr truncated)");
        }

        let mut results = Self::parse_test_results(&stdout, &stderr, &framework);
        results["framework"] = json!(framework.name());
        results["command"] = json!(format!("{} {}", cmd, args.join(" ")));
        results["exit_code"] = json!(output.status.code());
        results["stdout"] = json!(stdout);
        results["stderr"] = json!(stderr);

        Ok(results)
    }

    async fn action_list(&self, params: &Value) -> Result<Value> {
        let framework_name = params.get("framework").and_then(|v| v.as_str());
        let file = params.get("file").and_then(|v| v.as_str());
        let working_dir = params.get("working_dir").and_then(|v| v.as_str());

        let root = PathBuf::from(working_dir.unwrap_or("."));

        let framework = if let Some(name) = framework_name {
            Self::parse_framework(name)
                .ok_or_else(|| LlmError::ToolCall(format!("Unknown framework: '{name}'")))?
        } else {
            Self::detect_framework(&root)
                .ok_or_else(|| LlmError::ToolCall(
                    "Could not auto-detect test framework. Set 'framework' explicitly.".to_string()
                ))?
        };

        // Build list command based on framework
        let (cmd, args) = match &framework {
            Framework::CargoTest => {
                let mut a = vec!["test".to_string()];
                if let Some(f) = file {
                    a.push("--test".to_string());
                    a.push(f.to_string());
                }
                a.push("--".to_string());
                a.push("--list".to_string());
                ("cargo".to_string(), a)
            }
            Framework::Jest => {
                let cmd = if cfg!(windows) { "npx.cmd" } else { "npx" }.to_string();
                let mut a = vec!["jest".to_string(), "--listTests".to_string()];
                if let Some(f) = file {
                    a.push(f.to_string());
                }
                (cmd, a)
            }
            Framework::Vitest => {
                let cmd = if cfg!(windows) { "npx.cmd" } else { "npx" }.to_string();
                (cmd, vec!["vitest".to_string(), "list".to_string()])
            }
            Framework::Pytest => {
                let mut a = vec!["-m".to_string(), "pytest".to_string(), "--collect-only".to_string(), "-q".to_string()];
                if let Some(f) = file {
                    a.push(f.to_string());
                }
                ("python".to_string(), a)
            }
            Framework::GoTest => {
                let mut a = vec!["test".to_string(), "-list".to_string(), ".*".to_string()];
                if let Some(f) = file {
                    a.push(f.to_string());
                } else {
                    a.push("./...".to_string());
                }
                ("go".to_string(), a)
            }
            _ => {
                return Err(LlmError::ToolCall(format!(
                    "Test listing not supported for framework: {}",
                    framework.name()
                )));
            }
        };

        let output = Command::new(&cmd)
            .args(&args)
            .current_dir(&root)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to run {cmd}: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let tests: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();

        Ok(json!({
            "framework": framework.name(),
            "tests": tests,
            "count": tests.len()
        }))
    }
}

#[async_trait]
impl NativeTool for TestRunnerTool {
    fn name(&self) -> &str {
        "test_runner"
    }

    fn description(&self) -> &str {
        "Run tests with automatic framework detection and structured output. Actions: 'run' (execute tests \
         and parse results), 'list' (list available tests without running). Supports: cargo test (Rust), \
         jest/vitest/mocha (JS/TS), pytest (Python), go test (Go), dotnet test (.NET). Returns structured \
         results with pass/fail counts and failure details. Prefer this over exec with raw test commands."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["run", "list", "coverage"],
                    "description": "Action to perform: 'run' tests, 'list' available tests, or 'coverage' (run with coverage report)"
                },
                "framework": {
                    "type": "string",
                    "enum": ["cargo", "jest", "vitest", "mocha", "pytest", "go", "dotnet"],
                    "description": "Test framework (auto-detected if omitted)"
                },
                "file": {
                    "type": "string",
                    "description": "Specific test file or module to run"
                },
                "pattern": {
                    "type": "string",
                    "description": "Test name pattern to filter (for 'run' action)"
                },
                "coverage": {
                    "type": "boolean",
                    "description": "Generate code coverage report (for 'run' action)"
                },
                "working_dir": {
                    "type": "string",
                    "description": "Working directory (defaults to current directory)"
                }
            },
            "required": ["action"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::ProcessSpawn(None)
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let action = params
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'action' is required".to_string()))?;

        match action {
            "run" => self.action_run(&params).await,
            "list" => self.action_list(&params).await,
            "coverage" => {
                // Coverage is just run with coverage=true forced on
                let mut cov_params = params.clone();
                if let Some(obj) = cov_params.as_object_mut() {
                    obj.insert("coverage".to_string(), Value::Bool(true));
                    // Remove action so action_run doesn't choke
                }
                self.action_run(&cov_params).await
            }
            _ => Err(LlmError::ToolCall(format!(
                "Unknown test_runner action: '{action}'. Valid actions: run, list, coverage"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_framework_cargo() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        assert_eq!(
            TestRunnerTool::detect_framework(dir.path()),
            Some(Framework::CargoTest)
        );
    }

    #[test]
    fn test_detect_framework_jest() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"devDependencies":{"jest":"^29.0.0"}}"#,
        )
        .unwrap();
        assert_eq!(
            TestRunnerTool::detect_framework(dir.path()),
            Some(Framework::Jest)
        );
    }

    #[test]
    fn test_detect_framework_vitest() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"devDependencies":{"vitest":"^1.0.0"}}"#,
        )
        .unwrap();
        assert_eq!(
            TestRunnerTool::detect_framework(dir.path()),
            Some(Framework::Vitest)
        );
    }

    #[test]
    fn test_detect_framework_pytest() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("pyproject.toml"), "[tool.pytest]").unwrap();
        assert_eq!(
            TestRunnerTool::detect_framework(dir.path()),
            Some(Framework::Pytest)
        );
    }

    #[test]
    fn test_detect_framework_go() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.mod"), "module test").unwrap();
        assert_eq!(
            TestRunnerTool::detect_framework(dir.path()),
            Some(Framework::GoTest)
        );
    }

    #[test]
    fn test_detect_framework_dotnet() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("MyProject.csproj"), "<Project></Project>").unwrap();
        assert_eq!(
            TestRunnerTool::detect_framework(dir.path()),
            Some(Framework::DotnetTest)
        );
    }

    #[test]
    fn test_detect_framework_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(TestRunnerTool::detect_framework(dir.path()), None);
    }

    #[test]
    fn test_parse_framework_names() {
        assert_eq!(TestRunnerTool::parse_framework("cargo"), Some(Framework::CargoTest));
        assert_eq!(TestRunnerTool::parse_framework("rust"), Some(Framework::CargoTest));
        assert_eq!(TestRunnerTool::parse_framework("jest"), Some(Framework::Jest));
        assert_eq!(TestRunnerTool::parse_framework("pytest"), Some(Framework::Pytest));
        assert_eq!(TestRunnerTool::parse_framework("go"), Some(Framework::GoTest));
        assert_eq!(TestRunnerTool::parse_framework("unknown"), None);
    }

    #[test]
    fn test_parse_cargo_results() {
        let output = r#"
running 5 tests
test tests::test_one ... ok
test tests::test_two ... ok
test tests::test_three ... FAILED
test tests::test_four ... ok
test tests::test_five ... ok

test result: ok. 4 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
"#;
        let result = TestRunnerTool::parse_test_results(output, "", &Framework::CargoTest);
        assert_eq!(result["passed"], 4);
        assert_eq!(result["failed"], 1);
        assert_eq!(result["skipped"], 0);
        assert_eq!(result["success"], false);
    }

    #[test]
    fn test_parse_pytest_results() {
        let output = "====== 10 passed, 2 failed, 1 skipped in 3.45s ======";
        let result = TestRunnerTool::parse_test_results(output, "", &Framework::Pytest);
        assert_eq!(result["passed"], 10);
        assert_eq!(result["failed"], 2);
        assert_eq!(result["skipped"], 1);
    }

    #[test]
    fn test_parse_go_results() {
        let output = r#"
--- PASS: TestAdd (0.00s)
--- PASS: TestSub (0.00s)
--- FAIL: TestDiv (0.01s)
--- SKIP: TestMul (0.00s)
FAIL
"#;
        let result = TestRunnerTool::parse_test_results(output, "", &Framework::GoTest);
        assert_eq!(result["passed"], 2);
        assert_eq!(result["failed"], 1);
        assert_eq!(result["skipped"], 1);
    }

    #[test]
    fn test_build_command_cargo() {
        let (cmd, args) = TestRunnerTool::build_test_command(
            &Framework::CargoTest,
            None,
            Some("test_auth"),
            false,
        );
        assert_eq!(cmd, "cargo");
        assert!(args.contains(&"test".to_string()));
        assert!(args.contains(&"test_auth".to_string()));
    }

    #[test]
    fn test_build_command_pytest_coverage() {
        let (cmd, args) = TestRunnerTool::build_test_command(
            &Framework::Pytest,
            Some("tests/test_auth.py"),
            None,
            true,
        );
        assert_eq!(cmd, "python");
        assert!(args.contains(&"--cov".to_string()));
        assert!(args.contains(&"tests/test_auth.py".to_string()));
    }

    #[tokio::test]
    async fn test_unknown_action() {
        let tool = TestRunnerTool::new();
        let result = tool.execute(json!({ "action": "invalid" })).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_missing_action() {
        let tool = TestRunnerTool::new();
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
    }
}
