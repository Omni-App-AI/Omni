use std::collections::HashMap;

use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

/// A complete flowchart definition, stored as JSON on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowchartDefinition {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub tools: Vec<FlowchartToolDef>,
    #[serde(default)]
    pub permissions: Vec<FlowchartPermission>,
    #[serde(default)]
    pub config: HashMap<String, FlowchartConfigField>,
    #[serde(default)]
    pub auto_triggers: Vec<AutoTrigger>,
    #[serde(default)]
    pub nodes: Vec<FlowNode>,
    #[serde(default)]
    pub edges: Vec<FlowEdge>,
    /// React Flow viewport state for UI persistence.
    pub viewport: Option<FlowViewport>,
}

/// A tool exposed by this flowchart (equivalent to ToolDefinition in manifest.rs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowchartToolDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    /// Which trigger node starts this tool's execution.
    pub trigger_node_id: String,
}

/// Permission declaration for a flowchart.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowchartPermission {
    pub capability: String,
    pub reason: String,
    #[serde(default = "default_true")]
    pub required: bool,
}

/// Config field schema for user-facing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowchartConfigField {
    pub field_type: String,
    pub label: String,
    pub help: Option<String>,
    #[serde(default)]
    pub sensitive: bool,
    #[serde(default)]
    pub required: bool,
    pub default: Option<serde_json::Value>,
}

/// React Flow viewport state for editor persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowViewport {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

/// A node in the flowchart graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowNode {
    pub id: String,
    pub node_type: FlowNodeType,
    pub label: String,
    pub position: NodePosition,
    /// Node-specific configuration (varies by type).
    #[serde(default = "serde_json::Value::default")]
    pub config: serde_json::Value,
}

/// Position for React Flow rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

/// All supported node types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FlowNodeType {
    /// Entry point -- receives tool parameters.
    Trigger,
    /// Conditional branching (if/else).
    Condition,
    /// HTTP request action.
    HttpRequest,
    /// LLM inference action.
    LlmRequest,
    /// Send message via channel.
    ChannelSend,
    /// Read/write extension storage.
    StorageOp,
    /// Read extension config.
    ConfigGet,
    /// Data transformation (JSONPath, template, regex).
    Transform,
    /// Combine/merge data from multiple branches.
    Merge,
    /// Loop over array items.
    Loop,
    /// Set a variable for later use.
    SetVariable,
    /// Delay/sleep.
    Delay,
    /// Log a message (for debugging).
    Log,
    /// Return result to caller.
    Output,
    /// Error handler -- catches errors from upstream nodes.
    ErrorHandler,
    /// Execute any registered native tool by name.
    NativeTool,
    /// Call another flowchart as a sub-flow.
    SubFlow,
    /// Multi-way switch/match branching.
    Switch,
    /// Annotation/comment node (no-op, for documentation).
    Comment,
    /// Invoke the full agent loop (multi-turn LLM with tool use).
    AgentRequest,
    /// Check a permission without failing -- routes to "allowed" or "denied" handle.
    PermissionCheck,
}

/// Auto-trigger configuration for event/webhook/schedule-driven flowcharts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoTrigger {
    pub id: String,
    pub trigger_type: AutoTriggerType,
    /// Which tool (trigger node) to invoke when this trigger fires.
    pub tool_name: String,
    /// Trigger-specific config (event_types, path, interval_secs, etc.).
    #[serde(default = "serde_json::Value::default")]
    pub config: serde_json::Value,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Types of automatic triggers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AutoTriggerType {
    /// Triggered by EventBus events (e.g., ChannelMessageReceived).
    Event,
    /// Triggered by incoming HTTP requests to a configured path.
    Webhook,
    /// Triggered on a recurring schedule (interval_secs).
    Schedule,
}

/// An edge connecting two nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    /// Handle label for branching (e.g., "true"/"false" from Condition).
    pub source_handle: Option<String>,
    pub target_handle: Option<String>,
    /// Optional label displayed on the edge.
    pub label: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_flowchart_definition() {
        let def = FlowchartDefinition {
            id: "flow.test.echo".to_string(),
            name: "Echo Tool".to_string(),
            version: "1.0.0".to_string(),
            author: "Test".to_string(),
            description: "Echoes input back".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            enabled: true,
            tools: vec![FlowchartToolDef {
                name: "echo".to_string(),
                description: "Echo the input".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "message": { "type": "string" }
                    },
                    "required": ["message"]
                }),
                trigger_node_id: "trigger_1".to_string(),
            }],
            permissions: vec![],
            config: HashMap::new(),
            auto_triggers: vec![],
            nodes: vec![
                FlowNode {
                    id: "trigger_1".to_string(),
                    node_type: FlowNodeType::Trigger,
                    label: "Start".to_string(),
                    position: NodePosition { x: 0.0, y: 0.0 },
                    config: serde_json::json!({}),
                },
                FlowNode {
                    id: "output_1".to_string(),
                    node_type: FlowNodeType::Output,
                    label: "Return".to_string(),
                    position: NodePosition { x: 0.0, y: 200.0 },
                    config: serde_json::json!({
                        "result_template": "{{$.params.message}}"
                    }),
                },
            ],
            edges: vec![FlowEdge {
                id: "e1".to_string(),
                source: "trigger_1".to_string(),
                target: "output_1".to_string(),
                source_handle: None,
                target_handle: None,
                label: None,
            }],
            viewport: Some(FlowViewport {
                x: 0.0,
                y: 0.0,
                zoom: 1.0,
            }),
        };

        let json = serde_json::to_string_pretty(&def).unwrap();
        let parsed: FlowchartDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "flow.test.echo");
        assert_eq!(parsed.tools.len(), 1);
        assert_eq!(parsed.nodes.len(), 2);
        assert_eq!(parsed.edges.len(), 1);
        assert!(parsed.enabled);
    }

    #[test]
    fn test_node_type_serde() {
        let node_type = FlowNodeType::HttpRequest;
        let json = serde_json::to_string(&node_type).unwrap();
        assert_eq!(json, "\"http_request\"");

        let parsed: FlowNodeType = serde_json::from_str("\"llm_request\"").unwrap();
        assert_eq!(parsed, FlowNodeType::LlmRequest);
    }

    #[test]
    fn test_minimal_definition() {
        let json = r#"{
            "id": "flow.test.min",
            "name": "Minimal",
            "version": "0.1.0",
            "author": "Test",
            "description": "Minimal flowchart"
        }"#;
        let def: FlowchartDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.id, "flow.test.min");
        assert!(def.enabled);
        assert!(def.tools.is_empty());
        assert!(def.nodes.is_empty());
        assert!(def.edges.is_empty());
    }
}
