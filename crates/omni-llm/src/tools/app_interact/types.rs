use serde::{Deserialize, Serialize};

fn is_false(b: &bool) -> bool {
    !b
}

/// A process launched by the app_interact tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchedProcess {
    pub pid: u32,
    pub executable: String,
    pub window_title: Option<String>,
}

/// Information about a top-level window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub title: String,
    pub process_name: String,
    pub pid: u32,
    pub class_name: String,
    pub is_visible: bool,
    pub bounds: Option<WindowBounds>,
}

/// Rectangle bounds of a window or element.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// A UI element found by find_element.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoundElement {
    /// Opaque reference for subsequent actions (encoded search criteria).
    pub element_ref: String,
    pub name: String,
    pub control_type: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub automation_id: String,
    pub is_enabled: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub is_password: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounding_rect: Option<WindowBounds>,
    /// Supported interaction patterns (e.g., "Invoke", "Value", "Toggle").
    pub patterns: Vec<String>,
    /// Toggle state for checkboxes/toggle buttons (e.g., "On", "Off", "Indeterminate").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toggle_state: Option<String>,
}

/// A UI element tree snapshot for a window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiElementTree {
    pub window_title: String,
    pub process_name: String,
    pub root: UiTreeNode,
    pub total_elements: u32,
    pub depth_reached: u32,
    /// True if the tree was truncated due to element cap or depth limit.
    #[serde(skip_serializing_if = "is_false")]
    pub truncated: bool,
    /// Why the tree was truncated (e.g., "element cap reached (500)", "depth limit reached (5)").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation_reason: Option<String>,
}

/// A single node in the UI element tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiTreeNode {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    pub control_type: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub automation_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub element_ref: String,
    pub is_enabled: bool,
    /// Text value, or "[REDACTED]" if password field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<UiTreeNode>,
}

/// State of a rate limiter for one application.
pub struct RateLimiterState {
    pub window_start: std::time::Instant,
    pub action_count: u32,
}

/// A managed process launched by the tool.
/// Tracked by PID for correct key management and force-kill capability.
pub struct ManagedProcess {
    pub pid: u32,
    pub executable: String,
    pub launched_at: std::time::Instant,
}

