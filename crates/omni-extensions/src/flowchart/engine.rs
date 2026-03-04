use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::json;

use omni_core::events::{EventBus, OmniEvent};
use omni_permissions::capability::Capability;
use omni_permissions::policy::PolicyEngine;

use crate::sandbox::{AgentCallback, ChannelCallback, FlowchartCallback, GuardianCallback, LlmCallback, LlmProgressReporter, NativeToolCallback};
use crate::storage::{DatabaseStorage, ExtensionStorage};

use super::error::{FlowchartError, Result};
use super::expression::{evaluate_condition, evaluate_path, evaluate_template, ExpressionContext};
use super::types::*;

/// Maximum number of node executions per invocation (infinite-loop protection).
const MAX_NODE_EXECUTIONS: u32 = 500;

/// Default execution timeout.
const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// Maximum delay allowed in a Delay node.
const MAX_DELAY_MS: u64 = 30_000;

/// Maximum sub-flow recursion depth.
const MAX_SUBFLOW_DEPTH: u32 = 10;

/// Maximum retry count per node.
const MAX_RETRY_COUNT: u32 = 10;

/// Default per-node timeout (0 = use global timeout).
const DEFAULT_NODE_TIMEOUT_MS: u64 = 0;

/// Maximum HTTP response body size (matches extension system sandbox limit).
const MAX_HTTP_RESPONSE_BYTES: usize = 5 * 1024 * 1024;

/// Runtime context for a single flowchart execution.
pub struct ExecutionContext {
    /// Outputs from previously executed nodes, keyed by node ID.
    pub node_outputs: HashMap<String, serde_json::Value>,
    /// Named variables set by SetVariable nodes.
    pub variables: HashMap<String, serde_json::Value>,
    /// The initial tool parameters from the trigger.
    pub params: serde_json::Value,
    /// Number of nodes executed so far.
    pub execution_count: u32,
    /// Start time of this execution.
    pub started_at: Instant,
    /// Maximum execution time.
    pub timeout: Duration,
    /// Whether to collect node trace entries (enabled for test_execute).
    pub tracing_enabled: bool,
    /// Collected trace entries (only populated when tracing_enabled is true).
    pub trace: Vec<NodeTraceEntry>,
    /// Current sub-flow recursion depth.
    pub subflow_depth: u32,
}

/// Trace entry for test/debug runs.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NodeTraceEntry {
    pub node_id: String,
    pub node_type: String,
    pub label: String,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
    pub duration_ms: u64,
    pub error: Option<String>,
}

/// Result of a test execution.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TestResult {
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub node_trace: Vec<NodeTraceEntry>,
    pub execution_time_ms: u64,
}

/// The flowchart interpreter engine.
pub struct FlowchartEngine {
    policy_engine: Arc<PolicyEngine>,
    db: Arc<std::sync::Mutex<omni_core::database::Database>>,
    llm_callback: Option<Arc<dyn LlmCallback>>,
    channel_callback: Option<Arc<dyn ChannelCallback>>,
    native_tool_callback: Option<Arc<dyn NativeToolCallback>>,
    /// Uses OnceLock to allow setting after the engine is wrapped in Arc
    /// (needed because the callback references FlowchartRegistry which
    /// contains Arc<FlowchartEngine> -- a chicken-and-egg situation).
    flowchart_callback: std::sync::OnceLock<Arc<dyn FlowchartCallback>>,
    /// Agent callback for AgentRequest nodes -- invokes the full agent loop.
    agent_callback: Option<Arc<dyn AgentCallback>>,
    /// Guardian anti-injection scanner -- scans content at every external boundary
    /// (LLM prompts/responses, native tool params/results, HTTP responses,
    /// channel messages, sub-flow params).
    guardian_callback: Option<Arc<dyn GuardianCallback>>,
    /// EventBus for emitting audit events (node execution, Guardian blocks, permission denials).
    event_bus: Option<EventBus>,
    http_client: reqwest::Client,
}

