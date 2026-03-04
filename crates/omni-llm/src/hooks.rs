//! Hook system for the AgentLoop.
//!
//! Hooks allow registered callbacks to intercept and modify agent behavior
//! at key points in the conversation loop. Two kinds of hooks exist:
//!
//! - **Modifying hooks** run sequentially and can transform data flowing
//!   through the pipeline (e.g., rewriting user input, filtering tool results).
//! - **Notification hooks** run in parallel and are fire-and-forget
//!   (e.g., logging session start/end).
//!
//! Hooks are ordered by priority (higher = runs first).

use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::types::ChatMessage;

/// Points in the agent loop where hooks can intercept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookPoint {
    /// Before the user message is added to the conversation.
    /// Can modify the user input text.
    MessageReceived,
    /// Before sending the prompt to the LLM.
    /// Can inject or modify messages.
    LlmInput,
    /// After the LLM responds (before tool processing).
    /// Can modify or filter the response text.
    LlmOutput,
    /// Before executing a tool call.
    /// Can block or modify the tool call.
    BeforeToolCall,
    /// After a tool call returns its result.
    /// Can modify the tool result.
    AfterToolCall,
    /// When a new session begins (fire-and-forget).
    SessionStart,
    /// When a session ends (fire-and-forget).
    SessionEnd,
}

impl fmt::Display for HookPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MessageReceived => write!(f, "message_received"),
            Self::LlmInput => write!(f, "llm_input"),
            Self::LlmOutput => write!(f, "llm_output"),
            Self::BeforeToolCall => write!(f, "before_tool_call"),
            Self::AfterToolCall => write!(f, "after_tool_call"),
            Self::SessionStart => write!(f, "session_start"),
            Self::SessionEnd => write!(f, "session_end"),
        }
    }
}

/// Context passed to hooks, containing data they can read and modify.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    /// The hook point being invoked.
    pub hook_point: HookPoint,
    /// Session ID (if available).
    pub session_id: Option<String>,
    /// Text content (user input, LLM output, or tool result).
    pub text: Option<String>,
    /// Tool call info (for before/after_tool_call hooks).
    pub tool_call: Option<ToolCallInfo>,
    /// Current conversation messages (for llm_input hook).
    pub messages: Option<Vec<ChatMessage>>,
    /// Additional metadata.
    pub metadata: serde_json::Value,
}

/// Simplified tool call info passed to hooks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    pub name: String,
    pub arguments: String,
    pub result: Option<String>,
}

/// Result of a modifying hook execution.
#[derive(Debug, Clone)]
pub enum HookResult {
    /// Continue with possibly modified data.
    Continue(HookContext),
    /// Block the operation (e.g., block a tool call).
    Block { reason: String },
}

/// Trait for hook handlers.
#[async_trait]
pub trait HookHandler: Send + Sync {
    /// Unique ID for this hook handler.
    fn id(&self) -> &str;

    /// Priority (higher = runs first). Default is 100.
    fn priority(&self) -> i32 {
        100
    }

    /// Which hook points this handler is interested in.
    fn hook_points(&self) -> Vec<HookPoint>;

    /// Execute the hook. Modifying hooks return a HookResult.
    async fn execute(&self, ctx: HookContext) -> HookResult;
}

