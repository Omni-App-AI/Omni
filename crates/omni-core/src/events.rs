use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OmniEvent {
    // Message lifecycle
    MessageReceived {
        session_id: String,
        message_id: String,
    },
    LlmChunk {
        session_id: String,
        chunk: String,
    },
    LlmThinking {
        session_id: String,
        chunk: String,
    },
    LlmComplete {
        session_id: String,
        message_id: String,
    },
    LlmError {
        session_id: String,
        error: String,
    },

    // Extension events
    ExtensionInvoked {
        extension_id: String,
        tool_name: String,
        params: serde_json::Value,
    },
    ExtensionResult {
        extension_id: String,
        tool_name: String,
        result: serde_json::Value,
    },
    ExtensionError {
        extension_id: String,
        error: String,
    },

    // Permission events
    PermissionChecked {
        extension_id: String,
        capability: String,
        decision: String,
    },
    PermissionPrompt {
        extension_id: String,
        capability: String,
        reason: String,
    },
    PermissionRevoked {
        extension_id: String,
        capability: String,
    },

    // Guardian events
    GuardianScan {
        scan_type: String,
        result: String,
        confidence: Option<f64>,
    },
    GuardianBlocked {
        layer: String,
        reason: String,
        content_preview: String,
    },
    GuardianOverridden {
        scan_id: String,
    },

    // Channel events
    ChannelConnected {
        channel_id: String,
    },
    ChannelDisconnected {
        channel_id: String,
    },
    ChannelMessageReceived {
        channel_id: String,
        sender: String,
        text: String,
    },
    ChannelMessageSent {
        channel_id: String,
        recipient: String,
    },
    ChannelError {
        channel_id: String,
        error: String,
    },
    ChannelQrCode {
        channel_id: String,
        qr_data: String,
    },

    // Notification events
    Notification {
        title: String,
        body: String,
        urgency: String,
    },

    // Channel instance lifecycle
    ChannelInstanceCreated {
        channel_id: String,
        channel_type: String,
        instance_id: String,
    },
    ChannelInstanceRemoved {
        channel_id: String,
    },

    // Binding events
    ChannelBindingAdded {
        binding_id: String,
        channel_instance: String,
        extension_id: String,
    },
    ChannelBindingRemoved {
        binding_id: String,
    },

    // App automation events
    AppAutomationAction {
        action: String,
        target_app: String,
        target_element: Option<String>,
        success: bool,
        error: Option<String>,
    },

    // MCP events
    McpServerConnected {
        server_name: String,
        tool_count: usize,
    },
    McpServerDisconnected {
        server_name: String,
    },
    McpToolInvoked {
        server_name: String,
        tool_name: String,
    },

    // Sub-agent events
    SubAgentSpawned {
        task_id: String,
        task: String,
    },
    SubAgentCompleted {
        task_id: String,
        success: bool,
    },

    // Test events
    TestRunCompleted {
        framework: String,
        passed: u32,
        failed: u32,
        skipped: u32,
    },

    // System events
    ConfigChanged,
    ExtensionInstalled {
        extension_id: String,
    },
    ExtensionActivated {
        extension_id: String,
    },
    ExtensionDeactivated {
        extension_id: String,
    },
    ExtensionRemoved {
        extension_id: String,
    },
    ExtensionInstanceCreated {
        instance_id: String,
        extension_id: String,
        instance_name: String,
    },
    ExtensionInstanceDeleted {
        instance_id: String,
        extension_id: String,
    },

    // Flowchart events
    FlowchartSaved {
        flowchart_id: String,
    },
    FlowchartDeleted {
        flowchart_id: String,
    },
    FlowchartExecutionStarted {
        flowchart_id: String,
        tool_name: String,
    },
    FlowchartExecutionCompleted {
        flowchart_id: String,
        tool_name: String,
        success: bool,
    },
    /// Emitted after each individual node executes within a flowchart.
    FlowchartNodeExecuted {
        flowchart_id: String,
        node_id: String,
        node_type: String,
        duration_ms: u64,
        success: bool,
    },
    /// Emitted when a Guardian scan blocks content inside a flowchart node.
    FlowchartGuardianBlocked {
        flowchart_id: String,
        node_id: String,
        reason: String,
    },
    /// Emitted when a permission check fails for a flowchart node.
    FlowchartPermissionDenied {
        flowchart_id: String,
        node_id: String,
        capability: String,
    },
    /// Emitted for streaming progress during LLM/agent requests within flowcharts.
    FlowchartNodeProgress {
        flowchart_id: String,
        node_id: String,
        chunk: String,
    },
}

#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<OmniEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn emit(&self, event: OmniEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<OmniEvent> {
        self.sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_emit_and_receive() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();

        bus.emit(OmniEvent::ConfigChanged);

        let event = rx.recv().await.unwrap();
        assert!(matches!(event, OmniEvent::ConfigChanged));
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let bus = EventBus::new(16);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        bus.emit(OmniEvent::ConfigChanged);

        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();
        assert!(matches!(e1, OmniEvent::ConfigChanged));
        assert!(matches!(e2, OmniEvent::ConfigChanged));
    }

    #[tokio::test]
    async fn test_no_subscriber_doesnt_panic() {
        let bus = EventBus::new(16);
        bus.emit(OmniEvent::ConfigChanged); // should not panic
    }

    #[tokio::test]
    async fn test_structured_event() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();

        bus.emit(OmniEvent::MessageReceived {
            session_id: "sess-1".to_string(),
            message_id: "msg-1".to_string(),
        });

        let event = rx.recv().await.unwrap();
        match event {
            OmniEvent::MessageReceived {
                session_id,
                message_id,
            } => {
                assert_eq!(session_id, "sess-1");
                assert_eq!(message_id, "msg-1");
            }
            _ => panic!("Expected MessageReceived"),
        }
    }
}