impl FlowchartEngine {
    pub fn new(
        policy_engine: Arc<PolicyEngine>,
        db: Arc<std::sync::Mutex<omni_core::database::Database>>,
    ) -> Self {
        Self {
            policy_engine,
            db,
            llm_callback: None,
            channel_callback: None,
            native_tool_callback: None,
            flowchart_callback: std::sync::OnceLock::new(),
            agent_callback: None,
            guardian_callback: None,
            event_bus: None,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .redirect(reqwest::redirect::Policy::limited(5))
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn set_llm_callback(&mut self, cb: Arc<dyn LlmCallback>) {
        self.llm_callback = Some(cb);
    }

    pub fn set_channel_callback(&mut self, cb: Arc<dyn ChannelCallback>) {
        self.channel_callback = Some(cb);
    }

    pub fn set_native_tool_callback(&mut self, cb: Arc<dyn NativeToolCallback>) {
        self.native_tool_callback = Some(cb);
    }

    pub fn set_agent_callback(&mut self, cb: Arc<dyn AgentCallback>) {
        self.agent_callback = Some(cb);
    }

    pub fn set_guardian_callback(&mut self, cb: Arc<dyn GuardianCallback>) {
        self.guardian_callback = Some(cb);
    }

    pub fn set_event_bus(&mut self, bus: EventBus) {
        self.event_bus = Some(bus);
    }

    /// Set the flowchart callback. Can be called after the engine is wrapped in Arc.
    /// Only the first call takes effect (OnceLock semantics).
    pub fn set_flowchart_callback(&self, cb: Arc<dyn FlowchartCallback>) {
        let _ = self.flowchart_callback.set(cb);
    }

    /// Execute a flowchart tool, starting from the specified trigger node.
    pub async fn execute(
        &self,
        flowchart: &FlowchartDefinition,
        trigger_node_id: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.execute_with_depth(flowchart, trigger_node_id, params, 0).await
    }

    /// Execute a flowchart tool with an explicit initial sub-flow depth.
    /// Used by FlowchartCallback to propagate recursion depth from parent flows.
    pub async fn execute_with_depth(
        &self,
        flowchart: &FlowchartDefinition,
        trigger_node_id: &str,
        params: serde_json::Value,
        initial_depth: u32,
    ) -> Result<serde_json::Value> {
        // Build lookup maps
        let node_map: HashMap<&str, &FlowNode> =
            flowchart.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        let edge_map = build_edge_map(&flowchart.edges);

        // Find the trigger node
        let trigger = node_map
            .get(trigger_node_id)
            .ok_or_else(|| FlowchartError::TriggerNotFound(trigger_node_id.to_string()))?;

        if trigger.node_type != FlowNodeType::Trigger {
            return Err(FlowchartError::TriggerNotFound(format!(
                "Node '{}' is not a Trigger node",
                trigger_node_id
            )));
        }

        let mut ctx = ExecutionContext {
            node_outputs: HashMap::new(),
            variables: HashMap::new(),
            params: params.clone(),
            execution_count: 0,
            started_at: Instant::now(),
            timeout: Duration::from_millis(DEFAULT_TIMEOUT_MS),
            tracing_enabled: false,
            trace: Vec::new(),
            subflow_depth: initial_depth,
        };

        // Store trigger output (the params themselves)
        ctx.node_outputs
            .insert(trigger_node_id.to_string(), params);

        // Execute the graph starting from the trigger's successors
        self.execute_from(
            trigger_node_id,
            None,
            &node_map,
            &edge_map,
            &mut ctx,
            &flowchart.id,
        )
        .await
    }

    /// Execute a test run with full tracing.
    pub async fn test_execute(
        &self,
        flowchart: &FlowchartDefinition,
        trigger_node_id: &str,
        params: serde_json::Value,
    ) -> TestResult {
        let start = Instant::now();

        // Build lookup maps
        let node_map: HashMap<&str, &FlowNode> =
            flowchart.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        let edge_map = build_edge_map(&flowchart.edges);

        // Find the trigger node
        let trigger = match node_map.get(trigger_node_id) {
            Some(t) if t.node_type == FlowNodeType::Trigger => t,
            _ => {
                return TestResult {
                    success: false,
                    output: None,
                    error: Some(format!("Trigger node not found: {trigger_node_id}")),
                    node_trace: Vec::new(),
                    execution_time_ms: start.elapsed().as_millis() as u64,
                };
            }
        };

        let mut ctx = ExecutionContext {
            node_outputs: HashMap::new(),
            variables: HashMap::new(),
            params: params.clone(),
            execution_count: 0,
            started_at: Instant::now(),
            timeout: Duration::from_millis(DEFAULT_TIMEOUT_MS),
            tracing_enabled: true,
            trace: Vec::new(),
            subflow_depth: 0,
        };

        // Record trigger trace entry
        ctx.trace.push(NodeTraceEntry {
            node_id: trigger.id.clone(),
            node_type: format!("{:?}", trigger.node_type),
            label: trigger.label.clone(),
            input: params.clone(),
            output: params.clone(),
            duration_ms: 0,
            error: None,
        });
        ctx.node_outputs.insert(trigger_node_id.to_string(), params);

        match self
            .execute_from(
                trigger_node_id,
                None,
                &node_map,
                &edge_map,
                &mut ctx,
                &flowchart.id,
            )
            .await
        {
            Ok(output) => TestResult {
                success: true,
                output: Some(output),
                error: None,
                node_trace: ctx.trace,
                execution_time_ms: start.elapsed().as_millis() as u64,
            },
            Err(e) => TestResult {
                success: false,
                output: None,
                error: Some(e.to_string()),
                node_trace: ctx.trace,
                execution_time_ms: start.elapsed().as_millis() as u64,
            },
        }
    }

    /// Execute from a given node, following edges iteratively.
    /// Uses a loop for linear chains and only recurses for branching (Condition/Loop).
    async fn execute_from(
        &self,
        start_node_id: &str,
        start_handle: Option<&str>,
        node_map: &HashMap<&str, &FlowNode>,
        edge_map: &HashMap<String, Vec<&FlowEdge>>,
        ctx: &mut ExecutionContext,
        flowchart_id: &str,
    ) -> Result<serde_json::Value> {
        let mut current_id = start_node_id.to_string();
        let mut current_handle: Option<String> = start_handle.map(|s| s.to_string());

        loop {
            let next_ids = get_successors(
                &current_id,
                current_handle.as_deref(),
                edge_map,
            );

            if next_ids.is_empty() {
                return Ok(serde_json::Value::Null);
            }

            // Fan-out: execute additional successors recursively so their
            // side-effects (variables, node_outputs) persist in ctx.
            // The first successor continues iteratively below.
            if next_ids.len() > 1 {
                for extra_id in &next_ids[1..] {
                    // Execute the extra branch by running execute_node on it
                    // then continuing from there
                    let _ = Box::pin(self.execute_branch_from_node(
                        extra_id,
                        node_map,
                        edge_map,
                        ctx,
                        flowchart_id,
                    ))
                    .await;
                }
            }

            let next_id = &next_ids[0];

            // Safety checks
            ctx.execution_count += 1;
            if ctx.execution_count > MAX_NODE_EXECUTIONS {
                // If we've already seen this node, it's a cycle
                if ctx.node_outputs.contains_key(next_id.as_str()) {
                    return Err(FlowchartError::CycleDetected(next_id.clone()));
                }
                return Err(FlowchartError::MaxDepth(MAX_NODE_EXECUTIONS));
            }
            if ctx.started_at.elapsed() > ctx.timeout {
                return Err(FlowchartError::Timeout(ctx.timeout.as_millis() as u64));
            }

            let node = node_map.get(next_id.as_str()).ok_or_else(|| {
                FlowchartError::NodeExecution {
                    node_id: next_id.clone(),
                    message: "Node not found in graph".to_string(),
                }
            })?;

            // Execute the node with per-node timeout (D3) and retry (D4)
            let node_start = Instant::now();
            let input_snapshot = if ctx.tracing_enabled {
                json!({"params": ctx.params, "variables": ctx.variables})
            } else {
                serde_json::Value::Null
            };

            let node_timeout_ms = node
                .config
                .get("timeout_ms")
                .and_then(|t| t.as_u64())
                .unwrap_or(DEFAULT_NODE_TIMEOUT_MS);
            let retry_count = node
                .config
                .get("retry_count")
                .and_then(|r| r.as_u64())
                .unwrap_or(0)
                .min(MAX_RETRY_COUNT as u64) as u32;
            let retry_delay_ms = node
                .config
                .get("retry_delay_ms")
                .and_then(|d| d.as_u64())
                .unwrap_or(1000);

            let output = match self
                .execute_node_with_timeout_retry(
                    node,
                    ctx,
                    flowchart_id,
                    node_timeout_ms,
                    retry_count,
                    retry_delay_ms,
                )
                .await
            {
                Ok(out) => {
                    let elapsed_ms = node_start.elapsed().as_millis() as u64;
                    if let Some(ref bus) = self.event_bus {
                        bus.emit(OmniEvent::FlowchartNodeExecuted {
                            flowchart_id: flowchart_id.to_string(),
                            node_id: node.id.clone(),
                            node_type: format!("{:?}", node.node_type),
                            duration_ms: elapsed_ms,
                            success: true,
                        });
                    }
                    if ctx.tracing_enabled {
                        ctx.trace.push(NodeTraceEntry {
                            node_id: node.id.clone(),
                            node_type: format!("{:?}", node.node_type),
                            label: node.label.clone(),
                            input: input_snapshot.clone(),
                            output: out.clone(),
                            duration_ms: elapsed_ms,
                            error: None,
                        });
                    }
                    out
                }
                Err(e) => {
                    let elapsed_ms = node_start.elapsed().as_millis() as u64;
                    if let Some(ref bus) = self.event_bus {
                        bus.emit(OmniEvent::FlowchartNodeExecuted {
                            flowchart_id: flowchart_id.to_string(),
                            node_id: node.id.clone(),
                            node_type: format!("{:?}", node.node_type),
                            duration_ms: elapsed_ms,
                            success: false,
                        });
                    }
                    if ctx.tracing_enabled {
                        ctx.trace.push(NodeTraceEntry {
                            node_id: node.id.clone(),
                            node_type: format!("{:?}", node.node_type),
                            label: node.label.clone(),
                            input: input_snapshot,
                            output: serde_json::Value::Null,
                            duration_ms: elapsed_ms,
                            error: Some(e.to_string()),
                        });
                    }
                    // D1: Scoped error handling -- BFS from failed node to find nearest ErrorHandler
                    let error_handler =
                        find_error_handler(next_id, node_map, edge_map);
                    if let Some(handler) = error_handler {
                        let fallback = handler
                            .config
                            .get("fallback_value")
                            .cloned()
                            .unwrap_or_else(|| json!({"error": e.to_string()}));
                        ctx.node_outputs
                            .insert(handler.id.clone(), fallback.clone());
                        current_id = handler.id.clone();
                        current_handle = None;
                        continue;
                    }
                    return Err(e);
                }
            };

            // Handle SetVariable side-effect
            if node.node_type == FlowNodeType::SetVariable {
                if let (Some(var_name), Some(value)) = (
                    output.get("_set_variable").and_then(|n| n.as_str()),
                    output.get("value"),
                ) {
                    ctx.variables.insert(var_name.to_string(), value.clone());
                }
            }

            // Store output
            ctx.node_outputs.insert(next_id.clone(), output.clone());

            match node.node_type {
                FlowNodeType::Output => {
                    return Ok(output);
                }
                FlowNodeType::Condition => {
                    let branch = output
                        .get("branch")
                        .and_then(|b| b.as_str())
                        .unwrap_or("false")
                        .to_string();
                    current_id = next_id.clone();
                    current_handle = Some(branch);
                }
                FlowNodeType::Switch => {
                    let branch = output
                        .get("branch")
                        .and_then(|b| b.as_str())
                        .unwrap_or("default")
                        .to_string();
                    current_id = next_id.clone();
                    current_handle = Some(branch);
                }
                FlowNodeType::PermissionCheck => {
                    let branch = output
                        .get("branch")
                        .and_then(|b| b.as_str())
                        .unwrap_or("denied")
                        .to_string();
                    current_id = next_id.clone();
                    current_handle = Some(branch);
                }
                FlowNodeType::Loop => {
                    let items = output
                        .get("items")
                        .and_then(|i| i.as_array())
                        .cloned()
                        .unwrap_or_default();

                    let max_iter = node
                        .config
                        .get("max_iterations")
                        .and_then(|m| m.as_u64())
                        .unwrap_or(100) as usize;

                    let item_var = "loop_item".to_string();

                    let mut loop_results = Vec::new();
                    for (i, item) in items.iter().enumerate() {
                        if i >= max_iter {
                            break;
                        }
                        ctx.variables.insert(item_var.clone(), item.clone());
                        ctx.variables.insert("loop_index".to_string(), json!(i));

                        // Execute loop body -- this may recurse but loop bodies are short
                        let body_result = Box::pin(self.execute_from(
                            next_id,
                            Some("body"),
                            node_map,
                            edge_map,
                            ctx,
                            flowchart_id,
                        ))
                        .await;
                        match body_result {
                            Ok(val) => loop_results.push(val),
                            Err(e) => {
                                tracing::warn!("Loop iteration {i} error: {e}");
                                loop_results.push(json!({"error": e.to_string()}));
                            }
                        }
                    }

                    ctx.node_outputs
                        .insert(next_id.clone(), json!({"results": loop_results}));

                    // Continue from the "done" handle
                    current_id = next_id.clone();
                    current_handle = Some("done".to_string());
                }
                _ => {
                    // Linear continuation -- iterate instead of recurse
                    current_id = next_id.clone();
                    current_handle = None;
                }
            }
        }
    }

    /// Execute a fan-out branch starting at a specific node ID.
    /// Executes that node, stores its output, and continues from its successors.
    async fn execute_branch_from_node(
        &self,
        node_id: &str,
        node_map: &HashMap<&str, &FlowNode>,
        edge_map: &HashMap<String, Vec<&FlowEdge>>,
        ctx: &mut ExecutionContext,
        flowchart_id: &str,
    ) -> Result<serde_json::Value> {
        ctx.execution_count += 1;
        if ctx.execution_count > MAX_NODE_EXECUTIONS {
            return Err(FlowchartError::MaxDepth(MAX_NODE_EXECUTIONS));
        }
        if ctx.started_at.elapsed() > ctx.timeout {
            return Err(FlowchartError::Timeout(ctx.timeout.as_millis() as u64));
        }

        let node = node_map.get(node_id).ok_or_else(|| {
            FlowchartError::NodeExecution {
                node_id: node_id.to_string(),
                message: "Node not found in graph".to_string(),
            }
        })?;

        let output = self.execute_node(node, ctx, flowchart_id).await?;
        ctx.node_outputs.insert(node_id.to_string(), output.clone());

        if node.node_type == FlowNodeType::Output {
            return Ok(output);
        }

        // Continue from this node using the normal execute_from flow
        self.execute_from(node_id, None, node_map, edge_map, ctx, flowchart_id)
            .await
    }

    /// Execute a single node and return its output.
    async fn execute_node(
        &self,
        node: &FlowNode,
        ctx: &mut ExecutionContext,
        flowchart_id: &str,
    ) -> Result<serde_json::Value> {
        let expr_ctx = ExpressionContext {
            params: &ctx.params,
            node_outputs: &ctx.node_outputs,
            variables: &ctx.variables,
        };

        match node.node_type {
            FlowNodeType::Trigger => {
                // Already handled -- return params
                Ok(ctx.params.clone())
            }

            FlowNodeType::Condition => {
                let expression = node
                    .config
                    .get("expression")
                    .and_then(|e| e.as_str())
                    .unwrap_or("false");
                let result = evaluate_condition(expression, &expr_ctx)?;
                Ok(json!({
                    "branch": if result { "true" } else { "false" },
                    "value": result
                }))
            }

            FlowNodeType::HttpRequest => {
                self.check_permission(flowchart_id, "network.http")?;
                self.execute_http_request(node, &expr_ctx, flowchart_id).await
            }

            FlowNodeType::LlmRequest => {
                self.check_permission(flowchart_id, "ai.inference")?;
                self.execute_llm_request(node, &expr_ctx, flowchart_id).await
            }

            FlowNodeType::ChannelSend => {
                self.check_permission(flowchart_id, "channel.send")?;
                self.execute_channel_send(node, &expr_ctx, flowchart_id).await
            }

            FlowNodeType::StorageOp => {
                self.check_permission(flowchart_id, "storage.persistent")?;
                self.execute_storage_op(node, &expr_ctx, flowchart_id)
            }

            FlowNodeType::ConfigGet => {
                self.execute_config_get(node, &expr_ctx, flowchart_id)
            }

            FlowNodeType::Transform => self.execute_transform(node, &expr_ctx),

            FlowNodeType::Merge => self.execute_merge(node, &expr_ctx),

            FlowNodeType::Loop => {
                // Evaluate the array to iterate over
                let array_path = node
                    .config
                    .get("array_path")
                    .and_then(|p| p.as_str())
                    .unwrap_or("$.params");
                let items = evaluate_path(array_path, &expr_ctx)?;
                Ok(json!({ "items": items }))
            }

            FlowNodeType::SetVariable => {
                let var_name = node
                    .config
                    .get("variable_name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unnamed");
                let value_expr = node
                    .config
                    .get("value_expression")
                    .and_then(|e| e.as_str())
                    .unwrap_or("null");
                let value = evaluate_path(value_expr, &expr_ctx)?;
                // Mutate context through the mutable reference we have
                // (variables are updated after this function returns, or we can use
                // a side-channel; for simplicity we return the name+value pair)
                Ok(json!({
                    "_set_variable": var_name,
                    "value": value
                }))
            }

            FlowNodeType::Delay => {
                let ms = node
                    .config
                    .get("milliseconds")
                    .and_then(|m| m.as_u64())
                    .unwrap_or(1000)
                    .min(MAX_DELAY_MS);
                tokio::time::sleep(Duration::from_millis(ms)).await;
                Ok(json!({ "delayed_ms": ms }))
            }

            FlowNodeType::Log => {
                let msg_template = node
                    .config
                    .get("message_template")
                    .and_then(|m| m.as_str())
                    .unwrap_or("");
                let level = node
                    .config
                    .get("level")
                    .and_then(|l| l.as_str())
                    .unwrap_or("info");
                let message = evaluate_template(msg_template, &expr_ctx)?;
                match level {
                    "error" => tracing::error!(flowchart = flowchart_id, "{}", message),
                    "warn" => tracing::warn!(flowchart = flowchart_id, "{}", message),
                    "debug" => tracing::debug!(flowchart = flowchart_id, "{}", message),
                    _ => tracing::info!(flowchart = flowchart_id, "{}", message),
                }
                Ok(json!({ "logged": message }))
            }

            FlowNodeType::Output => {
                let result_template = node
                    .config
                    .get("result_template")
                    .and_then(|r| r.as_str());

                if let Some(template) = result_template {
                    if template.starts_with("$.") || template.starts_with("$var.") {
                        // JSONPath expression -- return raw value
                        evaluate_path(template, &expr_ctx)
                    } else if template.contains("{{") {
                        // Template string -- return as string
                        let result = evaluate_template(template, &expr_ctx)?;
                        Ok(json!({ "result": result }))
                    } else {
                        // Literal
                        Ok(json!({ "result": template }))
                    }
                } else {
                    // No template -- return the result_value if specified, otherwise
                    // collect all predecessor outputs
                    let result_value = node.config.get("result_value");
                    if let Some(val) = result_value {
                        Ok(val.clone())
                    } else {
                        // Return a merged view of all node outputs
                        Ok(json!(ctx.node_outputs))
                    }
                }
            }

            FlowNodeType::ErrorHandler => {
                let fallback = node
                    .config
                    .get("fallback_value")
                    .cloned()
                    .unwrap_or(json!({}));
                Ok(fallback)
            }

            FlowNodeType::NativeTool => {
                self.execute_native_tool(node, &expr_ctx, flowchart_id)
                    .await
            }

            FlowNodeType::SubFlow => {
                self.check_permission(flowchart_id, "flowchart.invoke")?;
                self.execute_sub_flow(node, &expr_ctx, ctx.subflow_depth, flowchart_id)
                    .await
            }

            FlowNodeType::Switch => self.execute_switch(node, &expr_ctx),

            FlowNodeType::Comment => {
                // No-op annotation node -- just pass through
                Ok(json!({}))
            }

            FlowNodeType::AgentRequest => {
                self.check_permission(flowchart_id, "ai.inference")?;
                self.execute_agent_request(node, &expr_ctx, flowchart_id)
                    .await
            }

            FlowNodeType::PermissionCheck => {
                self.execute_permission_check(node, &expr_ctx, flowchart_id)
            }
        }
    }

    /// Wrap execute_node with per-node timeout and retry logic.
    async fn execute_node_with_timeout_retry(
        &self,
        node: &FlowNode,
        ctx: &mut ExecutionContext,
        flowchart_id: &str,
        timeout_ms: u64,
        retry_count: u32,
        retry_delay_ms: u64,
    ) -> Result<serde_json::Value> {
        let attempts = retry_count + 1; // 0 retries = 1 attempt
        let mut last_err = None;

        for attempt in 0..attempts {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(retry_delay_ms)).await;
            }

            let result = if timeout_ms > 0 {
                match tokio::time::timeout(
                    Duration::from_millis(timeout_ms),
                    self.execute_node(node, ctx, flowchart_id),
                )
                .await
                {
                    Ok(inner) => inner,
                    Err(_) => Err(FlowchartError::NodeTimeout(timeout_ms)),
                }
            } else {
                self.execute_node(node, ctx, flowchart_id).await
            };

            match result {
                Ok(val) => return Ok(val),
                Err(e) => {
                    if attempt + 1 < attempts {
                        tracing::debug!(
                            node_id = node.id,
                            attempt = attempt + 1,
                            max = attempts,
                            "Node failed, retrying: {e}"
                        );
                    }
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap())
    }

    // ── Node action implementations ─────────────────────────────────────

    async fn execute_http_request(
        &self,
        node: &FlowNode,
        ctx: &ExpressionContext<'_>,
        flowchart_id: &str,
    ) -> Result<serde_json::Value> {
        let url_template = node
            .config
            .get("url")
            .and_then(|u| u.as_str())
            .unwrap_or("");
        let url = evaluate_template(url_template, ctx)?;

        // Validate URL -- block empty, non-HTTP(S), and private/loopback addresses
        if url.is_empty() {
            return Err(FlowchartError::NodeExecution {
                node_id: node.id.clone(),
                message: "HTTP request URL is empty".to_string(),
            });
        }
        let parsed_url = url::Url::parse(&url).map_err(|e| FlowchartError::NodeExecution {
            node_id: node.id.clone(),
            message: format!("Invalid URL: {e}"),
        })?;
        match parsed_url.scheme() {
            "http" | "https" => {}
            other => {
                return Err(FlowchartError::NodeExecution {
                    node_id: node.id.clone(),
                    message: format!("URL scheme '{other}' not allowed, only http/https"),
                });
            }
        }
        // SECURITY NOTE: This hostname-based check is vulnerable to DNS rebinding
        // attacks where an attacker controls DNS records that initially resolve to
        // a public IP but later resolve to 127.0.0.1. A full fix would require a
        // custom DNS resolver with pre-connect IP validation. Mitigated by:
        // - Guardian output scanning (catches injection in HTTP responses)
        // - HTTP response size limit (bounds damage from SSRF reads)
        // - Redirect policy limited to 5 hops (set on http_client)
        if let Some(host) = parsed_url.host_str() {
            let is_private = host == "localhost"
                || host == "127.0.0.1"
                || host == "::1"
                || host == "0.0.0.0"
                || host.starts_with("10.")
                || host.starts_with("192.168.")
                || host.starts_with("169.254.")
                || (host.starts_with("172.")
                    && host
                        .split('.')
                        .nth(1)
                        .and_then(|s| s.parse::<u8>().ok())
                        .map(|n| (16..=31).contains(&n))
                        .unwrap_or(false));
            if is_private {
                return Err(FlowchartError::NodeExecution {
                    node_id: node.id.clone(),
                    message: format!("HTTP requests to private/loopback addresses are blocked: {host}"),
                });
            }
        }

        let method = node
            .config
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("GET")
            .to_uppercase();

        let mut builder = match method.as_str() {
            "POST" => self.http_client.post(&url),
            "PUT" => self.http_client.put(&url),
            "DELETE" => self.http_client.delete(&url),
            "PATCH" => self.http_client.patch(&url),
            "HEAD" => self.http_client.head(&url),
            _ => self.http_client.get(&url),
        };

        // Add headers
        if let Some(headers) = node.config.get("headers").and_then(|h| h.as_object()) {
            for (key, value) in headers {
                let val = if let Some(s) = value.as_str() {
                    evaluate_template(s, ctx)?
                } else {
                    value.to_string()
                };
                builder = builder.header(key.as_str(), val);
            }
        }

        // Add body
        if let Some(body_template) = node.config.get("body_template").and_then(|b| b.as_str()) {
            let body = evaluate_template(body_template, ctx)?;
            builder = builder.body(body);
        } else if let Some(body_json) = node.config.get("body_json") {
            // Template-expand string values within the body JSON
            let expanded = expand_json_templates(body_json, ctx)?;
            builder = builder.json(&expanded);
        }

        let response = builder
            .send()
            .await
            .map_err(|e| FlowchartError::Http(e.to_string()))?;

        let status = response.status().as_u16();

        // Enforce response body size limit (matches extension system 5MB cap)
        if let Some(len) = response.content_length() {
            if len > MAX_HTTP_RESPONSE_BYTES as u64 {
                return Err(FlowchartError::NodeExecution {
                    node_id: node.id.clone(),
                    message: format!(
                        "HTTP response too large: {} bytes (max: {} bytes)",
                        len, MAX_HTTP_RESPONSE_BYTES
                    ),
                });
            }
        }

        let body_bytes = response
            .bytes()
            .await
            .map_err(|e| FlowchartError::Http(e.to_string()))?;

        if body_bytes.len() > MAX_HTTP_RESPONSE_BYTES {
            return Err(FlowchartError::NodeExecution {
                node_id: node.id.clone(),
                message: format!(
                    "HTTP response body too large: {} bytes (max: {} bytes)",
                    body_bytes.len(), MAX_HTTP_RESPONSE_BYTES
                ),
            });
        }

        let body = String::from_utf8_lossy(&body_bytes).to_string();

        // Guardian: scan HTTP response body for injection
        self.guardian_scan_output(flowchart_id, &node.id, "http_response", &body)?;

        // Try to parse body as JSON
        let body_value = serde_json::from_str::<serde_json::Value>(&body)
            .unwrap_or_else(|_| json!(body));

        Ok(json!({
            "status": status,
            "body": body_value
        }))
    }

    async fn execute_llm_request(
        &self,
        node: &FlowNode,
        ctx: &ExpressionContext<'_>,
        flowchart_id: &str,
    ) -> Result<serde_json::Value> {
        let cb = self
            .llm_callback
            .as_ref()
            .ok_or_else(|| FlowchartError::CallbackNotAvailable("LLM callback not set".to_string()))?;

        let prompt_template = node
            .config
            .get("prompt_template")
            .and_then(|p| p.as_str())
            .unwrap_or("");
        let prompt = evaluate_template(prompt_template, ctx)?;

        // Guardian: scan assembled LLM prompt for injection
        self.guardian_scan_input(flowchart_id, &node.id, &prompt)?;

        let max_tokens = node
            .config
            .get("max_tokens")
            .and_then(|m| m.as_u64())
            .map(|m| m as u32);

        // Use streaming progress if event bus is available, so the frontend
        // can display incremental LLM output during flowchart execution.
        let response = if let Some(ref bus) = self.event_bus {
            let reporter = FlowchartProgressReporter {
                event_bus: bus.clone(),
                flowchart_id: flowchart_id.to_string(),
                node_id: node.id.clone(),
            };
            cb.request_with_progress(&prompt, max_tokens, &reporter)
        } else {
            cb.request(&prompt, max_tokens)
        }
        .map_err(|e| FlowchartError::NodeExecution {
            node_id: node.id.clone(),
            message: format!("LLM request failed: {e}"),
        })?;

        // Guardian: scan LLM response for injection
        self.guardian_scan_output(flowchart_id, &node.id, "llm_response", &response)?;

        Ok(json!({ "response": response }))
    }

    async fn execute_channel_send(
        &self,
        node: &FlowNode,
        ctx: &ExpressionContext<'_>,
        flowchart_id: &str,
    ) -> Result<serde_json::Value> {
        let cb = self.channel_callback.as_ref().ok_or_else(|| {
            FlowchartError::CallbackNotAvailable("Channel callback not set".to_string())
        })?;

        let channel_id_template = node
            .config
            .get("channel_id")
            .and_then(|c| c.as_str())
            .unwrap_or("");
        let channel_id = evaluate_template(channel_id_template, ctx)?;

        let recipient_template = node
            .config
            .get("recipient_template")
            .and_then(|r| r.as_str())
            .unwrap_or("");
        let recipient = evaluate_template(recipient_template, ctx)?;

        let message_template = node
            .config
            .get("message_template")
            .and_then(|m| m.as_str())
            .unwrap_or("");
        let message = evaluate_template(message_template, ctx)?;

        // Guardian: scan outgoing channel message for injection
        self.guardian_scan_input(flowchart_id, &node.id, &message)?;

        // Sanitize output to prevent XSS in HTML-rendering channels
        let message = Self::sanitize_channel_output(&message);

        let result = cb
            .send_message(&channel_id, &recipient, &message)
            .map_err(|e| FlowchartError::NodeExecution {
                node_id: node.id.clone(),
                message: format!("Channel send failed: {e}"),
            })?;

        Ok(json!({
            "sent": true,
            "channel_id": channel_id,
            "recipient": recipient,
            "response": result
        }))
    }

    fn execute_storage_op(
        &self,
        node: &FlowNode,
        ctx: &ExpressionContext<'_>,
        flowchart_id: &str,
    ) -> Result<serde_json::Value> {
        let storage = DatabaseStorage::new(self.db.clone(), flowchart_id);

        let operation = node
            .config
            .get("operation")
            .and_then(|o| o.as_str())
            .unwrap_or("get");

        let key_template = node
            .config
            .get("key_template")
            .and_then(|k| k.as_str())
            .unwrap_or("");
        let key = evaluate_template(key_template, ctx)?;

        match operation {
            "get" => {
                let value = storage
                    .get(&key)
                    .map_err(|e| FlowchartError::NodeExecution {
                        node_id: node.id.clone(),
                        message: format!("Storage get failed: {e}"),
                    })?;
                Ok(json!({
                    "key": key,
                    "value": value,
                    "found": value.is_some()
                }))
            }
            "set" => {
                let value_template = node
                    .config
                    .get("value_template")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let value = evaluate_template(value_template, ctx)?;
                storage
                    .set(&key, &value)
                    .map_err(|e| FlowchartError::NodeExecution {
                        node_id: node.id.clone(),
                        message: format!("Storage set failed: {e}"),
                    })?;
                Ok(json!({ "key": key, "stored": true }))
            }
            "delete" => {
                let deleted = storage
                    .delete(&key)
                    .map_err(|e| FlowchartError::NodeExecution {
                        node_id: node.id.clone(),
                        message: format!("Storage delete failed: {e}"),
                    })?;
                Ok(json!({ "key": key, "deleted": deleted }))
            }
            _ => Err(FlowchartError::NodeExecution {
                node_id: node.id.clone(),
                message: format!("Unknown storage operation: {operation}"),
            }),
        }
    }

    fn execute_config_get(
        &self,
        node: &FlowNode,
        ctx: &ExpressionContext<'_>,
        flowchart_id: &str,
    ) -> Result<serde_json::Value> {
        let storage = DatabaseStorage::new(self.db.clone(), flowchart_id);

        let key_template = node
            .config
            .get("key")
            .and_then(|k| k.as_str())
            .unwrap_or("");
        let key = evaluate_template(key_template, ctx)?;

        // Config keys use `_config.` prefix (same convention as WASM extensions)
        let config_key = format!("_config.{key}");
        let value = storage
            .get(&config_key)
            .map_err(|e| FlowchartError::NodeExecution {
                node_id: node.id.clone(),
                message: format!("Config get failed: {e}"),
            })?;

        Ok(json!({
            "key": key,
            "value": value
        }))
    }

    fn execute_transform(
        &self,
        node: &FlowNode,
        ctx: &ExpressionContext<'_>,
    ) -> Result<serde_json::Value> {
        let transform_type = node
            .config
            .get("transform_type")
            .and_then(|t| t.as_str())
            .unwrap_or("template");

        let expression = node
            .config
            .get("expression")
            .and_then(|e| e.as_str())
            .unwrap_or("");

        match transform_type {
            "json_path" => {
                let value = evaluate_path(expression, ctx)?;
                Ok(value)
            }
            "template" => {
                let result = evaluate_template(expression, ctx)?;
                Ok(json!(result))
            }
            "regex" => {
                let input_path = node
                    .config
                    .get("input_path")
                    .and_then(|p| p.as_str())
                    .unwrap_or("$.params");
                let input = evaluate_path(input_path, ctx)?;
                let input_str = match &input {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };

                // SECURITY: Rust's `regex` crate uses Thompson NFA / lazy DFA internally,
                // immune to catastrophic backtracking (ReDoS). The 1MB size limit
                // prevents excessive memory from very large compiled patterns.
                let re = regex::RegexBuilder::new(expression)
                    .size_limit(1 << 20) // 1 MB compiled size limit
                    .build()
                    .map_err(|e| {
                        FlowchartError::Expression(format!("Invalid regex: {e}"))
                    })?;

                if let Some(caps) = re.captures(&input_str) {
                    let mut groups = serde_json::Map::new();
                    for (i, name) in re.capture_names().enumerate() {
                        if let Some(m) = caps.get(i) {
                            let key = name
                                .map(|n| n.to_string())
                                .unwrap_or_else(|| i.to_string());
                            groups.insert(key, json!(m.as_str()));
                        }
                    }
                    Ok(json!({
                        "matched": true,
                        "groups": groups,
                        "full_match": caps.get(0).map(|m| m.as_str()).unwrap_or("")
                    }))
                } else {
                    Ok(json!({ "matched": false }))
                }
            }
            "json_build" => {
                // Build a JSON object from a template object
                if let Some(template_obj) = node.config.get("template").and_then(|t| t.as_object())
                {
                    let mut result = serde_json::Map::new();
                    for (key, val_template) in template_obj {
                        if let Some(tmpl) = val_template.as_str() {
                            if tmpl.starts_with("$.") || tmpl.starts_with("$var.") {
                                let val = evaluate_path(tmpl, ctx)?;
                                result.insert(key.clone(), val);
                            } else if tmpl.contains("{{") {
                                let val = evaluate_template(tmpl, ctx)?;
                                result.insert(key.clone(), json!(val));
                            } else {
                                result.insert(key.clone(), val_template.clone());
                            }
                        } else {
                            result.insert(key.clone(), val_template.clone());
                        }
                    }
                    Ok(serde_json::Value::Object(result))
                } else {
                    Ok(json!({}))
                }
            }
            _ => Err(FlowchartError::NodeExecution {
                node_id: node.id.clone(),
                message: format!("Unknown transform type: {transform_type}"),
            }),
        }
    }

    fn execute_merge(
        &self,
        node: &FlowNode,
        ctx: &ExpressionContext<'_>,
    ) -> Result<serde_json::Value> {
        let strategy = node
            .config
            .get("strategy")
            .and_then(|s| s.as_str())
            .unwrap_or("merge_objects");

        let input_paths: Vec<&str> = node
            .config
            .get("input_paths")
            .and_then(|p| p.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let values: Vec<serde_json::Value> = input_paths
            .iter()
            .map(|path| evaluate_path(path, ctx).unwrap_or(serde_json::Value::Null))
            .collect();

        match strategy {
            "merge_objects" => {
                let mut merged = serde_json::Map::new();
                for val in &values {
                    if let Some(obj) = val.as_object() {
                        for (k, v) in obj {
                            merged.insert(k.clone(), v.clone());
                        }
                    }
                }
                Ok(serde_json::Value::Object(merged))
            }
            "array_concat" => {
                let mut arr = Vec::new();
                for val in values {
                    if let Some(items) = val.as_array() {
                        arr.extend(items.iter().cloned());
                    } else {
                        arr.push(val);
                    }
                }
                Ok(json!(arr))
            }
            "first_non_null" => {
                for val in values {
                    if !val.is_null() {
                        return Ok(val);
                    }
                }
                Ok(serde_json::Value::Null)
            }
            _ => Err(FlowchartError::NodeExecution {
                node_id: node.id.clone(),
                message: format!("Unknown merge strategy: {strategy}"),
            }),
        }
    }

    async fn execute_native_tool(
        &self,
        node: &FlowNode,
        ctx: &ExpressionContext<'_>,
        flowchart_id: &str,
    ) -> Result<serde_json::Value> {
        let cb = self.native_tool_callback.as_ref().ok_or_else(|| {
            FlowchartError::CallbackNotAvailable(
                "Native tool callback not set".to_string(),
            )
        })?;

        let tool_name_template = node
            .config
            .get("tool_name")
            .and_then(|t| t.as_str())
            .unwrap_or("");
        let tool_name = evaluate_template(tool_name_template, ctx)?;

        if tool_name.is_empty() {
            return Err(FlowchartError::NodeExecution {
                node_id: node.id.clone(),
                message: "Native tool name is empty".to_string(),
            });
        }

        // Permission check: map native tool to its required capability
        if let Some(capability) = Self::native_tool_capability(&tool_name) {
            self.check_permission(flowchart_id, capability)?;
        }

        // Build params -- either from a JSON template or from individual fields
        let params = if let Some(params_template) = node.config.get("params_template").and_then(|p| p.as_str()) {
            // Template string -- evaluate and parse as JSON
            let expanded = evaluate_template(params_template, ctx)?;
            serde_json::from_str::<serde_json::Value>(&expanded).map_err(|e| {
                FlowchartError::NodeExecution {
                    node_id: node.id.clone(),
                    message: format!("Failed to parse params JSON: {e}"),
                }
            })?
        } else if let Some(params_json) = node.config.get("params_json") {
            // Structured JSON with template expansion in string values
            expand_json_templates(params_json, ctx)?
        } else {
            json!({})
        };

        let params_str = serde_json::to_string(&params).map_err(|e| {
            FlowchartError::NodeExecution {
                node_id: node.id.clone(),
                message: format!("Failed to serialize params: {e}"),
            }
        })?;

        // Guardian: scan native tool params for injection
        self.guardian_scan_input(flowchart_id, &node.id, &params_str)?;

        let cb_clone = cb.clone();
        let tool_name_clone = tool_name.clone();
        let node_id = node.id.clone();

        let result_json = tokio::task::spawn_blocking(move || {
            cb_clone.execute(&tool_name_clone, &params_str)
        })
        .await
        .map_err(|e| FlowchartError::NodeExecution {
            node_id: node_id.clone(),
            message: format!("Native tool task join error: {e}"),
        })?
        .map_err(|e| FlowchartError::NodeExecution {
            node_id: node_id.clone(),
            message: format!("Native tool '{}' failed: {}", tool_name, e),
        })?;

        // Guardian: scan native tool result for injection
        self.guardian_scan_output(flowchart_id, &node.id, "omni.native", &result_json)?;

        // Parse the JSON result
        serde_json::from_str::<serde_json::Value>(&result_json).map_err(|e| {
            FlowchartError::NodeExecution {
                node_id: node.id.clone(),
                message: format!("Failed to parse native tool result: {e}"),
            }
        })
    }

    async fn execute_sub_flow(
        &self,
        node: &FlowNode,
        ctx: &ExpressionContext<'_>,
        current_depth: u32,
        caller_flowchart_id: &str,
    ) -> Result<serde_json::Value> {
        if current_depth >= MAX_SUBFLOW_DEPTH {
            return Err(FlowchartError::SubFlowDepthExceeded(MAX_SUBFLOW_DEPTH));
        }

        let cb = self.flowchart_callback.get().ok_or_else(|| {
            FlowchartError::CallbackNotAvailable(
                "Flowchart callback not set (SubFlow requires it)".to_string(),
            )
        })?;

        let flowchart_id_template = node
            .config
            .get("flowchart_id")
            .and_then(|f| f.as_str())
            .unwrap_or("");
        let flowchart_id = evaluate_template(flowchart_id_template, ctx)?;

        if flowchart_id.is_empty() {
            return Err(FlowchartError::NodeExecution {
                node_id: node.id.clone(),
                message: "SubFlow flowchart_id is empty".to_string(),
            });
        }

        let tool_name_template = node
            .config
            .get("tool_name")
            .and_then(|t| t.as_str())
            .unwrap_or("main");
        let tool_name = evaluate_template(tool_name_template, ctx)?;

        // Build params to pass to the sub-flow
        let mut params = if let Some(params_json) = node.config.get("params_json") {
            expand_json_templates(params_json, ctx)?
        } else {
            json!({})
        };

        // Share parent execution context with the child flow when enabled.
        // The child can access:
        //   - Variables:    {{$.params._parent_variables.some_var}}
        //   - Node outputs: {{$.params._parent_nodes.http_1.body}}
        let share_context = node
            .config
            .get("share_context")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if share_context {
            if let serde_json::Value::Object(ref mut map) = params {
                let parent_vars: serde_json::Map<String, serde_json::Value> = ctx
                    .variables
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                map.insert(
                    "_parent_variables".to_string(),
                    serde_json::Value::Object(parent_vars),
                );

                let parent_nodes: serde_json::Map<String, serde_json::Value> = ctx
                    .node_outputs
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                map.insert(
                    "_parent_nodes".to_string(),
                    serde_json::Value::Object(parent_nodes),
                );
            }
        }

        let params_str = serde_json::to_string(&params).map_err(|e| {
            FlowchartError::NodeExecution {
                node_id: node.id.clone(),
                message: format!("Failed to serialize sub-flow params: {e}"),
            }
        })?;

        // Guardian: scan sub-flow params for injection
        self.guardian_scan_input(caller_flowchart_id, &node.id, &params_str)?;

        let cb_clone = cb.clone();
        let fc_id = flowchart_id.clone();
        let tn = tool_name.clone();
        let node_id = node.id.clone();
        let depth = current_depth + 1;

        let result_json = tokio::task::spawn_blocking(move || {
            cb_clone.invoke(&fc_id, &tn, &params_str, depth)
        })
        .await
        .map_err(|e| FlowchartError::NodeExecution {
            node_id: node_id.clone(),
            message: format!("SubFlow task join error: {e}"),
        })?
        .map_err(|e| FlowchartError::NodeExecution {
            node_id: node_id.clone(),
            message: format!("SubFlow '{}' / '{}' failed: {}", flowchart_id, tool_name, e),
        })?;

        // Guardian: scan sub-flow result for injection
        self.guardian_scan_output(caller_flowchart_id, &node.id, &flowchart_id, &result_json)?;

        serde_json::from_str::<serde_json::Value>(&result_json).map_err(|e| {
            FlowchartError::NodeExecution {
                node_id: node.id.clone(),
                message: format!("Failed to parse sub-flow result: {e}"),
            }
        })
    }

    fn execute_switch(
        &self,
        node: &FlowNode,
        ctx: &ExpressionContext<'_>,
    ) -> Result<serde_json::Value> {
        let expression = node
            .config
            .get("expression")
            .and_then(|e| e.as_str())
            .unwrap_or("");

        // Evaluate the expression to get the value to match against
        let value = evaluate_path(expression, ctx)?;
        let value_str = match &value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Null => "null".to_string(),
            other => other.to_string(),
        };

        // Get the cases from config
        let cases = node
            .config
            .get("cases")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();

        // Find matching case
        let mut branch = "default".to_string();
        for case in &cases {
            let case_value = case
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if case_value == value_str {
                branch = case
                    .get("handle")
                    .and_then(|h| h.as_str())
                    .unwrap_or(case_value)
                    .to_string();
                break;
            }
        }

        Ok(json!({
            "branch": branch,
            "value": value,
            "matched": branch != "default"
        }))
    }

    // ── AgentRequest node ────────────────────────────────────────────────

    async fn execute_agent_request(
        &self,
        node: &FlowNode,
        ctx: &ExpressionContext<'_>,
        flowchart_id: &str,
    ) -> Result<serde_json::Value> {
        let cb = self.agent_callback.as_ref().ok_or_else(|| {
            FlowchartError::CallbackNotAvailable(
                "Agent callback not set (AgentRequest requires it)".to_string(),
            )
        })?;

        let message_template = node
            .config
            .get("message_template")
            .and_then(|m| m.as_str())
            .unwrap_or("");
        let user_message = evaluate_template(message_template, ctx)?;

        if user_message.is_empty() {
            return Err(FlowchartError::NodeExecution {
                node_id: node.id.clone(),
                message: "AgentRequest message is empty".to_string(),
            });
        }

        // Guardian: scan the assembled user message for injection
        self.guardian_scan_input(flowchart_id, &node.id, &user_message)?;

        let system_prompt = node
            .config
            .get("system_prompt")
            .and_then(|s| s.as_str())
            .map(|s| evaluate_template(s, ctx))
            .transpose()?;

        let max_iterations = node
            .config
            .get("max_iterations")
            .and_then(|m| m.as_u64())
            .map(|m| m.min(20) as u32)
            .unwrap_or(5);

        let cb_clone = cb.clone();
        let msg = user_message.clone();
        let sys = system_prompt.clone();
        let node_id = node.id.clone();

        let response = tokio::task::spawn_blocking(move || {
            cb_clone.run(&msg, sys.as_deref(), Some(max_iterations))
        })
        .await
        .map_err(|e| FlowchartError::NodeExecution {
            node_id: node_id.clone(),
            message: format!("Agent task join error: {e}"),
        })?
        .map_err(|e| FlowchartError::NodeExecution {
            node_id: node_id.clone(),
            message: format!("Agent request failed: {e}"),
        })?;

        // Guardian: scan agent response for injection
        self.guardian_scan_output(flowchart_id, &node.id, "agent_response", &response)?;

        Ok(json!({ "response": response }))
    }

    // ── PermissionCheck node ────────────────────────────────────────────

    fn execute_permission_check(
        &self,
        node: &FlowNode,
        ctx: &ExpressionContext<'_>,
        flowchart_id: &str,
    ) -> Result<serde_json::Value> {
        let capability_template = node
            .config
            .get("capability")
            .and_then(|c| c.as_str())
            .unwrap_or("");
        let capability_str = evaluate_template(capability_template, ctx)?;

        if capability_str.is_empty() {
            return Err(FlowchartError::NodeExecution {
                node_id: node.id.clone(),
                message: "PermissionCheck capability is empty".to_string(),
            });
        }

        let capability: Capability = capability_str.parse().map_err(|_| {
            FlowchartError::PermissionDenied(format!("Unknown capability: {capability_str}"))
        })?;

        let allowed = matches!(
            self.policy_engine.check_sync(flowchart_id, &capability),
            omni_permissions::policy::PermissionDecision::Allow
        );

        Ok(json!({
            "branch": if allowed { "allowed" } else { "denied" },
            "capability": capability_str,
            "allowed": allowed
        }))
    }

    // ── Permission warm-up ──────────────────────────────────────────────

    /// Pre-resolve all declared permissions for a flowchart before execution.
    /// This populates the PolicyEngine's cache so that `check_sync()` returns
    /// `Allow` or `Deny` instead of `Prompt` during flow execution.
    pub async fn warm_permissions(&self, flowchart: &FlowchartDefinition) {
        for perm in &flowchart.permissions {
            if let Ok(capability) = perm.capability.parse::<Capability>() {
                // check() is async and populates the cache
                let _ = self.policy_engine.check(&flowchart.id, &capability).await;
            }
        }
    }

    // ── Guardian scanning helpers ────────────────────────────────────────

    /// Scan input content (prompts, params, messages) before sending to external services.
    fn guardian_scan_input(&self, flowchart_id: &str, node_id: &str, content: &str) -> Result<()> {
        if let Some(ref guard) = self.guardian_callback {
            if let Err(reason) = guard.scan_input(content) {
                if let Some(ref bus) = self.event_bus {
                    bus.emit(OmniEvent::FlowchartGuardianBlocked {
                        flowchart_id: flowchart_id.to_string(),
                        node_id: node_id.to_string(),
                        reason: reason.clone(),
                    });
                }
                return Err(FlowchartError::GuardianBlocked {
                    node_id: node_id.to_string(),
                    reason,
                });
            }
        }
        Ok(())
    }

    /// Scan output/result content coming back from external services.
    fn guardian_scan_output(&self, flowchart_id: &str, node_id: &str, source_id: &str, content: &str) -> Result<()> {
        if let Some(ref guard) = self.guardian_callback {
            if let Err(reason) = guard.scan_output(source_id, content) {
                if let Some(ref bus) = self.event_bus {
                    bus.emit(OmniEvent::FlowchartGuardianBlocked {
                        flowchart_id: flowchart_id.to_string(),
                        node_id: node_id.to_string(),
                        reason: reason.clone(),
                    });
                }
                return Err(FlowchartError::GuardianBlocked {
                    node_id: node_id.to_string(),
                    reason,
                });
            }
        }
        Ok(())
    }

    // ── Native tool → capability mapping ────────────────────────────────

    /// Map native tool names to required capability strings for permission checking.
    fn native_tool_capability(tool_name: &str) -> Option<&'static str> {
        match tool_name {
            "exec" => Some("process.spawn"),
            "read_file" | "list_files" | "grep_search" => Some("filesystem.read"),
            "write_file" | "apply_patch" | "edit_file" => Some("filesystem.write"),
            "web_fetch" | "web_search" => Some("network.http"),
            "web_scrape" => Some("browser.scrape"),
            "send_message" | "list_channels" => Some("channel.send"),
            "app_interact" => Some("app.automation"),
            "image_analyze" => Some("ai.inference"),
            "cron_schedule" => Some("system.scheduling"),
            _ => None, // memory_*, session_*, notify, log -- low risk
        }
    }

    /// Sanitize output for HTML-rendering channels to prevent XSS.
    fn sanitize_channel_output(content: &str) -> String {
        content
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }

    // ── Permission checking ─────────────────────────────────────────────

    fn check_permission(&self, flowchart_id: &str, capability_str: &str) -> Result<()> {
        let capability: Capability = capability_str
            .parse()
            .map_err(|_| FlowchartError::PermissionDenied(format!("Unknown capability: {capability_str}")))?;

        match self.policy_engine.check_sync(flowchart_id, &capability) {
            omni_permissions::policy::PermissionDecision::Allow => Ok(()),
            omni_permissions::policy::PermissionDecision::Deny { reason } => {
                if let Some(ref bus) = self.event_bus {
                    bus.emit(OmniEvent::FlowchartPermissionDenied {
                        flowchart_id: flowchart_id.to_string(),
                        node_id: String::new(),
                        capability: capability_str.to_string(),
                    });
                }
                Err(FlowchartError::PermissionDenied(format!(
                    "{capability_str}: {reason}"
                )))
            }
            omni_permissions::policy::PermissionDecision::Prompt { .. } => {
                // check_sync is cache-only -- Prompt means the cache was cold.
                // Log a warning so users know to pre-approve flowchart permissions.
                tracing::warn!(
                    flowchart = flowchart_id,
                    capability = capability_str,
                    "Flowchart permission not cached -- denying (pre-approve via policy rules)"
                );
                if let Some(ref bus) = self.event_bus {
                    bus.emit(OmniEvent::FlowchartPermissionDenied {
                        flowchart_id: flowchart_id.to_string(),
                        node_id: String::new(),
                        capability: capability_str.to_string(),
                    });
                }
                Err(FlowchartError::PermissionDenied(format!(
                    "{capability_str}: requires user approval (add a policy rule to pre-approve)"
                )))
            }
        }
    }
}

// ── Graph traversal helpers ─────────────────────────────────────────────

/// Build a lookup map from source node ID to edges.
fn build_edge_map<'a>(edges: &'a [FlowEdge]) -> HashMap<String, Vec<&'a FlowEdge>> {
    let mut map: HashMap<String, Vec<&FlowEdge>> = HashMap::new();
    for edge in edges {
        map.entry(edge.source.clone())
            .or_default()
            .push(edge);
    }
    map
}

