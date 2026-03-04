use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::manifest::ToolDefinition;

use super::engine::{FlowchartEngine, TestResult};
use super::error::{FlowchartError, Result};
use super::types::FlowchartDefinition;

/// Summary info for listing flowcharts.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FlowchartSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub enabled: bool,
    pub tool_count: usize,
    pub permission_count: usize,
}

/// Validation warning or error.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationIssue {
    pub level: ValidationLevel,
    pub message: String,
    pub node_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ValidationLevel {
    Error,
    Warning,
}

/// Manages flowchart definitions and exposes their tools.
pub struct FlowchartRegistry {
    flowcharts: RwLock<HashMap<String, FlowchartDefinition>>,
    flowcharts_dir: PathBuf,
    engine: Arc<FlowchartEngine>,
}

impl FlowchartRegistry {
    pub fn new(flowcharts_dir: PathBuf, engine: Arc<FlowchartEngine>) -> Self {
        Self {
            flowcharts: RwLock::new(HashMap::new()),
            flowcharts_dir,
            engine,
        }
    }

    /// Discover and load all flowchart JSON files from the flowcharts directory.
    pub async fn discover(&self) -> Result<Vec<String>> {
        let dir = self.flowcharts_dir.clone();
        let loaded = tokio::task::spawn_blocking(move || -> Result<Vec<(String, FlowchartDefinition)>> {
            if !dir.exists() {
                std::fs::create_dir_all(&dir)?;
                return Ok(Vec::new());
            }
            let mut results = Vec::new();
            for entry in std::fs::read_dir(&dir)?.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    match std::fs::read_to_string(&path)
                        .map_err(FlowchartError::from)
                        .and_then(|c| serde_json::from_str::<FlowchartDefinition>(&c).map_err(FlowchartError::from))
                    {
                        Ok(def) => results.push((def.id.clone(), def)),
                        Err(e) => tracing::warn!("Failed to load flowchart from {}: {}", path.display(), e),
                    }
                }
            }
            Ok(results)
        })
        .await
        .map_err(|e| FlowchartError::Other(format!("spawn_blocking failed: {e}")))??;

        let mut discovered = Vec::new();
        let mut flowcharts = self.flowcharts.write().await;
        for (id, def) in loaded {
            discovered.push(id.clone());
            flowcharts.insert(id, def);
        }
        Ok(discovered)
    }

    /// Save a flowchart definition (create or update).
    pub async fn save(&self, mut definition: FlowchartDefinition) -> Result<()> {
        // Validate the definition
        let issues = self.validate_definition(&definition);
        let has_errors = issues.iter().any(|i| i.level == ValidationLevel::Error);
        if has_errors {
            let errors: Vec<String> = issues
                .iter()
                .filter(|i| i.level == ValidationLevel::Error)
                .map(|i| i.message.clone())
                .collect();
            return Err(FlowchartError::Validation(errors.join("; ")));
        }

        // Update timestamp
        definition.updated_at = chrono::Utc::now().to_rfc3339();
        if definition.created_at.is_empty() {
            definition.created_at = definition.updated_at.clone();
        }

        // Save to disk (non-blocking)
        let filename = format!("{}.json", definition.id);
        let path = self.flowcharts_dir.join(&filename);
        let json = serde_json::to_string_pretty(&definition)?;
        tokio::task::spawn_blocking(move || std::fs::write(&path, json))
            .await
            .map_err(|e| FlowchartError::Other(format!("spawn_blocking failed: {e}")))?
            ?;

        // Update in-memory registry
        let id = definition.id.clone();
        let mut flowcharts = self.flowcharts.write().await;
        flowcharts.insert(id, definition);

        Ok(())
    }

    /// Get a specific flowchart by ID.
    pub async fn get(&self, id: &str) -> Option<FlowchartDefinition> {
        let flowcharts = self.flowcharts.read().await;
        flowcharts.get(id).cloned()
    }

    /// Delete a flowchart by ID.
    pub async fn delete(&self, id: &str) -> Result<()> {
        // Remove from memory
        let mut flowcharts = self.flowcharts.write().await;
        flowcharts.remove(id);

        // Remove from disk (non-blocking)
        let filename = format!("{id}.json");
        let path = self.flowcharts_dir.join(&filename);
        tokio::task::spawn_blocking(move || {
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
            Ok::<(), std::io::Error>(())
        })
        .await
        .map_err(|e| FlowchartError::Other(format!("spawn_blocking failed: {e}")))?
        ?;

        Ok(())
    }

    /// List all loaded flowcharts (summary only).
    pub async fn list(&self) -> Vec<FlowchartSummary> {
        let flowcharts = self.flowcharts.read().await;
        flowcharts
            .values()
            .map(|fc| FlowchartSummary {
                id: fc.id.clone(),
                name: fc.name.clone(),
                version: fc.version.clone(),
                author: fc.author.clone(),
                description: fc.description.clone(),
                enabled: fc.enabled,
                tool_count: fc.tools.len(),
                permission_count: fc.permissions.len(),
            })
            .collect()
    }

    /// Toggle a flowchart's enabled state.
    pub async fn set_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        let mut flowcharts = self.flowcharts.write().await;
        let fc = flowcharts
            .get_mut(id)
            .ok_or_else(|| FlowchartError::NotFound(id.to_string()))?;
        fc.enabled = enabled;
        fc.updated_at = chrono::Utc::now().to_rfc3339();

        // Persist to disk (non-blocking)
        let filename = format!("{id}.json");
        let path = self.flowcharts_dir.join(&filename);
        let json = serde_json::to_string_pretty(fc)?;
        drop(flowcharts); // Release write lock before blocking I/O
        tokio::task::spawn_blocking(move || std::fs::write(&path, json))
            .await
            .map_err(|e| FlowchartError::Other(format!("spawn_blocking failed: {e}")))?
            ?;

        Ok(())
    }

    /// Get tool definitions from all enabled flowcharts.
    /// Returns `(flowchart_id, ToolDefinition)` pairs.
    pub async fn get_all_tools(&self) -> Vec<(String, ToolDefinition)> {
        let flowcharts = self.flowcharts.read().await;
        let mut tools = Vec::new();

        for fc in flowcharts.values() {
            if !fc.enabled {
                continue;
            }
            for tool in &fc.tools {
                tools.push((
                    fc.id.clone(),
                    ToolDefinition {
                        name: tool.name.clone(),
                        description: tool.description.clone(),
                        parameters: tool.parameters.clone(),
                    },
                ));
            }
        }

        tools
    }

    /// Execute a flowchart tool.
    pub async fn invoke_tool(
        &self,
        flowchart_id: &str,
        tool_name: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.invoke_tool_with_depth(flowchart_id, tool_name, params, 0).await
    }

    /// Execute a flowchart tool with an explicit sub-flow depth.
    /// Called by FlowchartCallback to propagate recursion depth from parent flows.
    pub async fn invoke_tool_with_depth(
        &self,
        flowchart_id: &str,
        tool_name: &str,
        params: &serde_json::Value,
        depth: u32,
    ) -> Result<serde_json::Value> {
        let flowcharts = self.flowcharts.read().await;
        let fc = flowcharts
            .get(flowchart_id)
            .ok_or_else(|| FlowchartError::NotFound(flowchart_id.to_string()))?;

        if !fc.enabled {
            return Err(FlowchartError::Other(format!(
                "Flowchart '{flowchart_id}' is disabled"
            )));
        }

        let tool = fc
            .tools
            .iter()
            .find(|t| t.name == tool_name)
            .ok_or_else(|| {
                FlowchartError::NotFound(format!(
                    "Tool '{tool_name}' not found in flowchart '{flowchart_id}'"
                ))
            })?;

        // Validate params against tool's JSON Schema
        if tool.parameters.is_object() {
            if let Ok(validator) = jsonschema::validator_for(&tool.parameters) {
                let errors: Vec<String> = validator
                    .iter_errors(params)
                    .map(|e| e.to_string())
                    .collect();
                if !errors.is_empty() {
                    return Err(FlowchartError::Validation(format!(
                        "Parameter validation failed: {}",
                        errors.join("; ")
                    )));
                }
            }
        }

        let trigger_node_id = tool.trigger_node_id.clone();
        let fc_clone = fc.clone();
        drop(flowcharts); // Release the read lock before executing

        // Warm permission cache (async check populates cache for sync check_sync during execution)
        self.engine.warm_permissions(&fc_clone).await;

        self.engine
            .execute_with_depth(&fc_clone, &trigger_node_id, params.clone(), depth)
            .await
    }

    /// Run a test execution.
    pub async fn test_execute(
        &self,
        flowchart_id: &str,
        tool_name: &str,
        test_params: serde_json::Value,
    ) -> Result<TestResult> {
        let flowcharts = self.flowcharts.read().await;
        let fc = flowcharts
            .get(flowchart_id)
            .ok_or_else(|| FlowchartError::NotFound(flowchart_id.to_string()))?;

        let tool = fc
            .tools
            .iter()
            .find(|t| t.name == tool_name)
            .ok_or_else(|| {
                FlowchartError::NotFound(format!(
                    "Tool '{tool_name}' not found in flowchart '{flowchart_id}'"
                ))
            })?;

        let trigger_node_id = tool.trigger_node_id.clone();
        let fc_clone = fc.clone();
        drop(flowcharts);

        // Warm permission cache for test execution too
        self.engine.warm_permissions(&fc_clone).await;

        Ok(self
            .engine
            .test_execute(&fc_clone, &trigger_node_id, test_params)
            .await)
    }

    /// Validate a flowchart definition.
    pub fn validate(&self, definition: &FlowchartDefinition) -> Vec<ValidationIssue> {
        self.validate_definition(definition)
    }

    // ── Internal ────────────────────────────────────────────────────────

    fn validate_definition(&self, def: &FlowchartDefinition) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // ID format: must start with "flow.", contain a dot, be >= 5 chars
        // The "flow." prefix prevents ID collisions with WASM extensions
        if !def.id.starts_with("flow.") || !def.id.contains('.') || def.id.len() < 6 {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                message: format!(
                    "ID '{}' must start with 'flow.' prefix (e.g., 'flow.my.tool')",
                    def.id
                ),
                node_id: None,
            });
        }

        // Block path traversal characters in ID
        if def.id.contains('/')
            || def.id.contains('\\')
            || def.id.contains("..")
            || def.id.starts_with('.')
            || def.id.ends_with('.')
        {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                message: format!(
                    "ID '{}' contains invalid characters (path separators, leading/trailing dots, or '..')",
                    def.id
                ),
                node_id: None,
            });
        }

        // Check for duplicate node IDs
        {
            let mut seen_node_ids = std::collections::HashSet::new();
            for node in &def.nodes {
                if !seen_node_ids.insert(node.id.as_str()) {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Error,
                        message: format!("Duplicate node ID: '{}'", node.id),
                        node_id: Some(node.id.clone()),
                    });
                }
            }
        }

        // Check for duplicate and invalid tool names
        {
            let mut seen_tool_names = std::collections::HashSet::new();
            for tool in &def.tools {
                if !seen_tool_names.insert(tool.name.as_str()) {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Error,
                        message: format!("Duplicate tool name: '{}'", tool.name),
                        node_id: None,
                    });
                }
                // Tool names must not contain dots -- the agent's parse_tool_name()
                // uses rfind('.') to split "{fc_id}.{tool_name}", so a dot in the
                // tool name would cause an incorrect split and potential collision.
                if tool.name.contains('.') {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Error,
                        message: format!(
                            "Tool name '{}' must not contain dots (use underscores or hyphens instead)",
                            tool.name
                        ),
                        node_id: None,
                    });
                }
            }
        }

        // Must have at least one tool
        if def.tools.is_empty() {
            issues.push(ValidationIssue {
                level: ValidationLevel::Warning,
                message: "No tools defined -- this flowchart won't expose any tools".to_string(),
                node_id: None,
            });
        }

        // Build node ID set
        let node_ids: std::collections::HashSet<&str> =
            def.nodes.iter().map(|n| n.id.as_str()).collect();

        // Each tool's trigger_node_id must reference an existing Trigger node
        for tool in &def.tools {
            if !node_ids.contains(tool.trigger_node_id.as_str()) {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Error,
                    message: format!(
                        "Tool '{}' references trigger node '{}' which doesn't exist",
                        tool.name, tool.trigger_node_id
                    ),
                    node_id: None,
                });
            } else if let Some(node) = def.nodes.iter().find(|n| n.id == tool.trigger_node_id) {
                if node.node_type != super::types::FlowNodeType::Trigger {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Error,
                        message: format!(
                            "Tool '{}' references node '{}' which is not a Trigger node",
                            tool.name, tool.trigger_node_id
                        ),
                        node_id: Some(tool.trigger_node_id.clone()),
                    });
                }
            }
        }

        // Validate edges reference existing nodes
        for edge in &def.edges {
            if !node_ids.contains(edge.source.as_str()) {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Error,
                    message: format!(
                        "Edge '{}' source '{}' references non-existent node",
                        edge.id, edge.source
                    ),
                    node_id: None,
                });
            }
            if !node_ids.contains(edge.target.as_str()) {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Error,
                    message: format!(
                        "Edge '{}' target '{}' references non-existent node",
                        edge.id, edge.target
                    ),
                    node_id: None,
                });
            }
        }

        // Check for nodes with no incoming edges (except Trigger nodes)
        let targets: std::collections::HashSet<&str> =
            def.edges.iter().map(|e| e.target.as_str()).collect();
        for node in &def.nodes {
            if node.node_type != super::types::FlowNodeType::Trigger
                && !targets.contains(node.id.as_str())
            {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Warning,
                    message: format!(
                        "Node '{}' ({}) has no incoming edges -- it will never execute",
                        node.id, node.label
                    ),
                    node_id: Some(node.id.clone()),
                });
            }
        }

        // Check that Output nodes exist (at least one)
        let has_output = def
            .nodes
            .iter()
            .any(|n| n.node_type == super::types::FlowNodeType::Output);
        if !has_output && !def.nodes.is_empty() {
            issues.push(ValidationIssue {
                level: ValidationLevel::Warning,
                message: "No Output node found -- flowchart will return null".to_string(),
                node_id: None,
            });
        }

        // Detect cycles using DFS
        {
            let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
            for edge in &def.edges {
                adjacency
                    .entry(edge.source.as_str())
                    .or_default()
                    .push(edge.target.as_str());
            }

            let mut visited = std::collections::HashSet::new();
            let mut in_stack = std::collections::HashSet::new();

            fn dfs<'a>(
                node: &'a str,
                adjacency: &HashMap<&'a str, Vec<&'a str>>,
                visited: &mut std::collections::HashSet<&'a str>,
                in_stack: &mut std::collections::HashSet<&'a str>,
            ) -> bool {
                visited.insert(node);
                in_stack.insert(node);

                if let Some(neighbors) = adjacency.get(node) {
                    for &next in neighbors {
                        if !visited.contains(next) {
                            if dfs(next, adjacency, visited, in_stack) {
                                return true;
                            }
                        } else if in_stack.contains(next) {
                            return true; // back edge = cycle
                        }
                    }
                }

                in_stack.remove(node);
                false
            }

            for node in &def.nodes {
                if !visited.contains(node.id.as_str())
                    && dfs(node.id.as_str(), &adjacency, &mut visited, &mut in_stack)
                {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Error,
                        message: "Cycle detected in flowchart -- execution may loop indefinitely"
                            .to_string(),
                        node_id: None,
                    });
                    break;
                }
            }
        }

        issues
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flowchart::types::*;
    use omni_permissions::policy::DefaultPolicy;
    use omni_permissions::policy::PolicyEngine;
    use std::sync::Mutex;

    fn make_registry(dir: &std::path::Path) -> (FlowchartRegistry, tempfile::TempDir) {
        let tmp_db = tempfile::tempdir().unwrap();
        let db_path = tmp_db.path().join("test.db");
        let db = Arc::new(Mutex::new(
            omni_core::database::Database::open(&db_path, "test-key").unwrap(),
        ));
        let policy = Arc::new(PolicyEngine::new(db.clone(), DefaultPolicy::Deny));
        let engine = Arc::new(FlowchartEngine::new(policy, db));
        (FlowchartRegistry::new(dir.to_path_buf(), engine), tmp_db)
    }

    fn sample_flowchart() -> FlowchartDefinition {
        FlowchartDefinition {
            id: "flow.test.sample".to_string(),
            name: "Sample".to_string(),
            version: "1.0.0".to_string(),
            author: "Test".to_string(),
            description: "Sample flowchart".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            enabled: true,
            tools: vec![FlowchartToolDef {
                name: "greet".to_string(),
                description: "Greet someone".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {"name": {"type": "string"}},
                    "required": ["name"]
                }),
                trigger_node_id: "t1".to_string(),
            }],
            permissions: vec![],
            config: HashMap::new(),
            auto_triggers: vec![],
            nodes: vec![
                FlowNode {
                    id: "t1".to_string(),
                    node_type: FlowNodeType::Trigger,
                    label: "Start".to_string(),
                    position: NodePosition { x: 0.0, y: 0.0 },
                    config: serde_json::json!({}),
                },
                FlowNode {
                    id: "o1".to_string(),
                    node_type: FlowNodeType::Output,
                    label: "Return".to_string(),
                    position: NodePosition { x: 0.0, y: 200.0 },
                    config: serde_json::json!({
                        "result_template": "Hello, {{$.params.name}}!"
                    }),
                },
            ],
            edges: vec![FlowEdge {
                id: "e1".to_string(),
                source: "t1".to_string(),
                target: "o1".to_string(),
                source_handle: None,
                target_handle: None,
                label: None,
            }],
            viewport: None,
        }
    }

    #[tokio::test]
    async fn test_save_and_get() {
        let dir = std::env::temp_dir().join(format!("omni_fc_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let (registry, _db_tmp) = make_registry(&dir);

        let fc = sample_flowchart();
        registry.save(fc.clone()).await.unwrap();

        let loaded = registry.get("flow.test.sample").await.unwrap();
        assert_eq!(loaded.id, "flow.test.sample");
        assert_eq!(loaded.name, "Sample");
        assert!(!loaded.created_at.is_empty());

        // Clean up
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_save_and_discover() {
        let dir = std::env::temp_dir().join(format!("omni_fc_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let (registry, _db_tmp) = make_registry(&dir);

        registry.save(sample_flowchart()).await.unwrap();

        // Create a new registry to test discovery
        let (registry2, _db_tmp2) = make_registry(&dir);
        let discovered = registry2.discover().await.unwrap();
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0], "flow.test.sample");

        let loaded = registry2.get("flow.test.sample").await.unwrap();
        assert_eq!(loaded.name, "Sample");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_delete() {
        let dir = std::env::temp_dir().join(format!("omni_fc_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let (registry, _db_tmp) = make_registry(&dir);

        registry.save(sample_flowchart()).await.unwrap();
        assert!(registry.get("flow.test.sample").await.is_some());

        registry.delete("flow.test.sample").await.unwrap();
        assert!(registry.get("flow.test.sample").await.is_none());

        // File should be gone
        let path = dir.join("flow.test.sample.json");
        assert!(!path.exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_list() {
        let dir = std::env::temp_dir().join(format!("omni_fc_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let (registry, _db_tmp) = make_registry(&dir);

        registry.save(sample_flowchart()).await.unwrap();

        let list = registry.list().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "flow.test.sample");
        assert_eq!(list[0].tool_count, 1);
        assert!(list[0].enabled);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_get_all_tools() {
        let dir = std::env::temp_dir().join(format!("omni_fc_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let (registry, _db_tmp) = make_registry(&dir);

        registry.save(sample_flowchart()).await.unwrap();

        let tools = registry.get_all_tools().await;
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].0, "flow.test.sample");
        assert_eq!(tools[0].1.name, "greet");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_disabled_flowchart_hidden() {
        let dir = std::env::temp_dir().join(format!("omni_fc_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let (registry, _db_tmp) = make_registry(&dir);

        registry.save(sample_flowchart()).await.unwrap();
        registry
            .set_enabled("flow.test.sample", false)
            .await
            .unwrap();

        let tools = registry.get_all_tools().await;
        assert!(tools.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_invoke_tool() {
        let dir = std::env::temp_dir().join(format!("omni_fc_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let (registry, _db_tmp) = make_registry(&dir);

        registry.save(sample_flowchart()).await.unwrap();

        let result = registry
            .invoke_tool(
                "flow.test.sample",
                "greet",
                &serde_json::json!({"name": "World"}),
            )
            .await
            .unwrap();

        assert_eq!(result, serde_json::json!({"result": "Hello, World!"}));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_validate_good_flowchart() {
        let dir = std::env::temp_dir().join(format!("omni_fc_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let (registry, _db_tmp) = make_registry(&dir);

        let issues = registry.validate(&sample_flowchart());
        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.level == ValidationLevel::Error)
            .collect();
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_validate_bad_id() {
        let dir = std::env::temp_dir().join(format!("omni_fc_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let (registry, _db_tmp) = make_registry(&dir);

        let mut fc = sample_flowchart();
        fc.id = "bad".to_string();

        let issues = registry.validate(&fc);
        assert!(issues.iter().any(|i| i.level == ValidationLevel::Error
            && i.message.contains("flow.")));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_validate_missing_trigger() {
        let dir = std::env::temp_dir().join(format!("omni_fc_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let (registry, _db_tmp) = make_registry(&dir);

        let mut fc = sample_flowchart();
        fc.tools[0].trigger_node_id = "nonexistent".to_string();

        let issues = registry.validate(&fc);
        assert!(issues.iter().any(|i| i.level == ValidationLevel::Error
            && i.message.contains("doesn't exist")));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_validate_tool_name_with_dots_rejected() {
        let dir = std::env::temp_dir().join(format!("omni_fc_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let (registry, _db_tmp) = make_registry(&dir);

        let mut fc = sample_flowchart();
        fc.tools[0].name = "send.msg".to_string();

        let issues = registry.validate(&fc);
        assert!(
            issues.iter().any(|i| i.level == ValidationLevel::Error
                && i.message.contains("must not contain dots")),
            "Expected dot-in-tool-name error, got: {:?}",
            issues
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_validate_tool_name_with_underscores_ok() {
        let dir = std::env::temp_dir().join(format!("omni_fc_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let (registry, _db_tmp) = make_registry(&dir);

        let mut fc = sample_flowchart();
        fc.tools[0].name = "send_message".to_string();

        let issues = registry.validate(&fc);
        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.level == ValidationLevel::Error)
            .collect();
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);

        std::fs::remove_dir_all(&dir).ok();
    }
}
