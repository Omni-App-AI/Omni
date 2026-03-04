use omni_core::events::{EventBus, OmniEvent};
use tauri::{AppHandle, Emitter};

/// Maps OmniEvent variants to Tauri event names and forwards them to the frontend.
pub async fn run_event_bridge(app: AppHandle, event_bus: EventBus) {
    let mut rx = event_bus.subscribe();

    loop {
        match rx.recv().await {
            Ok(event) => {
                let (event_name, payload) = map_event(&event);
                if let Err(e) = app.emit(event_name, payload) {
                    tracing::warn!("Failed to emit Tauri event '{}': {}", event_name, e);
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("Event bridge lagged, missed {} events", n);
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                tracing::info!("Event bus closed, stopping event bridge");
                break;
            }
        }
    }
}

fn map_event(event: &OmniEvent) -> (&'static str, serde_json::Value) {
    match event {
        OmniEvent::MessageReceived {
            session_id,
            message_id,
        } => (
            "omni:message-received",
            serde_json::json!({ "sessionId": session_id, "messageId": message_id }),
        ),
        OmniEvent::LlmChunk { session_id, chunk } => (
            "omni:llm-chunk",
            serde_json::json!({ "sessionId": session_id, "chunk": chunk }),
        ),
        OmniEvent::LlmThinking { session_id, chunk } => (
            "omni:llm-thinking",
            serde_json::json!({ "sessionId": session_id, "chunk": chunk }),
        ),
        OmniEvent::LlmComplete {
            session_id,
            message_id,
        } => (
            "omni:llm-complete",
            serde_json::json!({ "sessionId": session_id, "messageId": message_id }),
        ),
        OmniEvent::LlmError { session_id, error } => (
            "omni:llm-error",
            serde_json::json!({ "sessionId": session_id, "error": error }),
        ),
        OmniEvent::ExtensionInvoked {
            extension_id,
            tool_name,
            params,
        } => (
            "omni:extension-invoked",
            serde_json::json!({
                "extensionId": extension_id,
                "toolName": tool_name,
                "params": params,
            }),
        ),
        OmniEvent::ExtensionResult {
            extension_id,
            tool_name,
            result,
        } => (
            "omni:extension-result",
            serde_json::json!({
                "extensionId": extension_id,
                "toolName": tool_name,
                "result": result,
            }),
        ),
        OmniEvent::ExtensionError {
            extension_id,
            error,
        } => (
            "omni:extension-error",
            serde_json::json!({ "extensionId": extension_id, "error": error }),
        ),
        OmniEvent::PermissionChecked {
            extension_id,
            capability,
            decision,
        } => (
            "omni:permission-checked",
            serde_json::json!({
                "extensionId": extension_id,
                "capability": capability,
                "decision": decision,
            }),
        ),
        OmniEvent::PermissionPrompt {
            extension_id,
            capability,
            reason,
        } => (
            "omni:permission-prompt",
            serde_json::json!({
                "extensionId": extension_id,
                "capability": capability,
                "reason": reason,
            }),
        ),
        OmniEvent::PermissionRevoked {
            extension_id,
            capability,
        } => (
            "omni:permission-revoked",
            serde_json::json!({ "extensionId": extension_id, "capability": capability }),
        ),
        OmniEvent::GuardianScan {
            scan_type,
            result,
            confidence,
        } => (
            "omni:guardian-scan",
            serde_json::json!({
                "scanType": scan_type,
                "result": result,
                "confidence": confidence,
            }),
        ),
        OmniEvent::GuardianBlocked {
            layer,
            reason,
            content_preview,
        } => (
            "omni:guardian-blocked",
            serde_json::json!({
                "layer": layer,
                "reason": reason,
                "contentPreview": content_preview,
            }),
        ),
        OmniEvent::GuardianOverridden { scan_id } => (
            "omni:guardian-overridden",
            serde_json::json!({ "scanId": scan_id }),
        ),
        OmniEvent::ChannelBindingAdded {
            binding_id,
            channel_instance,
            extension_id,
        } => (
            "omni:channel-binding-added",
            serde_json::json!({
                "bindingId": binding_id,
                "channelInstance": channel_instance,
                "extensionId": extension_id,
            }),
        ),
        OmniEvent::ChannelBindingRemoved { binding_id } => (
            "omni:channel-binding-removed",
            serde_json::json!({ "bindingId": binding_id }),
        ),
        OmniEvent::ConfigChanged => ("omni:config-changed", serde_json::json!({})),
        OmniEvent::ExtensionInstalled { extension_id } => (
            "omni:extension-installed",
            serde_json::json!({ "extensionId": extension_id }),
        ),
        OmniEvent::ExtensionActivated { extension_id } => (
            "omni:extension-activated",
            serde_json::json!({ "extensionId": extension_id }),
        ),
        OmniEvent::ExtensionDeactivated { extension_id } => (
            "omni:extension-deactivated",
            serde_json::json!({ "extensionId": extension_id }),
        ),
        OmniEvent::ExtensionRemoved { extension_id } => (
            "omni:extension-removed",
            serde_json::json!({ "extensionId": extension_id }),
        ),
        OmniEvent::ExtensionInstanceCreated {
            instance_id,
            extension_id,
            instance_name,
        } => (
            "omni:extension-instance-created",
            serde_json::json!({
                "instanceId": instance_id,
                "extensionId": extension_id,
                "instanceName": instance_name,
            }),
        ),
        OmniEvent::ExtensionInstanceDeleted {
            instance_id,
            extension_id,
        } => (
            "omni:extension-instance-deleted",
            serde_json::json!({
                "instanceId": instance_id,
                "extensionId": extension_id,
            }),
        ),
        OmniEvent::ChannelConnected { channel_id } => (
            "omni:channel-connected",
            serde_json::json!({ "channelId": channel_id }),
        ),
        OmniEvent::ChannelDisconnected { channel_id } => (
            "omni:channel-disconnected",
            serde_json::json!({ "channelId": channel_id }),
        ),
        OmniEvent::ChannelMessageReceived {
            channel_id,
            sender,
            text,
        } => (
            "omni:channel-message-received",
            serde_json::json!({
                "channelId": channel_id,
                "sender": sender,
                "text": text,
            }),
        ),
        OmniEvent::ChannelMessageSent {
            channel_id,
            recipient,
        } => (
            "omni:channel-message-sent",
            serde_json::json!({ "channelId": channel_id, "recipient": recipient }),
        ),
        OmniEvent::ChannelError { channel_id, error } => (
            "omni:channel-error",
            serde_json::json!({ "channelId": channel_id, "error": error }),
        ),
        OmniEvent::ChannelQrCode {
            channel_id,
            qr_data,
        } => (
            "omni:channel-qr-code",
            serde_json::json!({ "channelId": channel_id, "qrData": qr_data }),
        ),
        OmniEvent::ChannelInstanceCreated {
            channel_id,
            channel_type,
            instance_id,
        } => (
            "omni:channel-instance-created",
            serde_json::json!({
                "channelId": channel_id,
                "channelType": channel_type,
                "instanceId": instance_id,
            }),
        ),
        OmniEvent::ChannelInstanceRemoved { channel_id } => (
            "omni:channel-instance-removed",
            serde_json::json!({ "channelId": channel_id }),
        ),
        OmniEvent::Notification {
            title,
            body,
            urgency,
        } => (
            "omni:notification",
            serde_json::json!({
                "title": title,
                "body": body,
                "urgency": urgency,
            }),
        ),
        OmniEvent::AppAutomationAction {
            action,
            target_app,
            target_element,
            success,
            error,
        } => (
            "omni:app-automation-action",
            serde_json::json!({
                "action": action,
                "targetApp": target_app,
                "targetElement": target_element,
                "success": success,
                "error": error,
            }),
        ),
        OmniEvent::McpServerConnected {
            server_name,
            tool_count,
        } => (
            "omni:mcp-server-connected",
            serde_json::json!({ "serverName": server_name, "toolCount": tool_count }),
        ),
        OmniEvent::McpServerDisconnected { server_name } => (
            "omni:mcp-server-disconnected",
            serde_json::json!({ "serverName": server_name }),
        ),
        OmniEvent::McpToolInvoked {
            server_name,
            tool_name,
        } => (
            "omni:mcp-tool-invoked",
            serde_json::json!({ "serverName": server_name, "toolName": tool_name }),
        ),
        OmniEvent::SubAgentSpawned { task_id, task } => (
            "omni:sub-agent-spawned",
            serde_json::json!({ "taskId": task_id, "task": task }),
        ),
        OmniEvent::SubAgentCompleted { task_id, success } => (
            "omni:sub-agent-completed",
            serde_json::json!({ "taskId": task_id, "success": success }),
        ),
        OmniEvent::TestRunCompleted {
            framework,
            passed,
            failed,
            skipped,
        } => (
            "omni:test-run-completed",
            serde_json::json!({
                "framework": framework,
                "passed": passed,
                "failed": failed,
                "skipped": skipped,
            }),
        ),
        OmniEvent::FlowchartSaved { flowchart_id } => (
            "omni:flowchart-saved",
            serde_json::json!({ "flowchartId": flowchart_id }),
        ),
        OmniEvent::FlowchartDeleted { flowchart_id } => (
            "omni:flowchart-deleted",
            serde_json::json!({ "flowchartId": flowchart_id }),
        ),
        OmniEvent::FlowchartExecutionStarted {
            flowchart_id,
            tool_name,
        } => (
            "omni:flowchart-execution-started",
            serde_json::json!({ "flowchartId": flowchart_id, "toolName": tool_name }),
        ),
        OmniEvent::FlowchartExecutionCompleted {
            flowchart_id,
            tool_name,
            success,
        } => (
            "omni:flowchart-execution-completed",
            serde_json::json!({
                "flowchartId": flowchart_id,
                "toolName": tool_name,
                "success": success,
            }),
        ),
        OmniEvent::FlowchartNodeExecuted {
            flowchart_id,
            node_id,
            node_type,
            duration_ms,
            success,
        } => (
            "omni:flowchart-node-executed",
            serde_json::json!({
                "flowchartId": flowchart_id,
                "nodeId": node_id,
                "nodeType": node_type,
                "durationMs": duration_ms,
                "success": success,
            }),
        ),
        OmniEvent::FlowchartGuardianBlocked {
            flowchart_id,
            node_id,
            reason,
        } => (
            "omni:flowchart-guardian-blocked",
            serde_json::json!({
                "flowchartId": flowchart_id,
                "nodeId": node_id,
                "reason": reason,
            }),
        ),
        OmniEvent::FlowchartPermissionDenied {
            flowchart_id,
            node_id,
            capability,
        } => (
            "omni:flowchart-permission-denied",
            serde_json::json!({
                "flowchartId": flowchart_id,
                "nodeId": node_id,
                "capability": capability,
            }),
        ),
        OmniEvent::FlowchartNodeProgress {
            flowchart_id,
            node_id,
            chunk,
        } => (
            "omni:flowchart-node-progress",
            serde_json::json!({
                "flowchartId": flowchart_id,
                "nodeId": node_id,
                "chunk": chunk,
            }),
        ),
    }
}