/// Registry that manages all registered hooks.
pub struct HookRegistry {
    handlers: RwLock<Vec<Arc<dyn HookHandler>>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(Vec::new()),
        }
    }

    /// Register a hook handler.
    pub async fn register(&self, handler: Arc<dyn HookHandler>) {
        let mut handlers = self.handlers.write().await;
        handlers.push(handler);
        // Sort by priority (highest first)
        handlers.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    /// Unregister a hook handler by ID.
    pub async fn unregister(&self, id: &str) {
        let mut handlers = self.handlers.write().await;
        handlers.retain(|h| h.id() != id);
    }

    /// Run all modifying hooks for a given hook point sequentially.
    /// Returns the final context (possibly modified) or a Block result.
    pub async fn run_modifying(&self, hook_point: HookPoint, mut ctx: HookContext) -> HookResult {
        let handlers = self.handlers.read().await;
        let relevant: Vec<_> = handlers
            .iter()
            .filter(|h| h.hook_points().contains(&hook_point))
            .cloned()
            .collect();
        drop(handlers);

        for handler in relevant {
            match handler.execute(ctx.clone()).await {
                HookResult::Continue(new_ctx) => {
                    ctx = new_ctx;
                }
                HookResult::Block { reason } => {
                    tracing::info!(
                        hook_id = handler.id(),
                        hook_point = %hook_point,
                        reason = reason.as_str(),
                        "Hook blocked operation"
                    );
                    return HookResult::Block { reason };
                }
            }
        }

        HookResult::Continue(ctx)
    }

    /// Run all notification hooks for a given hook point in parallel.
    /// Results are ignored (fire-and-forget).
    pub async fn run_notification(&self, hook_point: HookPoint, ctx: HookContext) {
        let handlers = self.handlers.read().await;
        let relevant: Vec<_> = handlers
            .iter()
            .filter(|h| h.hook_points().contains(&hook_point))
            .cloned()
            .collect();
        drop(handlers);

        let futures: Vec<_> = relevant
            .into_iter()
            .map(|handler| {
                let ctx = ctx.clone();
                tokio::spawn(async move {
                    let _ = handler.execute(ctx).await;
                })
            })
            .collect();

        for f in futures {
            let _ = f.await;
        }
    }

    /// Get the count of registered handlers.
    pub async fn handler_count(&self) -> usize {
        self.handlers.read().await.len()
    }

    /// List registered handler IDs.
    pub async fn handler_ids(&self) -> Vec<String> {
        self.handlers
            .read()
            .await
            .iter()
            .map(|h| h.id().to_string())
            .collect()
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Test hook that uppercases text.
    struct UppercaseHook;

    #[async_trait]
    impl HookHandler for UppercaseHook {
        fn id(&self) -> &str {
            "test.uppercase"
        }
        fn priority(&self) -> i32 {
            100
        }
        fn hook_points(&self) -> Vec<HookPoint> {
            vec![HookPoint::MessageReceived]
        }
        async fn execute(&self, mut ctx: HookContext) -> HookResult {
            if let Some(ref text) = ctx.text {
                ctx.text = Some(text.to_uppercase());
            }
            HookResult::Continue(ctx)
        }
    }

    /// Test hook that blocks messages containing "block".
    struct BlockingHook;

    #[async_trait]
    impl HookHandler for BlockingHook {
        fn id(&self) -> &str {
            "test.blocker"
        }
        fn priority(&self) -> i32 {
            200 // Higher priority, runs first
        }
        fn hook_points(&self) -> Vec<HookPoint> {
            vec![HookPoint::MessageReceived]
        }
        async fn execute(&self, ctx: HookContext) -> HookResult {
            if ctx
                .text
                .as_ref()
                .map(|t| t.contains("block"))
                .unwrap_or(false)
            {
                HookResult::Block {
                    reason: "Message contains 'block'".to_string(),
                }
            } else {
                HookResult::Continue(ctx)
            }
        }
    }

    /// Counter hook for testing notification hooks.
    struct CounterHook {
        count: AtomicU32,
    }

    impl CounterHook {
        fn new() -> Self {
            Self {
                count: AtomicU32::new(0),
            }
        }
    }

    #[async_trait]
    impl HookHandler for CounterHook {
        fn id(&self) -> &str {
            "test.counter"
        }
        fn hook_points(&self) -> Vec<HookPoint> {
            vec![HookPoint::SessionStart, HookPoint::SessionEnd]
        }
        async fn execute(&self, ctx: HookContext) -> HookResult {
            self.count.fetch_add(1, Ordering::Relaxed);
            HookResult::Continue(ctx)
        }
    }

    fn make_ctx(text: &str) -> HookContext {
        HookContext {
            hook_point: HookPoint::MessageReceived,
            session_id: Some("test-session".to_string()),
            text: Some(text.to_string()),
            tool_call: None,
            messages: None,
            metadata: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn test_empty_registry() {
        let registry = HookRegistry::new();
        let ctx = make_ctx("hello");
        let result = registry.run_modifying(HookPoint::MessageReceived, ctx).await;
        match result {
            HookResult::Continue(ctx) => assert_eq!(ctx.text.unwrap(), "hello"),
            HookResult::Block { .. } => panic!("Should not block"),
        }
    }

    #[tokio::test]
    async fn test_modifying_hook() {
        let registry = HookRegistry::new();
        registry.register(Arc::new(UppercaseHook)).await;

        let ctx = make_ctx("hello world");
        let result = registry.run_modifying(HookPoint::MessageReceived, ctx).await;
        match result {
            HookResult::Continue(ctx) => assert_eq!(ctx.text.unwrap(), "HELLO WORLD"),
            HookResult::Block { .. } => panic!("Should not block"),
        }
    }

    #[tokio::test]
    async fn test_blocking_hook() {
        let registry = HookRegistry::new();
        registry.register(Arc::new(BlockingHook)).await;

        // Should block
        let ctx = make_ctx("please block this");
        let result = registry.run_modifying(HookPoint::MessageReceived, ctx).await;
        match result {
            HookResult::Block { reason } => {
                assert!(reason.contains("block"));
            }
            HookResult::Continue(_) => panic!("Should have blocked"),
        }

        // Should pass
        let ctx = make_ctx("hello world");
        let result = registry.run_modifying(HookPoint::MessageReceived, ctx).await;
        assert!(matches!(result, HookResult::Continue(_)));
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let registry = HookRegistry::new();
        // Blocker has higher priority (200), should run before uppercase (100)
        registry.register(Arc::new(UppercaseHook)).await;
        registry.register(Arc::new(BlockingHook)).await;

        // "block" should be caught by the blocker before uppercase runs
        let ctx = make_ctx("block this");
        let result = registry.run_modifying(HookPoint::MessageReceived, ctx).await;
        assert!(matches!(result, HookResult::Block { .. }));
    }

    #[tokio::test]
    async fn test_hook_point_filtering() {
        let registry = HookRegistry::new();
        registry.register(Arc::new(UppercaseHook)).await;

        // UppercaseHook only handles MessageReceived, not LlmInput
        let ctx = HookContext {
            hook_point: HookPoint::LlmInput,
            session_id: None,
            text: Some("hello".to_string()),
            tool_call: None,
            messages: None,
            metadata: serde_json::json!({}),
        };
        let result = registry.run_modifying(HookPoint::LlmInput, ctx).await;
        match result {
            HookResult::Continue(ctx) => assert_eq!(ctx.text.unwrap(), "hello"), // unchanged
            HookResult::Block { .. } => panic!("Should not block"),
        }
    }

    #[tokio::test]
    async fn test_notification_hooks() {
        let counter = Arc::new(CounterHook::new());
        let registry = HookRegistry::new();
        registry.register(counter.clone()).await;

        let ctx = HookContext {
            hook_point: HookPoint::SessionStart,
            session_id: Some("s1".to_string()),
            text: None,
            tool_call: None,
            messages: None,
            metadata: serde_json::json!({}),
        };

        registry.run_notification(HookPoint::SessionStart, ctx).await;
        assert_eq!(counter.count.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_register_and_unregister() {
        let registry = HookRegistry::new();
        registry.register(Arc::new(UppercaseHook)).await;
        registry.register(Arc::new(BlockingHook)).await;
        assert_eq!(registry.handler_count().await, 2);

        registry.unregister("test.uppercase").await;
        assert_eq!(registry.handler_count().await, 1);

        let ids = registry.handler_ids().await;
        assert_eq!(ids, vec!["test.blocker"]);
    }

    #[tokio::test]
    async fn test_hook_point_display() {
        assert_eq!(HookPoint::MessageReceived.to_string(), "message_received");
        assert_eq!(HookPoint::BeforeToolCall.to_string(), "before_tool_call");
        assert_eq!(HookPoint::SessionStart.to_string(), "session_start");
    }

    #[tokio::test]
    async fn test_hook_context_serialization() {
        let ctx = make_ctx("test");
        let json = serde_json::to_string(&ctx).unwrap();
        let deserialized: HookContext = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.text.unwrap(), "test");
        assert_eq!(deserialized.hook_point, HookPoint::MessageReceived);
    }
}
