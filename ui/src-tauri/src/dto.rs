use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct SessionDto {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageDto {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtensionDto {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub enabled: bool,
    pub active: bool,
    pub tools: Vec<String>,
    pub permissions: Vec<String>,
    pub instance_count: usize,
}

/// A single extension instance (child of an installed extension).
#[derive(Debug, Clone, Serialize)]
pub struct ExtensionInstanceDto {
    pub instance_id: String,
    pub extension_id: String,
    pub instance_name: String,
    pub display_name: Option<String>,
    pub enabled: bool,
    pub active: bool,
    pub tools: Vec<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuardianMetricsDto {
    pub scan_count: u64,
    pub block_count: u64,
    pub signature_blocks: u64,
    pub heuristic_blocks: u64,
    pub ml_blocks: u64,
    pub policy_blocks: u64,
    pub avg_scan_ms: f64,
    pub total_scans_db: u64,
    pub total_blocked_db: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditEntryDto {
    pub timestamp: String,
    pub event_type: String,
    pub extension_id: String,
    pub capability: String,
    pub decision: String,
}

/// Channel info for the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct ChannelDto {
    pub id: String,
    pub channel_type: String,
    pub instance_id: String,
    pub name: String,
    pub status: String,
    pub features: ChannelFeaturesDto,
}

/// Summary of a channel type (from a factory).
#[derive(Debug, Clone, Serialize)]
pub struct ChannelTypeDto {
    pub channel_type: String,
    pub name: String,
    pub features: ChannelFeaturesDto,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChannelFeaturesDto {
    pub direct_messages: bool,
    pub group_messages: bool,
    pub media_attachments: bool,
    pub reactions: bool,
    pub read_receipts: bool,
    pub typing_indicators: bool,
    pub threads: bool,
}

/// Channel-extension binding for the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct BindingDto {
    pub id: String,
    pub channel_instance: String,
    pub extension_id: String,
    pub peer_filter: Option<String>,
    pub group_filter: Option<String>,
    pub priority: i32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PendingBlockDto {
    pub scan_id: String,
    pub scan_type: String,
    pub layer: String,
    pub reason: String,
    pub confidence: f64,
    pub content_preview: String,
    pub created_at: String,
}

/// Provider configuration for the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderDto {
    pub id: String,
    pub provider_type: String,
    pub display_name: String,
    pub default_model: Option<String>,
    pub endpoint: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub enabled: bool,
    pub has_credential: bool,
    pub auth_type: String,
    pub env_var_hint: Option<String>,
}

/// Static metadata about a supported provider type.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderTypeInfoDto {
    pub provider_type: String,
    pub display_name: String,
    pub auth_type: String,
    pub env_var_hint: Option<String>,
    pub default_endpoint: Option<String>,
    pub description: String,
}

// ─── Marketplace DTOs ───────────────────────────────────────────

/// Marketplace extension listing (summary for grid cards).
#[derive(Debug, Clone, Serialize)]
pub struct MarketplaceExtensionDto {
    pub id: String,
    pub name: String,
    pub short_description: String,
    pub icon_url: Option<String>,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub trust_level: String,
    pub latest_version: String,
    pub total_downloads: i64,
    pub average_rating: f64,
    pub review_count: i64,
    pub publisher_name: String,
    pub publisher_verified: bool,
}

/// Full marketplace extension detail.
#[derive(Debug, Clone, Serialize)]
pub struct MarketplaceDetailDto {
    pub id: String,
    pub name: String,
    pub short_description: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub trust_level: String,
    pub latest_version: String,
    pub total_downloads: i64,
    pub average_rating: f64,
    pub review_count: i64,
    pub publisher_name: String,
    pub publisher_verified: bool,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    pub min_omni_version: Option<String>,
    pub tools: Vec<String>,
    pub permissions: Vec<String>,
    pub changelog: Option<String>,
    pub screenshots: Vec<String>,
    pub scan_status: Option<String>,
    pub scan_score: Option<f64>,
    pub wasm_size_bytes: Option<i64>,
}

/// Paginated marketplace search result.
#[derive(Debug, Clone, Serialize)]
pub struct MarketplaceSearchResultDto {
    pub extensions: Vec<MarketplaceExtensionDto>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    pub total_pages: i64,
}

/// Marketplace category with extension count.
#[derive(Debug, Clone, Serialize)]
pub struct MarketplaceCategoryDto {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub count: i64,
}

/// Update availability info for an installed extension.
#[derive(Debug, Clone, Serialize)]
pub struct ExtensionUpdateDto {
    pub extension_id: String,
    pub installed_version: String,
    pub latest_version: String,
    pub has_update: bool,
}

// ─── MCP DTOs ──────────────────────────────────────────────────

/// MCP server info for the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct McpServerDto {
    pub name: String,
    pub status: String,
    pub tool_count: usize,
    pub tools: Vec<McpToolDto>,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: Option<String>,
    pub auto_start: bool,
}

/// MCP tool info for the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct McpToolDto {
    pub name: String,
    pub description: Option<String>,
}

// ─── Flowchart DTOs ────────────────────────────────────────────────────

/// Flowchart summary for list view.
#[derive(Debug, Clone, Serialize)]
pub struct FlowchartDto {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub enabled: bool,
    pub tool_count: usize,
    pub permission_count: usize,
}

/// Full flowchart definition for the editor.
/// Nodes and edges are opaque JSON -- React Flow manages the structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowchartDefinitionDto {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub tools: Vec<FlowchartToolDefDto>,
    #[serde(default)]
    pub permissions: Vec<FlowchartPermissionDto>,
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub nodes: Vec<serde_json::Value>,
    #[serde(default)]
    pub edges: Vec<serde_json::Value>,
    pub viewport: Option<serde_json::Value>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowchartToolDefDto {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub trigger_node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowchartPermissionDto {
    pub capability: String,
    pub reason: String,
    #[serde(default = "default_true_fc")]
    pub required: bool,
}

fn default_true_fc() -> bool {
    true
}

/// Trace entry for a single node execution in a test run.
#[derive(Debug, Clone, Serialize)]
pub struct FlowchartNodeTraceDto {
    pub node_id: String,
    pub node_type: String,
    pub label: String,
    pub duration_ms: u64,
    pub error: Option<String>,
    /// The input data snapshot when this node executed (params + variables).
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    pub input: serde_json::Value,
    /// The output produced by this node.
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    pub output: serde_json::Value,
}

/// Test execution result.
#[derive(Debug, Clone, Serialize)]
pub struct FlowchartTestResultDto {
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub node_trace: Vec<FlowchartNodeTraceDto>,
}

/// Validation result.
#[derive(Debug, Clone, Serialize)]
pub struct FlowchartValidationDto {
    pub valid: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}
