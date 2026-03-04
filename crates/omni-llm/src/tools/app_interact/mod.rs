//! Desktop app automation tool.
//!
//! Opens executables and interacts with their UI elements via platform-native
//! UI Automation APIs (Windows UIA, macOS Accessibility, Linux AT-SPI2).
//!
//! Security-first design: LOLBIN blocklist, password field blocking,
//! per-app rate limiting, semantic actions only (no raw tree traversal or
//! coordinate-based input).

pub mod platform;
pub mod security;
pub mod stub;
pub mod types;

#[cfg(windows)]
pub mod windows;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use omni_core::events::{EventBus, OmniEvent};
use omni_permissions::capability::{AppAutomationScope, Capability};

use self::platform::UiAutomationBackend;
use self::security::SecurityGuard;
use self::types::{ManagedProcess, RateLimiterState};
use crate::error::{LlmError, Result};
use crate::tools::NativeTool;

/// Native tool for interacting with desktop applications via UI Automation.
pub struct AppInteractTool {
    backend: Arc<dyn UiAutomationBackend>,
    security: SecurityGuard,
    /// Configured scope for restricting allowed apps, actions, rate limits, etc.
    scope: Option<AppAutomationScope>,
    /// EventBus for emitting audit events.
    event_bus: Option<Arc<EventBus>>,
    /// Tracks managed processes by PID.
    managed_processes: Arc<Mutex<HashMap<u32, ManagedProcess>>>,
    /// Per-app rate limiters.
    rate_limiters: Arc<Mutex<HashMap<String, RateLimiterState>>>,
}