/// Screenshot capture result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotResult {
    /// Base64-encoded PNG image data.
    pub image_base64: String,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
    pub window_title: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_launched_process_serialization() {
        let p = LaunchedProcess {
            pid: 1234,
            executable: "notepad.exe".to_string(),
            window_title: Some("Untitled - Notepad".to_string()),
        };
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["pid"], 1234);
        assert_eq!(json["executable"], "notepad.exe");
        assert_eq!(json["window_title"], "Untitled - Notepad");
    }

    #[test]
    fn test_window_info_serialization() {
        let w = WindowInfo {
            title: "Calculator".to_string(),
            process_name: "calc.exe".to_string(),
            pid: 5678,
            class_name: "ApplicationFrameWindow".to_string(),
            is_visible: true,
            bounds: Some(WindowBounds {
                x: 100,
                y: 200,
                width: 400,
                height: 500,
            }),
        };
        let json = serde_json::to_value(&w).unwrap();
        assert_eq!(json["title"], "Calculator");
        assert_eq!(json["bounds"]["width"], 400);
    }

    #[test]
    fn test_found_element_serialization() {
        let e = FoundElement {
            element_ref: "win:abc|name:OK|type:Button|aid:btnOk|idx:0".to_string(),
            name: "OK".to_string(),
            control_type: "Button".to_string(),
            automation_id: "btnOk".to_string(),
            is_enabled: true,
            is_password: false,
            bounding_rect: None,
            patterns: vec!["Invoke".to_string()],
            toggle_state: None,
        };
        let json = serde_json::to_value(&e).unwrap();
        assert_eq!(json["name"], "OK");
        assert_eq!(json["patterns"][0], "Invoke");
    }

    #[test]
    fn test_found_element_compact_serialization() {
        // When is_password is false, bounding_rect is None, and automation_id is empty,
        // those fields should be omitted from JSON output.
        let e = FoundElement {
            element_ref: "win:abc|name:OK|type:Button|aid:|idx:0".to_string(),
            name: "OK".to_string(),
            control_type: "Button".to_string(),
            automation_id: String::new(),
            is_enabled: true,
            is_password: false,
            bounding_rect: None,
            patterns: vec!["Invoke".to_string()],
            toggle_state: None,
        };
        let json = serde_json::to_value(&e).unwrap();
        assert!(json.get("automation_id").is_none());
        assert!(json.get("is_password").is_none());
        assert!(json.get("bounding_rect").is_none());
    }

    #[test]
    fn test_ui_tree_serialization() {
        let tree = UiElementTree {
            window_title: "Notepad".to_string(),
            process_name: "notepad.exe".to_string(),
            root: UiTreeNode {
                name: "Notepad".to_string(),
                control_type: "Window".to_string(),
                automation_id: String::new(),
                element_ref: "win:abc|name:Notepad|type:Window|aid:|idx:0".to_string(),
                is_enabled: true,
                value: None,
                children: vec![UiTreeNode {
                    name: "Text Editor".to_string(),
                    control_type: "Edit".to_string(),
                    automation_id: "editor".to_string(),
                    element_ref: "win:abc|name:Text Editor|type:Edit|aid:editor|idx:0".to_string(),
                    is_enabled: true,
                    value: Some("Hello world".to_string()),
                    children: vec![],
                }],
            },
            total_elements: 2,
            depth_reached: 1,
            truncated: false,
            truncation_reason: None,
        };
        let json = serde_json::to_value(&tree).unwrap();
        assert_eq!(json["total_elements"], 2);
        assert_eq!(json["root"]["children"][0]["value"], "Hello world");
        // Compact: truncated=false should be omitted, empty automation_id omitted
        assert!(json.get("truncated").is_none());
        assert!(json.get("truncation_reason").is_none());
        assert!(json["root"].get("automation_id").is_none());
        // Empty children on leaf should be omitted
        assert!(json["root"]["children"][0].get("children").is_none());
    }

    #[test]
    fn test_ui_tree_truncation_indicator() {
        let tree = UiElementTree {
            window_title: "VS Code".to_string(),
            process_name: "code.exe".to_string(),
            root: UiTreeNode {
                name: "VS Code".to_string(),
                control_type: "Window".to_string(),
                automation_id: String::new(),
                element_ref: String::new(),
                is_enabled: true,
                value: None,
                children: vec![],
            },
            total_elements: 500,
            depth_reached: 5,
            truncated: true,
            truncation_reason: Some("element cap reached (500)".to_string()),
        };
        let json = serde_json::to_value(&tree).unwrap();
        assert_eq!(json["truncated"], true);
        assert_eq!(json["truncation_reason"], "element cap reached (500)");
    }

    #[test]
    fn test_ui_tree_password_redaction() {
        let node = UiTreeNode {
            name: "Password".to_string(),
            control_type: "Edit".to_string(),
            automation_id: "pwdField".to_string(),
            element_ref: "win:abc|name:Password|type:Edit|aid:pwdField|idx:0".to_string(),
            is_enabled: true,
            value: Some("[REDACTED]".to_string()),
            children: vec![],
        };
        let json = serde_json::to_value(&node).unwrap();
        assert_eq!(json["value"], "[REDACTED]");
    }

    #[test]
    fn test_screenshot_result_serialization() {
        let s = ScreenshotResult {
            image_base64: "iVBOR...".to_string(),
            mime_type: "image/png".to_string(),
            width: 800,
            height: 600,
            window_title: "Notepad".to_string(),
        };
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["mime_type"], "image/png");
        assert_eq!(json["width"], 800);
    }
}
