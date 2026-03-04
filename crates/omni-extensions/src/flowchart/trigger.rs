use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tokio::task::JoinHandle;

use omni_core::events::{EventBus, OmniEvent};

use crate::sandbox::GuardianCallback;

use super::registry::FlowchartRegistry;
use super::types::AutoTriggerType;

/// Manages automatic triggers for flowcharts (event-driven, scheduled, webhooks).
///
/// Start the service after flowchart discovery to begin listening for events
/// and running scheduled triggers. Call `reload()` after saving/deleting
/// flowcharts to update active triggers.
pub struct AutoTriggerService {
    registry: Arc<FlowchartRegistry>,
    /// Guardian scanner for pre-execution input scanning on auto-triggered flows.
    guardian: Option<Arc<dyn GuardianCallback>>,
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
    tasks: tokio::sync::Mutex<Vec<JoinHandle<()>>>,
}

impl AutoTriggerService {
    pub fn new(registry: Arc<FlowchartRegistry>) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Self {
            registry,
            guardian: None,
            shutdown_tx,
            shutdown_rx,
            tasks: tokio::sync::Mutex::new(Vec::new()),
        }
    }

    /// Set the Guardian callback for scanning auto-trigger inputs before execution.
    pub fn with_guardian(mut self, cb: Arc<dyn GuardianCallback>) -> Self {
        self.guardian = Some(cb);
        self
    }

    /// Start all auto-triggers from enabled flowcharts.
    pub async fn start(&self, event_bus: &EventBus) {
        let flowcharts = self.registry.list().await;
        let mut tasks = self.tasks.lock().await;

        for summary in &flowcharts {
            if !summary.enabled {
                continue;
            }
            let fc = match self.registry.get(&summary.id).await {
                Some(fc) => fc,
                None => continue,
            };

            for trigger in &fc.auto_triggers {
                if !trigger.enabled {
                    continue;
                }

                match trigger.trigger_type {
                    AutoTriggerType::Event => {
                        let task = self.spawn_event_trigger(
                            event_bus,
                            fc.id.clone(),
                            trigger.tool_name.clone(),
                            trigger.config.clone(),
                        );
                        tasks.push(task);
                    }
                    AutoTriggerType::Schedule => {
                        let task = self.spawn_schedule_trigger(
                            fc.id.clone(),
                            trigger.tool_name.clone(),
                            trigger.config.clone(),
                        );
                        tasks.push(task);
                    }
                    AutoTriggerType::Webhook => {
                        // Webhook triggers are handled by the HTTP server layer
                        // (registered externally via the webhook path in config).
                        // The AutoTriggerService stores the config but the actual
                        // HTTP endpoint is managed by the Tauri app's webhook router.
                        tracing::info!(
                            flowchart = fc.id,
                            path = trigger.config.get("path").and_then(|p| p.as_str()).unwrap_or(""),
                            "Webhook trigger registered (handled by HTTP server)"
                        );
                    }
                }
            }
        }

        tracing::info!(count = tasks.len(), "Auto-triggers started");
    }

    /// Stop all running triggers.
    pub async fn stop(&self) {
        let _ = self.shutdown_tx.send(true);
        let mut tasks = self.tasks.lock().await;
        for task in tasks.drain(..) {
            task.abort();
        }
        // Reset shutdown channel for next start cycle
        let _ = self.shutdown_tx.send(false);
    }

    /// Reload triggers (stop all, re-scan flowcharts, start new ones).
    pub async fn reload(&self, event_bus: &EventBus) {
        self.stop().await;
        self.start(event_bus).await;
    }

    /// Get webhook trigger configs for all enabled flowcharts.
    /// Returns (path, flowchart_id, tool_name, method) tuples for the HTTP server
    /// to register routes.
    pub async fn get_webhook_routes(&self) -> Vec<(String, String, String, String)> {
        let flowcharts = self.registry.list().await;
        let mut routes = Vec::new();

        for summary in &flowcharts {
            if !summary.enabled {
                continue;
            }
            let fc = match self.registry.get(&summary.id).await {
                Some(fc) => fc,
                None => continue,
            };

            for trigger in &fc.auto_triggers {
                if !trigger.enabled || trigger.trigger_type != AutoTriggerType::Webhook {
                    continue;
                }
                let path = trigger
                    .config
                    .get("path")
                    .and_then(|p| p.as_str())
                    .unwrap_or("")
                    .to_string();
                let method = trigger
                    .config
                    .get("method")
                    .and_then(|m| m.as_str())
                    .unwrap_or("POST")
                    .to_string();
                if !path.is_empty() {
                    routes.push((path, fc.id.clone(), trigger.tool_name.clone(), method));
                }
            }
        }

        routes
    }

    // ── Event Trigger ───────────────────────────────────────────────

    fn spawn_event_trigger(
        &self,
        event_bus: &EventBus,
        flowchart_id: String,
        tool_name: String,
        config: serde_json::Value,
    ) -> JoinHandle<()> {
        let mut rx = event_bus.subscribe();
        let mut shutdown = self.shutdown_rx.clone();
        let registry = self.registry.clone();
        let guardian = self.guardian.clone();

        // Parse which event types this trigger listens for
        let event_types: Vec<String> = config
            .get("event_types")
            .and_then(|e| e.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        tracing::info!(
            flowchart = flowchart_id,
            tool = tool_name,
            events = ?event_types,
            "Event trigger started"
        );

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown.changed() => {
                        if *shutdown.borrow() {
                            break;
                        }
                    }
                    result = rx.recv() => {
                        match result {
                            Ok(event) => {
                                let event_type = event_type_name(&event);
                                if event_types.is_empty() || event_types.iter().any(|t| t == event_type) {
                                    let params = event_to_params(&event);

                                    // Guardian: scan the full serialized event params
                                    // AND individual string fields. This catches injection
                                    // payloads in event content (e.g., ChannelMessageReceived.text)
                                    // BEFORE they enter the flowchart where template expansion
                                    // ({{$.params.text}}) could bypass JSON-level scanning.
                                    if let Some(ref guard) = guardian {
                                        let params_str = serde_json::to_string(&params).unwrap_or_default();
                                        if let Err(reason) = guard.scan_input(&params_str) {
                                            tracing::warn!(
                                                flowchart = flowchart_id,
                                                event = event_type,
                                                reason = reason.as_str(),
                                                "Guardian blocked auto-trigger input"
                                            );
                                            continue;
                                        }
                                        // Scan individual string fields that will be consumed
                                        // by template expansion. Payloads can be benign in JSON
                                        // context but dangerous after template interpolation.
                                        let mut field_blocked = false;
                                        if let Some(obj) = params.as_object() {
                                            for (key, val) in obj {
                                                if let Some(text) = val.as_str() {
                                                    if text.len() > 50 {
                                                        if let Err(reason) = guard.scan_input(text) {
                                                            tracing::warn!(
                                                                flowchart = flowchart_id,
                                                                event = event_type,
                                                                field = key.as_str(),
                                                                reason = reason.as_str(),
                                                                "Guardian blocked auto-trigger field"
                                                            );
                                                            field_blocked = true;
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        if field_blocked {
                                            continue;
                                        }
                                    }

                                    match registry
                                        .invoke_tool(&flowchart_id, &tool_name, &params)
                                        .await
                                    {
                                        Ok(_) => {
                                            tracing::debug!(
                                                flowchart = flowchart_id,
                                                event = event_type,
                                                "Event trigger executed successfully"
                                            );
                                        }
                                        Err(e) => {
                                            tracing::warn!(
                                                flowchart = flowchart_id,
                                                event = event_type,
                                                error = %e,
                                                "Event trigger execution failed"
                                            );
                                        }
                                    }
                                }
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                tracing::warn!(lagged = n, "Event trigger lagged behind");
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                break;
                            }
                        }
                    }
                }
            }
        })
    }

    // ── Schedule Trigger ────────────────────────────────────────────

    fn spawn_schedule_trigger(
        &self,
        flowchart_id: String,
        tool_name: String,
        config: serde_json::Value,
    ) -> JoinHandle<()> {
        let mut shutdown = self.shutdown_rx.clone();
        let registry = self.registry.clone();

        let interval_secs = config
            .get("interval_secs")
            .and_then(|i| i.as_u64())
            .unwrap_or(60)
            .max(5); // Minimum 5 seconds to prevent abuse

        tracing::info!(
            flowchart = flowchart_id,
            tool = tool_name,
            interval_secs,
            "Schedule trigger started"
        );

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
            // Skip the first immediate tick
            interval.tick().await;

            loop {
                tokio::select! {
                    _ = shutdown.changed() => {
                        if *shutdown.borrow() {
                            break;
                        }
                    }
                    _ = interval.tick() => {
                        let params = serde_json::json!({
                            "triggered_at": chrono::Utc::now().to_rfc3339(),
                            "trigger_type": "schedule",
                            "interval_secs": interval_secs,
                        });
                        match registry
                            .invoke_tool(&flowchart_id, &tool_name, &params)
                            .await
                        {
                            Ok(_) => {
                                tracing::debug!(
                                    flowchart = flowchart_id,
                                    "Schedule trigger executed successfully"
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    flowchart = flowchart_id,
                                    error = %e,
                                    "Schedule trigger execution failed"
                                );
                            }
                        }
                    }
                }
            }
        })
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Get a string name for an event variant (for matching against event_types config).
fn event_type_name(event: &OmniEvent) -> &'static str {
    match event {
        OmniEvent::MessageReceived { .. } => "MessageReceived",
        OmniEvent::LlmChunk { .. } => "LlmChunk",
        OmniEvent::LlmThinking { .. } => "LlmThinking",
        OmniEvent::LlmComplete { .. } => "LlmComplete",
        OmniEvent::LlmError { .. } => "LlmError",
        OmniEvent::ExtensionInvoked { .. } => "ExtensionInvoked",
        OmniEvent::ExtensionResult { .. } => "ExtensionResult",
        OmniEvent::ExtensionError { .. } => "ExtensionError",
        OmniEvent::PermissionChecked { .. } => "PermissionChecked",
        OmniEvent::PermissionPrompt { .. } => "PermissionPrompt",
        OmniEvent::PermissionRevoked { .. } => "PermissionRevoked",
        OmniEvent::GuardianScan { .. } => "GuardianScan",
        OmniEvent::GuardianBlocked { .. } => "GuardianBlocked",
        OmniEvent::GuardianOverridden { .. } => "GuardianOverridden",
        OmniEvent::ChannelConnected { .. } => "ChannelConnected",
        OmniEvent::ChannelDisconnected { .. } => "ChannelDisconnected",
        OmniEvent::ChannelMessageReceived { .. } => "ChannelMessageReceived",
        OmniEvent::ChannelMessageSent { .. } => "ChannelMessageSent",
        OmniEvent::ChannelError { .. } => "ChannelError",
        OmniEvent::ChannelQrCode { .. } => "ChannelQrCode",
        OmniEvent::Notification { .. } => "Notification",
        OmniEvent::ChannelInstanceCreated { .. } => "ChannelInstanceCreated",
        OmniEvent::ChannelInstanceRemoved { .. } => "ChannelInstanceRemoved",
        OmniEvent::ChannelBindingAdded { .. } => "ChannelBindingAdded",
        OmniEvent::ChannelBindingRemoved { .. } => "ChannelBindingRemoved",
        OmniEvent::AppAutomationAction { .. } => "AppAutomationAction",
        OmniEvent::McpServerConnected { .. } => "McpServerConnected",
        OmniEvent::McpServerDisconnected { .. } => "McpServerDisconnected",
        OmniEvent::McpToolInvoked { .. } => "McpToolInvoked",
        OmniEvent::SubAgentSpawned { .. } => "SubAgentSpawned",
        OmniEvent::SubAgentCompleted { .. } => "SubAgentCompleted",
        OmniEvent::TestRunCompleted { .. } => "TestRunCompleted",
        OmniEvent::ConfigChanged => "ConfigChanged",
        OmniEvent::ExtensionInstalled { .. } => "ExtensionInstalled",
        OmniEvent::ExtensionActivated { .. } => "ExtensionActivated",
        OmniEvent::ExtensionDeactivated { .. } => "ExtensionDeactivated",
        OmniEvent::ExtensionRemoved { .. } => "ExtensionRemoved",
        OmniEvent::FlowchartSaved { .. } => "FlowchartSaved",
        OmniEvent::FlowchartDeleted { .. } => "FlowchartDeleted",
        OmniEvent::FlowchartExecutionStarted { .. } => "FlowchartExecutionStarted",
        OmniEvent::FlowchartExecutionCompleted { .. } => "FlowchartExecutionCompleted",
        OmniEvent::FlowchartNodeExecuted { .. } => "FlowchartNodeExecuted",
        OmniEvent::FlowchartGuardianBlocked { .. } => "FlowchartGuardianBlocked",
        OmniEvent::FlowchartPermissionDenied { .. } => "FlowchartPermissionDenied",
        OmniEvent::FlowchartNodeProgress { .. } => "FlowchartNodeProgress",
        OmniEvent::ExtensionInstanceCreated { .. } => "ExtensionInstanceCreated",
        OmniEvent::ExtensionInstanceDeleted { .. } => "ExtensionInstanceDeleted",
    }
}

/// Convert an event into JSON params for the flowchart trigger.
fn event_to_params(event: &OmniEvent) -> serde_json::Value {
    serde_json::to_value(event).unwrap_or(serde_json::json!({
        "event_type": event_type_name(event),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_names() {
        let event = OmniEvent::ChannelMessageReceived {
            channel_id: "test".to_string(),
            sender: "user".to_string(),
            text: "hello".to_string(),
        };
        assert_eq!(event_type_name(&event), "ChannelMessageReceived");
    }

    #[test]
    fn test_event_to_params() {
        let event = OmniEvent::ChannelConnected {
            channel_id: "discord:main".to_string(),
        };
        let params = event_to_params(&event);
        assert!(params.get("ChannelConnected").is_some() || params.get("channel_id").is_some());
    }
}