/// Get successor node IDs from a given node, optionally filtered by source handle.
fn get_successors(
    node_id: &str,
    source_handle: Option<&str>,
    edge_map: &HashMap<String, Vec<&FlowEdge>>,
) -> Vec<String> {
    edge_map
        .get(node_id)
        .map(|edges| {
            edges
                .iter()
                .filter(|e| match source_handle {
                    Some(handle) => e.source_handle.as_deref() == Some(handle),
                    None => true,
                })
                .map(|e| e.target.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// D1: Scoped error handling -- BFS from the failed node through all reachable
/// successors to find the nearest ErrorHandler. This means an ErrorHandler catches
/// errors from any node in its upstream branch, not just its immediate predecessor.
fn find_error_handler(
    failed_node_id: &str,
    node_map: &HashMap<&str, &FlowNode>,
    edge_map: &HashMap<String, Vec<&FlowEdge>>,
) -> Option<FlowNode> {
    let mut visited = std::collections::HashSet::new();
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(failed_node_id.to_string());
    visited.insert(failed_node_id.to_string());

    while let Some(current) = queue.pop_front() {
        let successors = get_successors(&current, None, edge_map);
        for sid in successors {
            if visited.contains(&sid) {
                continue;
            }
            visited.insert(sid.clone());
            if let Some(node) = node_map.get(sid.as_str()) {
                if node.node_type == FlowNodeType::ErrorHandler {
                    return Some((*node).clone());
                }
            }
            queue.push_back(sid);
        }
    }
    None
}

/// Recursively expand `{{...}}` templates in all string values within a JSON tree.
fn expand_json_templates(
    value: &serde_json::Value,
    ctx: &ExpressionContext<'_>,
) -> Result<serde_json::Value> {
    match value {
        serde_json::Value::String(s) => {
            if s.contains("{{") {
                Ok(serde_json::Value::String(evaluate_template(s, ctx)?))
            } else {
                Ok(value.clone())
            }
        }
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                out.insert(k.clone(), expand_json_templates(v, ctx)?);
            }
            Ok(serde_json::Value::Object(out))
        }
        serde_json::Value::Array(arr) => {
            let out: std::result::Result<Vec<_>, _> =
                arr.iter().map(|v| expand_json_templates(v, ctx)).collect();
            Ok(serde_json::Value::Array(out?))
        }
        _ => Ok(value.clone()),
    }
}

/// Reports LLM streaming progress as `FlowchartNodeProgress` events.
struct FlowchartProgressReporter {
    event_bus: EventBus,
    flowchart_id: String,
    node_id: String,
}

impl LlmProgressReporter for FlowchartProgressReporter {
    fn on_chunk(&self, text: &str) {
        self.event_bus.emit(OmniEvent::FlowchartNodeProgress {
            flowchart_id: self.flowchart_id.clone(),
            node_id: self.node_id.clone(),
            chunk: text.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omni_permissions::policy::DefaultPolicy;
    use std::sync::Mutex;

    fn make_engine() -> (FlowchartEngine, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("test.db");
        let db = Arc::new(Mutex::new(
            omni_core::database::Database::open(&db_path, "test-key").unwrap(),
        ));
        let policy = Arc::new(PolicyEngine::new(db.clone(), DefaultPolicy::Deny));
        (FlowchartEngine::new(policy, db), tmp)
    }

    fn simple_echo_flowchart() -> FlowchartDefinition {
        FlowchartDefinition {
            id: "flow.test.echo".to_string(),
            name: "Echo".to_string(),
            version: "1.0.0".to_string(),
            author: "Test".to_string(),
            description: "Echo test".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            enabled: true,
            tools: vec![FlowchartToolDef {
                name: "echo".to_string(),
                description: "Echo input".to_string(),
                parameters: json!({"type": "object", "properties": {"msg": {"type": "string"}}}),
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
                    config: json!({}),
                },
                FlowNode {
                    id: "o1".to_string(),
                    node_type: FlowNodeType::Output,
                    label: "Return".to_string(),
                    position: NodePosition { x: 0.0, y: 200.0 },
                    config: json!({
                        "result_template": "$.params.msg"
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
    async fn test_simple_echo() {
        let (engine, _tmp) = make_engine();
        let fc = simple_echo_flowchart();
        let result = engine
            .execute(&fc, "t1", json!({"msg": "hello"}))
            .await
            .unwrap();
        assert_eq!(result, json!("hello"));
    }

    #[tokio::test]
    async fn test_condition_branching() {
        let (engine, _tmp) = make_engine();
        let fc = FlowchartDefinition {
            id: "flow.test.cond".to_string(),
            name: "Condition".to_string(),
            version: "1.0.0".to_string(),
            author: "Test".to_string(),
            description: "Condition test".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            enabled: true,
            tools: vec![],
            permissions: vec![],
            config: HashMap::new(),
            auto_triggers: vec![],
            nodes: vec![
                FlowNode {
                    id: "t1".to_string(),
                    node_type: FlowNodeType::Trigger,
                    label: "Start".to_string(),
                    position: NodePosition { x: 0.0, y: 0.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: "c1".to_string(),
                    node_type: FlowNodeType::Condition,
                    label: "Check".to_string(),
                    position: NodePosition { x: 0.0, y: 100.0 },
                    config: json!({
                        "expression": "$.params.value > 10"
                    }),
                },
                FlowNode {
                    id: "o_true".to_string(),
                    node_type: FlowNodeType::Output,
                    label: "High".to_string(),
                    position: NodePosition { x: -100.0, y: 200.0 },
                    config: json!({
                        "result_value": {"result": "high"}
                    }),
                },
                FlowNode {
                    id: "o_false".to_string(),
                    node_type: FlowNodeType::Output,
                    label: "Low".to_string(),
                    position: NodePosition { x: 100.0, y: 200.0 },
                    config: json!({
                        "result_value": {"result": "low"}
                    }),
                },
            ],
            edges: vec![
                FlowEdge {
                    id: "e1".to_string(),
                    source: "t1".to_string(),
                    target: "c1".to_string(),
                    source_handle: None,
                    target_handle: None,
                    label: None,
                },
                FlowEdge {
                    id: "e2".to_string(),
                    source: "c1".to_string(),
                    target: "o_true".to_string(),
                    source_handle: Some("true".to_string()),
                    target_handle: None,
                    label: Some("Yes".to_string()),
                },
                FlowEdge {
                    id: "e3".to_string(),
                    source: "c1".to_string(),
                    target: "o_false".to_string(),
                    source_handle: Some("false".to_string()),
                    target_handle: None,
                    label: Some("No".to_string()),
                },
            ],
            viewport: None,
        };

        let high = engine
            .execute(&fc, "t1", json!({"value": 20}))
            .await
            .unwrap();
        assert_eq!(high, json!({"result": "high"}));

        let low = engine
            .execute(&fc, "t1", json!({"value": 5}))
            .await
            .unwrap();
        assert_eq!(low, json!({"result": "low"}));
    }

    #[tokio::test]
    async fn test_transform_template() {
        let (engine, _tmp) = make_engine();
        let fc = FlowchartDefinition {
            id: "flow.test.transform".to_string(),
            name: "Transform".to_string(),
            version: "1.0.0".to_string(),
            author: "Test".to_string(),
            description: "Transform test".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            enabled: true,
            tools: vec![],
            permissions: vec![],
            config: HashMap::new(),
            auto_triggers: vec![],
            nodes: vec![
                FlowNode {
                    id: "t1".to_string(),
                    node_type: FlowNodeType::Trigger,
                    label: "Start".to_string(),
                    position: NodePosition { x: 0.0, y: 0.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: "tr1".to_string(),
                    node_type: FlowNodeType::Transform,
                    label: "Format".to_string(),
                    position: NodePosition { x: 0.0, y: 100.0 },
                    config: json!({
                        "transform_type": "template",
                        "expression": "Hello, {{$.params.name}}! Welcome to {{$.params.place}}."
                    }),
                },
                FlowNode {
                    id: "o1".to_string(),
                    node_type: FlowNodeType::Output,
                    label: "Return".to_string(),
                    position: NodePosition { x: 0.0, y: 200.0 },
                    config: json!({
                        "result_template": "$.nodes.tr1"
                    }),
                },
            ],
            edges: vec![
                FlowEdge {
                    id: "e1".to_string(),
                    source: "t1".to_string(),
                    target: "tr1".to_string(),
                    source_handle: None,
                    target_handle: None,
                    label: None,
                },
                FlowEdge {
                    id: "e2".to_string(),
                    source: "tr1".to_string(),
                    target: "o1".to_string(),
                    source_handle: None,
                    target_handle: None,
                    label: None,
                },
            ],
            viewport: None,
        };

        let result = engine
            .execute(&fc, "t1", json!({"name": "Alice", "place": "Omni"}))
            .await
            .unwrap();
        assert_eq!(result, json!("Hello, Alice! Welcome to Omni."));
    }

    #[tokio::test]
    async fn test_set_variable_and_use() {
        let (engine, _tmp) = make_engine();
        let fc = FlowchartDefinition {
            id: "flow.test.var".to_string(),
            name: "Variable".to_string(),
            version: "1.0.0".to_string(),
            author: "Test".to_string(),
            description: "Variable test".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            enabled: true,
            tools: vec![],
            permissions: vec![],
            config: HashMap::new(),
            auto_triggers: vec![],
            nodes: vec![
                FlowNode {
                    id: "t1".to_string(),
                    node_type: FlowNodeType::Trigger,
                    label: "Start".to_string(),
                    position: NodePosition { x: 0.0, y: 0.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: "sv1".to_string(),
                    node_type: FlowNodeType::SetVariable,
                    label: "Set Greeting".to_string(),
                    position: NodePosition { x: 0.0, y: 100.0 },
                    config: json!({
                        "variable_name": "greeting",
                        "value_expression": "$.params.name"
                    }),
                },
                FlowNode {
                    id: "o1".to_string(),
                    node_type: FlowNodeType::Output,
                    label: "Return".to_string(),
                    position: NodePosition { x: 0.0, y: 200.0 },
                    config: json!({
                        "result_template": "Hello, {{$var.greeting}}!"
                    }),
                },
            ],
            edges: vec![
                FlowEdge {
                    id: "e1".to_string(),
                    source: "t1".to_string(),
                    target: "sv1".to_string(),
                    source_handle: None,
                    target_handle: None,
                    label: None,
                },
                FlowEdge {
                    id: "e2".to_string(),
                    source: "sv1".to_string(),
                    target: "o1".to_string(),
                    source_handle: None,
                    target_handle: None,
                    label: None,
                },
            ],
            viewport: None,
        };

        let result = engine
            .execute(&fc, "t1", json!({"name": "Bob"}))
            .await
            .unwrap();
        assert_eq!(result, json!({"result": "Hello, Bob!"}));
    }

    #[tokio::test]
    async fn test_max_depth_enforcement() {
        let (engine, _tmp) = make_engine();
        // Create a cycle: t1 -> a1 -> a1 (self-loop)
        let fc = FlowchartDefinition {
            id: "flow.test.loop".to_string(),
            name: "Infinite".to_string(),
            version: "1.0.0".to_string(),
            author: "Test".to_string(),
            description: "Infinite loop".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            enabled: true,
            tools: vec![],
            permissions: vec![],
            config: HashMap::new(),
            auto_triggers: vec![],
            nodes: vec![
                FlowNode {
                    id: "t1".to_string(),
                    node_type: FlowNodeType::Trigger,
                    label: "Start".to_string(),
                    position: NodePosition { x: 0.0, y: 0.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: "log1".to_string(),
                    node_type: FlowNodeType::Log,
                    label: "Loop".to_string(),
                    position: NodePosition { x: 0.0, y: 100.0 },
                    config: json!({"message_template": "looping", "level": "debug"}),
                },
            ],
            edges: vec![
                FlowEdge {
                    id: "e1".to_string(),
                    source: "t1".to_string(),
                    target: "log1".to_string(),
                    source_handle: None,
                    target_handle: None,
                    label: None,
                },
                FlowEdge {
                    id: "e2".to_string(),
                    source: "log1".to_string(),
                    target: "log1".to_string(),
                    source_handle: None,
                    target_handle: None,
                    label: None,
                },
            ],
            viewport: None,
        };

        let err = engine
            .execute(&fc, "t1", json!({}))
            .await
            .unwrap_err();
        // Self-loop triggers cycle detection (CycleDetected) or max depth (MaxDepth)
        assert!(
            err.to_string().contains("Cycle detected") || err.to_string().contains("Maximum node execution count"),
            "got: {err}"
        );
    }

    #[tokio::test]
    async fn test_trigger_not_found() {
        let (engine, _tmp) = make_engine();
        let fc = simple_echo_flowchart();
        let err = engine
            .execute(&fc, "nonexistent", json!({}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("Trigger node not found"));
    }

    #[tokio::test]
    async fn test_log_node() {
        let (engine, _tmp) = make_engine();
        let fc = FlowchartDefinition {
            id: "flow.test.log".to_string(),
            name: "Log".to_string(),
            version: "1.0.0".to_string(),
            author: "Test".to_string(),
            description: "Log test".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            enabled: true,
            tools: vec![],
            permissions: vec![],
            config: HashMap::new(),
            auto_triggers: vec![],
            nodes: vec![
                FlowNode {
                    id: "t1".to_string(),
                    node_type: FlowNodeType::Trigger,
                    label: "Start".to_string(),
                    position: NodePosition { x: 0.0, y: 0.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: "l1".to_string(),
                    node_type: FlowNodeType::Log,
                    label: "Log".to_string(),
                    position: NodePosition { x: 0.0, y: 100.0 },
                    config: json!({
                        "message_template": "Processing: {{$.params.item}}",
                        "level": "info"
                    }),
                },
                FlowNode {
                    id: "o1".to_string(),
                    node_type: FlowNodeType::Output,
                    label: "Done".to_string(),
                    position: NodePosition { x: 0.0, y: 200.0 },
                    config: json!({"result_value": {"status": "ok"}}),
                },
            ],
            edges: vec![
                FlowEdge {
                    id: "e1".to_string(),
                    source: "t1".to_string(),
                    target: "l1".to_string(),
                    source_handle: None,
                    target_handle: None,
                    label: None,
                },
                FlowEdge {
                    id: "e2".to_string(),
                    source: "l1".to_string(),
                    target: "o1".to_string(),
                    source_handle: None,
                    target_handle: None,
                    label: None,
                },
            ],
            viewport: None,
        };

        let result = engine
            .execute(&fc, "t1", json!({"item": "test-data"}))
            .await
            .unwrap();
        assert_eq!(result, json!({"status": "ok"}));
    }

    #[tokio::test]
    async fn test_test_execute() {
        let (engine, _tmp) = make_engine();
        let fc = simple_echo_flowchart();
        let result = engine
            .test_execute(&fc, "t1", json!({"msg": "test"}))
            .await;
        assert!(result.success);
        assert_eq!(result.output, Some(json!("test")));
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_switch_node() {
        let (engine, _tmp) = make_engine();
        let fc = FlowchartDefinition {
            id: "flow.test.switch".to_string(),
            name: "Switch".to_string(),
            version: "1.0.0".to_string(),
            author: "Test".to_string(),
            description: "Switch test".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            enabled: true,
            tools: vec![],
            permissions: vec![],
            config: HashMap::new(),
            auto_triggers: vec![],
            nodes: vec![
                FlowNode {
                    id: "t1".to_string(),
                    node_type: FlowNodeType::Trigger,
                    label: "Start".to_string(),
                    position: NodePosition { x: 0.0, y: 0.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: "sw1".to_string(),
                    node_type: FlowNodeType::Switch,
                    label: "Route".to_string(),
                    position: NodePosition { x: 0.0, y: 100.0 },
                    config: json!({
                        "expression": "$.params.action",
                        "cases": [
                            {"value": "create", "handle": "create"},
                            {"value": "delete", "handle": "delete"}
                        ]
                    }),
                },
                FlowNode {
                    id: "o_create".to_string(),
                    node_type: FlowNodeType::Output,
                    label: "Create".to_string(),
                    position: NodePosition { x: -100.0, y: 200.0 },
                    config: json!({"result_value": {"action": "created"}}),
                },
                FlowNode {
                    id: "o_delete".to_string(),
                    node_type: FlowNodeType::Output,
                    label: "Delete".to_string(),
                    position: NodePosition { x: 0.0, y: 200.0 },
                    config: json!({"result_value": {"action": "deleted"}}),
                },
                FlowNode {
                    id: "o_default".to_string(),
                    node_type: FlowNodeType::Output,
                    label: "Default".to_string(),
                    position: NodePosition { x: 100.0, y: 200.0 },
                    config: json!({"result_value": {"action": "unknown"}}),
                },
            ],
            edges: vec![
                FlowEdge {
                    id: "e1".to_string(),
                    source: "t1".to_string(),
                    target: "sw1".to_string(),
                    source_handle: None,
                    target_handle: None,
                    label: None,
                },
                FlowEdge {
                    id: "e2".to_string(),
                    source: "sw1".to_string(),
                    target: "o_create".to_string(),
                    source_handle: Some("create".to_string()),
                    target_handle: None,
                    label: Some("Create".to_string()),
                },
                FlowEdge {
                    id: "e3".to_string(),
                    source: "sw1".to_string(),
                    target: "o_delete".to_string(),
                    source_handle: Some("delete".to_string()),
                    target_handle: None,
                    label: Some("Delete".to_string()),
                },
                FlowEdge {
                    id: "e4".to_string(),
                    source: "sw1".to_string(),
                    target: "o_default".to_string(),
                    source_handle: Some("default".to_string()),
                    target_handle: None,
                    label: Some("Default".to_string()),
                },
            ],
            viewport: None,
        };

        let create = engine
            .execute(&fc, "t1", json!({"action": "create"}))
            .await
            .unwrap();
        assert_eq!(create, json!({"action": "created"}));

        let delete = engine
            .execute(&fc, "t1", json!({"action": "delete"}))
            .await
            .unwrap();
        assert_eq!(delete, json!({"action": "deleted"}));

        let unknown = engine
            .execute(&fc, "t1", json!({"action": "update"}))
            .await
            .unwrap();
        assert_eq!(unknown, json!({"action": "unknown"}));
    }

    #[tokio::test]
    async fn test_comment_node() {
        let (engine, _tmp) = make_engine();
        let fc = FlowchartDefinition {
            id: "flow.test.comment".to_string(),
            name: "Comment".to_string(),
            version: "1.0.0".to_string(),
            author: "Test".to_string(),
            description: "Comment test".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            enabled: true,
            tools: vec![],
            permissions: vec![],
            config: HashMap::new(),
            auto_triggers: vec![],
            nodes: vec![
                FlowNode {
                    id: "t1".to_string(),
                    node_type: FlowNodeType::Trigger,
                    label: "Start".to_string(),
                    position: NodePosition { x: 0.0, y: 0.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: "c1".to_string(),
                    node_type: FlowNodeType::Comment,
                    label: "This step does setup".to_string(),
                    position: NodePosition { x: 100.0, y: 100.0 },
                    config: json!({"text": "Documentation note here"}),
                },
                FlowNode {
                    id: "o1".to_string(),
                    node_type: FlowNodeType::Output,
                    label: "Return".to_string(),
                    position: NodePosition { x: 0.0, y: 200.0 },
                    config: json!({"result_value": "ok"}),
                },
            ],
            edges: vec![
                FlowEdge {
                    id: "e1".to_string(),
                    source: "t1".to_string(),
                    target: "c1".to_string(),
                    source_handle: None,
                    target_handle: None,
                    label: None,
                },
                FlowEdge {
                    id: "e2".to_string(),
                    source: "c1".to_string(),
                    target: "o1".to_string(),
                    source_handle: None,
                    target_handle: None,
                    label: None,
                },
            ],
            viewport: None,
        };

        let result = engine.execute(&fc, "t1", json!({})).await.unwrap();
        assert_eq!(result, json!("ok"));
    }

    #[tokio::test]
    async fn test_scoped_error_handler() {
        // D1: ErrorHandler should catch errors from upstream nodes (not just immediate predecessor)
        let (engine, _tmp) = make_engine();
        let fc = FlowchartDefinition {
            id: "flow.test.scoped_err".to_string(),
            name: "ScopedErr".to_string(),
            version: "1.0.0".to_string(),
            author: "Test".to_string(),
            description: "Scoped error handler test".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            enabled: true,
            tools: vec![],
            permissions: vec![],
            config: HashMap::new(),
            auto_triggers: vec![],
            nodes: vec![
                FlowNode {
                    id: "t1".to_string(),
                    node_type: FlowNodeType::Trigger,
                    label: "Start".to_string(),
                    position: NodePosition { x: 0.0, y: 0.0 },
                    config: json!({}),
                },
                // This LLM node will fail because no callback is set
                FlowNode {
                    id: "llm1".to_string(),
                    node_type: FlowNodeType::LlmRequest,
                    label: "LLM".to_string(),
                    position: NodePosition { x: 0.0, y: 100.0 },
                    config: json!({"prompt_template": "test"}),
                },
                FlowNode {
                    id: "o1".to_string(),
                    node_type: FlowNodeType::Output,
                    label: "Output".to_string(),
                    position: NodePosition { x: 0.0, y: 200.0 },
                    config: json!({"result_value": "success"}),
                },
                // ErrorHandler is downstream of Output, not direct successor of LLM
                FlowNode {
                    id: "eh1".to_string(),
                    node_type: FlowNodeType::ErrorHandler,
                    label: "Catch".to_string(),
                    position: NodePosition { x: 100.0, y: 300.0 },
                    config: json!({"fallback_value": {"caught": true}}),
                },
            ],
            edges: vec![
                FlowEdge {
                    id: "e1".to_string(),
                    source: "t1".to_string(),
                    target: "llm1".to_string(),
                    source_handle: None, target_handle: None, label: None,
                },
                FlowEdge {
                    id: "e2".to_string(),
                    source: "llm1".to_string(),
                    target: "o1".to_string(),
                    source_handle: None, target_handle: None, label: None,
                },
                FlowEdge {
                    id: "e3".to_string(),
                    source: "o1".to_string(),
                    target: "eh1".to_string(),
                    source_handle: None, target_handle: None, label: None,
                },
            ],
            viewport: None,
        };

        // LLM fails (no callback), but ErrorHandler is found via BFS
        let result = engine.execute(&fc, "t1", json!({})).await;
        // With scoped error handling, the ErrorHandler should catch the error
        assert!(result.is_ok(), "Expected ErrorHandler to catch error: {:?}", result);
    }

    #[tokio::test]
    async fn test_per_node_timeout() {
        let (engine, _tmp) = make_engine();
        let fc = FlowchartDefinition {
            id: "flow.test.timeout".to_string(),
            name: "Timeout".to_string(),
            version: "1.0.0".to_string(),
            author: "Test".to_string(),
            description: "Per-node timeout test".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            enabled: true,
            tools: vec![],
            permissions: vec![],
            config: HashMap::new(),
            auto_triggers: vec![],
            nodes: vec![
                FlowNode {
                    id: "t1".to_string(),
                    node_type: FlowNodeType::Trigger,
                    label: "Start".to_string(),
                    position: NodePosition { x: 0.0, y: 0.0 },
                    config: json!({}),
                },
                // Delay 5000ms but with 100ms timeout → should fail
                FlowNode {
                    id: "d1".to_string(),
                    node_type: FlowNodeType::Delay,
                    label: "Slow".to_string(),
                    position: NodePosition { x: 0.0, y: 100.0 },
                    config: json!({"milliseconds": 5000, "timeout_ms": 100}),
                },
                FlowNode {
                    id: "o1".to_string(),
                    node_type: FlowNodeType::Output,
                    label: "Done".to_string(),
                    position: NodePosition { x: 0.0, y: 200.0 },
                    config: json!({"result_value": "ok"}),
                },
            ],
            edges: vec![
                FlowEdge {
                    id: "e1".to_string(),
                    source: "t1".to_string(),
                    target: "d1".to_string(),
                    source_handle: None, target_handle: None, label: None,
                },
                FlowEdge {
                    id: "e2".to_string(),
                    source: "d1".to_string(),
                    target: "o1".to_string(),
                    source_handle: None, target_handle: None, label: None,
                },
            ],
            viewport: None,
        };

        let err = engine.execute(&fc, "t1", json!({})).await.unwrap_err();
        assert!(err.to_string().contains("timed out"), "got: {err}");
    }
}