impl AppInteractTool {
    pub fn new() -> Self {
        #[cfg(windows)]
        let backend: Arc<dyn UiAutomationBackend> = Arc::new(windows::WindowsUiaBackend::new());
        #[cfg(not(windows))]
        let backend: Arc<dyn UiAutomationBackend> = Arc::new(stub::StubBackend);

        Self {
            backend,
            security: SecurityGuard::new(),
            scope: None,
            event_bus: None,
            managed_processes: Arc::new(Mutex::new(HashMap::new())),
            rate_limiters: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create with an explicit scope and event bus for full security enforcement.
    pub fn with_config(
        scope: Option<AppAutomationScope>,
        event_bus: Option<Arc<EventBus>>,
    ) -> Self {
        let mut tool = Self::new();
        tool.scope = scope;
        tool.event_bus = event_bus;
        tool
    }

    /// Get the app key for rate limiting (process_name or window_title).
    fn app_key(params: &serde_json::Value) -> String {
        params["process_name"]
            .as_str()
            .or(params["window_title"].as_str())
            .or(params["executable"].as_str())
            .unwrap_or("unknown")
            .to_string()
    }

    /// Check rate limit for the current action.
    fn check_rate_limit(&self, app_key: &str) -> Result<()> {
        let max_rate = SecurityGuard::rate_limit_for_scope(&self.scope);
        let mut limiters = self.rate_limiters.lock().map_err(|_| {
            LlmError::ToolCall("Internal error: rate limiter lock poisoned".to_string())
        })?;
        SecurityGuard::check_rate_limit(&mut limiters, app_key, max_rate)
    }

    /// Emit an audit event if EventBus is available.
    fn emit_audit(
        &self,
        action: &str,
        target_app: &str,
        target_element: Option<&str>,
        success: bool,
        error: Option<&str>,
    ) {
        if let Some(ref bus) = self.event_bus {
            bus.emit(OmniEvent::AppAutomationAction {
                action: action.to_string(),
                target_app: target_app.to_string(),
                target_element: target_element.map(String::from),
                success,
                error: error.map(String::from),
            });
        }
    }

    /// Check element_ref for sensitive element names. Used by click, type_text, read_text.
    fn check_sensitive_element_ref(&self, element_ref: &str, action_verb: &str) -> Result<()> {
        for part in element_ref.split('|') {
            if let Some(name) = part.strip_prefix("name:") {
                if self.security.is_sensitive_element(name) {
                    return Err(LlmError::ToolCall(format!(
                        "Cannot {} sensitive element '{}' (security restriction)",
                        action_verb, name
                    )));
                }
            }
        }
        Ok(())
    }

    async fn handle_launch(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let executable = params["executable"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'executable' is required for launch".to_string()))?
            .to_string();

        let args: Vec<String> = params["args"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        // Security: validate executable against LOLBIN blocklist + scope allowlist
        self.security.validate_launch(&executable, &self.scope)?;

        // Check max concurrent processes
        {
            let processes = self.managed_processes.lock().map_err(|_| {
                LlmError::ToolCall("Internal error: process lock poisoned".to_string())
            })?;
            let max = SecurityGuard::max_concurrent_for_scope(&self.scope);
            if processes.len() >= max as usize {
                return Err(LlmError::ToolCall(format!(
                    "Maximum concurrent processes reached ({}). Close an app before launching another.",
                    max
                )));
            }
        }

        let backend = self.backend.clone();
        let exe_clone = executable.clone();
        let result = tokio::task::spawn_blocking(move || backend.launch_app(&exe_clone, &args))
            .await
            .map_err(|e| LlmError::ToolCall(format!("Task join error: {e}")))?;

        match &result {
            Ok(launched) => {
                self.emit_audit("launch", &executable, None, true, None);

                if let Ok(mut processes) = self.managed_processes.lock() {
                    processes.insert(
                        launched.pid,
                        ManagedProcess {
                            pid: launched.pid,
                            executable: executable.clone(),
                            launched_at: std::time::Instant::now(),
                        },
                    );
                }
            }
            Err(e) => {
                self.emit_audit("launch", &executable, None, false, Some(&e.to_string()));
            }
        }

        let launched = result?;
        Ok(serde_json::to_value(&launched).unwrap_or_default())
    }

    async fn handle_list_windows(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let process_name = params["process_name"].as_str().map(String::from);
        let app_key = Self::app_key(&params);

        let backend = self.backend.clone();
        let result =
            tokio::task::spawn_blocking(move || backend.list_windows(process_name.as_deref()))
                .await
                .map_err(|e| LlmError::ToolCall(format!("Task join error: {e}")))?;

        match &result {
            Ok(_) => self.emit_audit("list_windows", &app_key, None, true, None),
            Err(e) => self.emit_audit("list_windows", &app_key, None, false, Some(&e.to_string())),
        }

        let windows = result?;
        Ok(serde_json::json!({
            "windows": windows,
            "count": windows.len(),
        }))
    }

    async fn handle_find_element(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let window_title = params["window_title"].as_str().map(String::from);
        let process_name = params["process_name"].as_str().map(String::from);
        let element_name = params["element_name"].as_str().map(String::from);
        let element_type = params["element_type"].as_str().map(String::from);
        let automation_id = params["automation_id"].as_str().map(String::from);
        let timeout_ms = params["timeout_ms"].as_u64().unwrap_or(5000).min(30000);
        let app_key = Self::app_key(&params);

        let backend = self.backend.clone();
        let result = tokio::task::spawn_blocking(move || {
            backend.find_element(
                window_title.as_deref(),
                process_name.as_deref(),
                element_name.as_deref(),
                element_type.as_deref(),
                automation_id.as_deref(),
                timeout_ms,
            )
        })
        .await
        .map_err(|e| LlmError::ToolCall(format!("Task join error: {e}")))?;

        match &result {
            Ok(found) => self.emit_audit("find_element", &app_key, Some(&found.name), true, None),
            Err(e) => self.emit_audit("find_element", &app_key, None, false, Some(&e.to_string())),
        }

        let found = result?;
        Ok(serde_json::to_value(&found).unwrap_or_default())
    }

    async fn handle_find_elements(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let window_title = params["window_title"].as_str().map(String::from);
        let process_name = params["process_name"].as_str().map(String::from);
        let element_name = params["element_name"].as_str().map(String::from);
        let element_type = params["element_type"].as_str().map(String::from);
        let automation_id = params["automation_id"].as_str().map(String::from);
        let timeout_ms = params["timeout_ms"].as_u64().unwrap_or(5000).min(30000);
        let max_results = params["max_results"].as_u64().unwrap_or(20).min(100) as u32;
        let app_key = Self::app_key(&params);

        let backend = self.backend.clone();
        let result = tokio::task::spawn_blocking(move || {
            backend.find_elements(
                window_title.as_deref(),
                process_name.as_deref(),
                element_name.as_deref(),
                element_type.as_deref(),
                automation_id.as_deref(),
                timeout_ms,
                max_results,
            )
        })
        .await
        .map_err(|e| LlmError::ToolCall(format!("Task join error: {e}")))?;

        match &result {
            Ok(found) => self.emit_audit(
                "find_elements",
                &app_key,
                Some(&format!("{} matches", found.len())),
                true,
                None,
            ),
            Err(e) => self.emit_audit("find_elements", &app_key, None, false, Some(&e.to_string())),
        }

        let found = result?;
        Ok(serde_json::json!({
            "elements": found,
            "count": found.len(),
        }))
    }

    async fn handle_click(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let element_ref = params["element_ref"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'element_ref' is required for click".to_string()))?
            .to_string();
        let app_key = Self::app_key(&params);

        // Check sensitive element name before clicking
        self.check_sensitive_element_ref(&element_ref, "click")?;

        let backend = self.backend.clone();
        let ref_clone = element_ref.clone();
        let result = tokio::task::spawn_blocking(move || backend.click_element(&ref_clone))
            .await
            .map_err(|e| LlmError::ToolCall(format!("Task join error: {e}")))?;

        match &result {
            Ok(()) => self.emit_audit("click", &app_key, Some(&element_ref), true, None),
            Err(e) => self.emit_audit(
                "click",
                &app_key,
                Some(&element_ref),
                false,
                Some(&e.to_string()),
            ),
        }

        result?;
        Ok(serde_json::json!({ "status": "clicked" }))
    }

    async fn handle_type_text(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let element_ref = params["element_ref"]
            .as_str()
            .ok_or_else(|| {
                LlmError::ToolCall("'element_ref' is required for type_text".to_string())
            })?
            .to_string();
        let text = params["text"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'text' is required for type_text".to_string()))?
            .to_string();
        let app_key = Self::app_key(&params);

        self.check_sensitive_element_ref(&element_ref, "type into")?;

        let backend = self.backend.clone();
        let ref_clone = element_ref.clone();
        let result = tokio::task::spawn_blocking(move || backend.type_text(&ref_clone, &text))
            .await
            .map_err(|e| LlmError::ToolCall(format!("Task join error: {e}")))?;

        match &result {
            Ok(()) => self.emit_audit("type_text", &app_key, Some(&element_ref), true, None),
            Err(e) => self.emit_audit(
                "type_text",
                &app_key,
                Some(&element_ref),
                false,
                Some(&e.to_string()),
            ),
        }

        result?;
        Ok(serde_json::json!({ "status": "typed" }))
    }

    async fn handle_read_text(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let element_ref = params["element_ref"]
            .as_str()
            .ok_or_else(|| {
                LlmError::ToolCall("'element_ref' is required for read_text".to_string())
            })?
            .to_string();
        let app_key = Self::app_key(&params);

        self.check_sensitive_element_ref(&element_ref, "read")?;

        let backend = self.backend.clone();
        let ref_clone = element_ref.clone();
        let result = tokio::task::spawn_blocking(move || backend.read_text(&ref_clone))
            .await
            .map_err(|e| LlmError::ToolCall(format!("Task join error: {e}")))?;

        match &result {
            Ok(_) => self.emit_audit("read_text", &app_key, Some(&element_ref), true, None),
            Err(e) => self.emit_audit(
                "read_text",
                &app_key,
                Some(&element_ref),
                false,
                Some(&e.to_string()),
            ),
        }

        let text = result?;
        Ok(serde_json::json!({ "text": text }))
    }

    async fn handle_get_tree(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let window_title = params["window_title"].as_str().map(String::from);
        let process_name = params["process_name"].as_str().map(String::from);
        let max_depth = params["max_depth"].as_u64().unwrap_or(4) as u32;
        let compact = params["compact"].as_bool().unwrap_or(true); // default compact for AI
        let app_key = Self::app_key(&params);

        let backend = self.backend.clone();
        let result = tokio::task::spawn_blocking(move || {
            backend.get_tree(
                window_title.as_deref(),
                process_name.as_deref(),
                max_depth,
                compact,
            )
        })
        .await
        .map_err(|e| LlmError::ToolCall(format!("Task join error: {e}")))?;

        match &result {
            Ok(_) => self.emit_audit("get_tree", &app_key, None, true, None),
            Err(e) => self.emit_audit("get_tree", &app_key, None, false, Some(&e.to_string())),
        }

        let tree = result?;
        Ok(serde_json::to_value(&tree).unwrap_or_default())
    }

    async fn handle_get_subtree(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let element_ref = params["element_ref"]
            .as_str()
            .ok_or_else(|| {
                LlmError::ToolCall("'element_ref' is required for get_subtree".to_string())
            })?
            .to_string();
        let max_depth = params["max_depth"].as_u64().unwrap_or(4) as u32;
        let app_key = Self::app_key(&params);

        let backend = self.backend.clone();
        let ref_clone = element_ref.clone();
        let result =
            tokio::task::spawn_blocking(move || backend.get_subtree(&ref_clone, max_depth))
                .await
                .map_err(|e| LlmError::ToolCall(format!("Task join error: {e}")))?;

        match &result {
            Ok(_) => self.emit_audit("get_subtree", &app_key, Some(&element_ref), true, None),
            Err(e) => self.emit_audit(
                "get_subtree",
                &app_key,
                Some(&element_ref),
                false,
                Some(&e.to_string()),
            ),
        }

        let tree = result?;
        Ok(serde_json::to_value(&tree).unwrap_or_default())
    }

    async fn handle_screenshot(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let window_title = params["window_title"].as_str().map(String::from);
        let process_name = params["process_name"].as_str().map(String::from);
        let app_key = Self::app_key(&params);

        let backend = self.backend.clone();
        let result = tokio::task::spawn_blocking(move || {
            backend.screenshot_window(window_title.as_deref(), process_name.as_deref())
        })
        .await
        .map_err(|e| LlmError::ToolCall(format!("Task join error: {e}")))?;

        match &result {
            Ok(_) => self.emit_audit("screenshot", &app_key, None, true, None),
            Err(e) => self.emit_audit("screenshot", &app_key, None, false, Some(&e.to_string())),
        }

        let screenshot = result?;
        // Return with _image_data for the agent loop to extract and send as a
        // multimodal content block alongside the text result.
        Ok(serde_json::json!({
            "window_title": screenshot.window_title,
            "width": screenshot.width,
            "height": screenshot.height,
            "_image_data": [{
                "mime_type": screenshot.mime_type,
                "data": screenshot.image_base64,
            }],
        }))
    }

    async fn handle_close(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let window_title = params["window_title"].as_str().map(String::from);
        let process_name = params["process_name"].as_str().map(String::from);
        let app_key = Self::app_key(&params);

        let backend = self.backend.clone();
        let pn_clone = process_name.clone();
        let wt_clone = window_title.clone();
        let result = tokio::task::spawn_blocking(move || {
            backend.close_window(wt_clone.as_deref(), pn_clone.as_deref())
        })
        .await
        .map_err(|e| LlmError::ToolCall(format!("Task join error: {e}")))?;

        match &result {
            Ok(()) => {
                self.emit_audit("close", &app_key, None, true, None);
            }
            Err(e) => {
                self.emit_audit("close", &app_key, None, false, Some(&e.to_string()));
                // If window close fails, try force-kill by PID.
                if let Ok(mut processes) = self.managed_processes.lock() {
                    let target = process_name
                        .as_deref()
                        .or(window_title.as_deref())
                        .unwrap_or_default()
                        .to_lowercase();
                    if !target.is_empty() {
                        let pid_to_kill: Option<u32> = processes
                            .iter()
                            .find(|(_pid, mp)| mp.executable.to_lowercase().contains(&target))
                            .map(|(pid, _)| *pid);
                        if let Some(pid) = pid_to_kill {
                            #[cfg(windows)]
                            {
                                let _ = std::process::Command::new("taskkill")
                                    .args(["/F", "/PID", &pid.to_string()])
                                    .output();
                            }
                            processes.remove(&pid);
                            self.emit_audit(
                                "close",
                                &app_key,
                                None,
                                true,
                                Some("force-killed by PID"),
                            );
                            return Ok(serde_json::json!({ "status": "force_closed" }));
                        }
                    }
                }
            }
        }

        // Remove from managed processes by matching executable name.
        if result.is_ok() {
            if let Ok(mut processes) = self.managed_processes.lock() {
                let target = process_name
                    .or(window_title)
                    .unwrap_or_default()
                    .to_lowercase();
                if !target.is_empty() {
                    processes.retain(|_pid, mp| !mp.executable.to_lowercase().contains(&target));
                }
            }
        }

        result?;
        Ok(serde_json::json!({ "status": "closed" }))
    }

    async fn handle_press_keys(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let keys = params["keys"]
            .as_str()
            .ok_or_else(|| {
                LlmError::ToolCall(
                    "'keys' is required for press_keys (e.g. 'ctrl+a', 'enter', 'tab')".to_string(),
                )
            })?
            .to_string();
        let window_title = params["window_title"].as_str().map(String::from);
        let process_name = params["process_name"].as_str().map(String::from);
        let app_key = Self::app_key(&params);

        let backend = self.backend.clone();
        let result = tokio::task::spawn_blocking(move || {
            backend.press_keys(window_title.as_deref(), process_name.as_deref(), &keys)
        })
        .await
        .map_err(|e| LlmError::ToolCall(format!("Task join error: {e}")))?;

        match &result {
            Ok(()) => self.emit_audit(
                "press_keys",
                &app_key,
                Some(&params["keys"].as_str().unwrap_or("")),
                true,
                None,
            ),
            Err(e) => self.emit_audit("press_keys", &app_key, None, false, Some(&e.to_string())),
        }

        result?;
        Ok(serde_json::json!({ "status": "keys_sent" }))
    }

    async fn handle_scroll(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let window_title = params["window_title"].as_str().map(String::from);
        let process_name = params["process_name"].as_str().map(String::from);
        let element_ref = params["element_ref"].as_str().map(String::from);
        let amount = params["amount"].as_i64().unwrap_or(-3) as i32; // default: scroll down 3
        let app_key = Self::app_key(&params);

        let backend = self.backend.clone();
        let result = tokio::task::spawn_blocking(move || {
            backend.scroll(
                window_title.as_deref(),
                process_name.as_deref(),
                element_ref.as_deref(),
                amount,
            )
        })
        .await
        .map_err(|e| LlmError::ToolCall(format!("Task join error: {e}")))?;

        match &result {
            Ok(()) => self.emit_audit("scroll", &app_key, None, true, None),
            Err(e) => self.emit_audit("scroll", &app_key, None, false, Some(&e.to_string())),
        }

        result?;
        Ok(serde_json::json!({ "status": "scrolled", "amount": amount }))
    }

    async fn handle_focus_window(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let window_title = params["window_title"].as_str().map(String::from);
        let process_name = params["process_name"].as_str().map(String::from);
        let app_key = Self::app_key(&params);

        let backend = self.backend.clone();
        let result = tokio::task::spawn_blocking(move || {
            backend.focus_window(window_title.as_deref(), process_name.as_deref())
        })
        .await
        .map_err(|e| LlmError::ToolCall(format!("Task join error: {e}")))?;

        match &result {
            Ok(()) => self.emit_audit("focus_window", &app_key, None, true, None),
            Err(e) => self.emit_audit("focus_window", &app_key, None, false, Some(&e.to_string())),
        }

        result?;
        Ok(serde_json::json!({ "status": "focused" }))
    }
}

#[async_trait]
impl NativeTool for AppInteractTool {
    fn name(&self) -> &str {
        "app_interact"
    }

    fn description(&self) -> &str {
        "Interact with desktop applications via UI Automation. Supports launching apps, \
         finding and clicking UI elements, typing text, sending keyboard shortcuts, reading \
         element text, scrolling, inspecting the UI element tree, exploring subtrees, \
         finding multiple elements, focusing windows, and taking screenshots. All actions \
         are scoped to allowed applications. Password fields and sensitive elements are \
         always blocked for security."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": [
                        "launch", "list_windows", "find_element", "find_elements",
                        "click", "type_text", "read_text", "get_tree", "get_subtree",
                        "screenshot", "close", "press_keys", "scroll", "focus_window"
                    ],
                    "description": "The action to perform on a desktop application"
                },
                "executable": {
                    "type": "string",
                    "description": "Path or name of the executable to launch (launch action only)"
                },
                "args": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Command-line arguments for launch (launch action only)"
                },
                "window_title": {
                    "type": "string",
                    "description": "Window title or substring to target (used by most actions)"
                },
                "process_name": {
                    "type": "string",
                    "description": "Process name to filter by (e.g., 'notepad.exe')"
                },
                "element_name": {
                    "type": "string",
                    "description": "UI element name or substring to find (find_element/find_elements)"
                },
                "element_type": {
                    "type": "string",
                    "description": "UI element control type (e.g., 'Button', 'Edit', 'Text', 'MenuItem')"
                },
                "automation_id": {
                    "type": "string",
                    "description": "Automation ID of the element (most reliable identifier)"
                },
                "element_ref": {
                    "type": "string",
                    "description": "Opaque element reference returned by find_element (for click, type_text, read_text, get_subtree, scroll)"
                },
                "text": {
                    "type": "string",
                    "description": "Text to type into an element (type_text action only)"
                },
                "keys": {
                    "type": "string",
                    "description": "Key combo to press (press_keys action). Examples: 'ctrl+a', 'enter', 'tab', 'ctrl+shift+t', 'alt+f4', 'escape'. Supports: ctrl, shift, alt, enter, tab, escape, space, backspace, delete, home, end, pageup, pagedown, up/down/left/right, f1-f12, a-z, 0-9"
                },
                "amount": {
                    "type": "integer",
                    "description": "Scroll amount in 'clicks' for scroll action. Positive = up, negative = down. Default: -3 (scroll down)"
                },
                "compact": {
                    "type": "boolean",
                    "description": "If true (default), get_tree omits element_ref from output to reduce size. Set to false if you need element_ref for subsequent actions."
                },
                "max_depth": {
                    "type": "integer",
                    "description": "Maximum depth for get_tree/get_subtree (default: 4, max: 8)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum elements to return for find_elements (default: 20, max: 100)"
                },
                "timeout_ms": {
                    "type": "integer",
                    "description": "Timeout in milliseconds for element search (default: 5000, max: 30000)"
                }
            },
            "required": ["action"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::AppAutomation(None)
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let action = params["action"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'action' parameter is required".to_string()))?;

        // Check if platform is supported
        if !self.backend.is_supported() {
            return Err(LlmError::ToolCall(
                "Desktop app automation is only supported on Windows".to_string(),
            ));
        }

        // Validate action against the configured scope
        SecurityGuard::validate_action(action, &self.scope)?;

        // Rate limit check using scope
        let app_key = Self::app_key(&params);
        self.check_rate_limit(&app_key)?;

        // Dispatch to handler
        match action {
            "launch" => self.handle_launch(params).await,
            "list_windows" => self.handle_list_windows(params).await,
            "find_element" => self.handle_find_element(params).await,
            "find_elements" => self.handle_find_elements(params).await,
            "click" => self.handle_click(params).await,
            "type_text" => self.handle_type_text(params).await,
            "read_text" => self.handle_read_text(params).await,
            "get_tree" => self.handle_get_tree(params).await,
            "get_subtree" => self.handle_get_subtree(params).await,
            "screenshot" => self.handle_screenshot(params).await,
            "close" => self.handle_close(params).await,
            "press_keys" => self.handle_press_keys(params).await,
            "scroll" => self.handle_scroll(params).await,
            "focus_window" => self.handle_focus_window(params).await,
            _ => Err(LlmError::ToolCall(format!(
                "Unknown action '{}'. Valid actions: launch, list_windows, find_element, find_elements, \
                 click, type_text, read_text, get_tree, get_subtree, screenshot, close, press_keys, scroll, focus_window",
                action
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_interact_name() {
        let tool = AppInteractTool::new();
        assert_eq!(tool.name(), "app_interact");
    }

    #[test]
    fn test_app_interact_capability() {
        let tool = AppInteractTool::new();
        assert_eq!(tool.required_capability(), Capability::AppAutomation(None));
    }

    #[test]
    fn test_app_interact_schema_has_all_actions() {
        let tool = AppInteractTool::new();
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["action"].is_object());
        let action_enum = schema["properties"]["action"]["enum"].as_array().unwrap();
        assert_eq!(action_enum.len(), 14); // 11 original + press_keys + scroll + focus_window
    }

    #[test]
    fn test_app_interact_schema_has_new_params() {
        let tool = AppInteractTool::new();
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["max_results"].is_object());
    }

    #[test]
    fn test_app_interact_schema_required_fields() {
        let tool = AppInteractTool::new();
        let schema = tool.parameters_schema();
        let required = schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "action");
    }

    #[tokio::test]
    async fn test_unknown_action() {
        let tool = AppInteractTool::new();
        let result = tool
            .execute(serde_json::json!({ "action": "destroy" }))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown action"));
    }

    #[tokio::test]
    async fn test_missing_action() {
        let tool = AppInteractTool::new();
        let result = tool.execute(serde_json::json!({})).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("'action' parameter is required"));
    }

    #[tokio::test]
    async fn test_launch_missing_executable() {
        let tool = AppInteractTool::new();
        let result = tool
            .execute(serde_json::json!({ "action": "launch" }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_launch_blocked_lolbin() {
        let tool = AppInteractTool::new();
        let result = tool
            .execute(serde_json::json!({
                "action": "launch",
                "executable": "cmd.exe"
            }))
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_app_key_extraction() {
        assert_eq!(
            AppInteractTool::app_key(&serde_json::json!({"process_name": "notepad.exe"})),
            "notepad.exe"
        );
        assert_eq!(
            AppInteractTool::app_key(&serde_json::json!({"window_title": "Untitled"})),
            "Untitled"
        );
        assert_eq!(
            AppInteractTool::app_key(&serde_json::json!({"executable": "calc.exe"})),
            "calc.exe"
        );
        assert_eq!(AppInteractTool::app_key(&serde_json::json!({})), "unknown");
    }

    // Scope enforcement
    #[tokio::test]
    async fn test_scope_action_restriction() {
        let scope = AppAutomationScope {
            allowed_apps: None,
            allowed_actions: Some(vec!["launch".to_string()]),
            rate_limit: None,
            max_concurrent: None,
        };
        let tool = AppInteractTool::with_config(Some(scope), None);
        let result = tool
            .execute(serde_json::json!({ "action": "click", "element_ref": "test" }))
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not in the allowed"));
    }

    // Audit events are emitted
    #[tokio::test]
    async fn test_audit_event_emitted() {
        let bus = Arc::new(EventBus::new(16));
        let mut rx = bus.subscribe();
        let tool = AppInteractTool::with_config(None, Some(bus));

        tool.emit_audit("test_action", "test_app", Some("test_el"), true, None);

        match rx.try_recv() {
            Ok(OmniEvent::AppAutomationAction {
                action,
                target_app,
                target_element,
                success,
                error,
            }) => {
                assert_eq!(action, "test_action");
                assert_eq!(target_app, "test_app");
                assert_eq!(target_element.as_deref(), Some("test_el"));
                assert!(success);
                assert!(error.is_none());
            }
            other => panic!("Expected AppAutomationAction, got: {:?}", other),
        }

        tool.emit_audit(
            "click",
            "notepad.exe",
            None,
            false,
            Some("Element not found"),
        );

        match rx.try_recv() {
            Ok(OmniEvent::AppAutomationAction {
                action,
                success,
                error,
                ..
            }) => {
                assert_eq!(action, "click");
                assert!(!success);
                assert_eq!(error.as_deref(), Some("Element not found"));
            }
            other => panic!("Expected failure event, got: {:?}", other),
        }
    }

    #[test]
    fn test_audit_no_panic_without_bus() {
        let tool = AppInteractTool::new();
        tool.emit_audit("test", "app", None, true, None);
    }

    #[test]
    fn test_sensitive_element_ref_check() {
        let tool = AppInteractTool::new();
        let result = tool.check_sensitive_element_ref(
            "win:abc|name:Password Field|type:Edit|aid:pwd|idx:0",
            "click",
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("sensitive element"));
    }

    #[test]
    fn test_non_sensitive_element_ref_passes() {
        let tool = AppInteractTool::new();
        let result = tool
            .check_sensitive_element_ref("win:abc|name:Submit|type:Button|aid:btn|idx:0", "click");
        assert!(result.is_ok());
    }

    #[test]
    fn test_with_config_stores_scope() {
        let scope = AppAutomationScope {
            allowed_apps: Some(vec!["notepad.exe".to_string()]),
            allowed_actions: None,
            rate_limit: Some(30),
            max_concurrent: Some(5),
        };
        let tool = AppInteractTool::with_config(Some(scope.clone()), None);
        assert!(tool.scope.is_some());
        assert_eq!(tool.scope.as_ref().unwrap().rate_limit, Some(30));
    }

    // New action tests
    #[tokio::test]
    async fn test_get_subtree_missing_ref() {
        let tool = AppInteractTool::new();
        let result = tool
            .execute(serde_json::json!({ "action": "get_subtree" }))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("element_ref"));
    }

    #[tokio::test]
    async fn test_screenshot_dispatches() {
        let tool = AppInteractTool::new();
        // On Windows this will try to find a window; we just verify it dispatches
        let result = tool
            .execute(serde_json::json!({
                "action": "screenshot",
                "window_title": "zzz_nonexistent_window_zzz"
            }))
            .await;
        // Should fail with window not found, not "unknown action"
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(!err.contains("Unknown action"));
    }
}
