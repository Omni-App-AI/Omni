#![cfg(windows)]

use uiautomation::{UIAutomation, UIElement};

use super::element_ref::FIND_ELEMENT_DEPTH;
use super::helpers;
use crate::error::{LlmError, Result};

/// Chromium-based window class names.
pub const CHROMIUM_CLASSES: &[&str] = &[
    "Chrome_WidgetWin_1",   // Chrome, Edge, Electron
    "Chrome_WidgetWin_0",   // Some Electron variants
    "CEF_BrowserWindow",    // Chromium Embedded Framework
];

/// The render widget host class name where web content lives.
const RENDER_WIDGET_HOST_CLASS: &str = "Chrome_RenderWidgetHostHWND";

/// Detect if a window belongs to a Chromium-based application.
pub fn is_chromium_window(element: &UIElement) -> bool {
    let class = element.get_classname().unwrap_or_default();
    CHROMIUM_CLASSES
        .iter()
        .any(|&c| c.eq_ignore_ascii_case(&class))
}

/// Find elements in a Chromium window by searching the render widget host subtree.
///
/// Chromium apps have a multi-process architecture where web content is rendered
/// in a separate process. The UIA tree for web content lives under a
/// `Chrome_RenderWidgetHostHWND` child of the main window.
pub fn find_in_chromium(
    automation: &UIAutomation,
    window: &UIElement,
    element_name: Option<&str>,
    element_type: Option<&str>,
    automation_id: Option<&str>,
    timeout_ms: u64,
) -> Result<Vec<UIElement>> {
    // Strategy 1: Search the entire window tree (works for Chrome 138+ with native UIA)
    let mut matcher = automation
        .create_matcher()
        .from(window.clone())
        .depth(FIND_ELEMENT_DEPTH)
        .timeout(timeout_ms);

    if let Some(name) = element_name {
        matcher = matcher.contains_name(name);
    }
    if let Some(et) = element_type {
        if let Some(ct) = helpers::parse_control_type(et) {
            matcher = matcher.control_type(ct);
        }
    }
    if let Some(aid) = automation_id {
        let aid_owned = aid.to_string();
        matcher = matcher.filter_fn(Box::new(move |e: &UIElement| {
            let elem_aid = e.get_automation_id().unwrap_or_default();
            Ok(elem_aid == aid_owned)
        }));
    }

    let results = matcher.find_all().unwrap_or_default();
    if !results.is_empty() {
        return Ok(results);
    }

    // Strategy 2: Find the render widget host and search within it
    if let Ok(render_host) = find_render_widget_host(automation, window) {
        let mut host_matcher = automation
            .create_matcher()
            .from(render_host)
            .depth(FIND_ELEMENT_DEPTH)
            .timeout(timeout_ms);

        if let Some(name) = element_name {
            host_matcher = host_matcher.contains_name(name);
        }
        if let Some(et) = element_type {
            if let Some(ct) = helpers::parse_control_type(et) {
                host_matcher = host_matcher.control_type(ct);
            }
        }
        if let Some(aid) = automation_id {
            let aid_owned = aid.to_string();
            host_matcher = host_matcher.filter_fn(Box::new(move |e: &UIElement| {
                let elem_aid = e.get_automation_id().unwrap_or_default();
                Ok(elem_aid == aid_owned)
            }));
        }

        let host_results = host_matcher.find_all().unwrap_or_default();
        if !host_results.is_empty() {
            return Ok(host_results);
        }
    }

    Ok(Vec::new())
}

/// Find the Chrome_RenderWidgetHostHWND child which hosts web content.
/// Uses a 3-second timeout to accommodate post-launch scenarios where the
/// render process may still be initializing.
fn find_render_widget_host(
    automation: &UIAutomation,
    window: &UIElement,
) -> Result<UIElement> {
    let matcher = automation
        .create_matcher()
        .from(window.clone())
        .depth(5) // RenderWidgetHost is usually within a few levels
        .timeout(3000)
        .classname(RENDER_WIDGET_HOST_CLASS);

    matcher
        .find_first()
        .map_err(|e| LlmError::ToolCall(format!("Render widget host not found: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chromium_class_detection() {
        // We can't create real UIElements in tests, but we can verify the constants
        assert!(CHROMIUM_CLASSES.contains(&"Chrome_WidgetWin_1"));
        assert!(CHROMIUM_CLASSES.contains(&"CEF_BrowserWindow"));
        assert!(!CHROMIUM_CLASSES.contains(&"Notepad"));
    }

    #[test]
    fn test_render_widget_host_class() {
        assert_eq!(RENDER_WIDGET_HOST_CLASS, "Chrome_RenderWidgetHostHWND");
    }

    #[test]
    fn test_chromium_classes_all_variants() {
        // Verify all 3 Chromium class patterns are present
        assert_eq!(CHROMIUM_CLASSES.len(), 3);
        assert!(CHROMIUM_CLASSES.contains(&"Chrome_WidgetWin_0"));  // Electron variant
        assert!(CHROMIUM_CLASSES.contains(&"Chrome_WidgetWin_1"));  // Chrome/Edge
        assert!(CHROMIUM_CLASSES.contains(&"CEF_BrowserWindow"));   // CEF
    }

    #[test]
    fn test_non_chromium_classes_not_matched() {
        let non_chromium = ["Notepad", "ConsoleWindowClass", "Shell_TrayWnd", "MozillaWindowClass"];
        for class in &non_chromium {
            assert!(!CHROMIUM_CLASSES.contains(class), "Should not match: {}", class);
        }
    }
}
