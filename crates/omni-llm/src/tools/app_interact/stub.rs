use super::platform::UiAutomationBackend;
use super::types::*;
use crate::error::{LlmError, Result};

/// Stub backend for platforms that don't support desktop UI automation.
pub struct StubBackend;

impl StubBackend {
    fn unsupported<T>() -> Result<T> {
        Err(LlmError::ToolCall(
            "Desktop app automation is only supported on Windows".to_string(),
        ))
    }
}

impl UiAutomationBackend for StubBackend {
    fn is_supported(&self) -> bool {
        false
    }

    fn launch_app(&self, _executable: &str, _args: &[String]) -> Result<LaunchedProcess> {
        Self::unsupported()
    }

    fn list_windows(&self, _process_filter: Option<&str>) -> Result<Vec<WindowInfo>> {
        Self::unsupported()
    }

    fn find_element(
        &self,
        _window_title: Option<&str>,
        _process_name: Option<&str>,
        _element_name: Option<&str>,
        _element_type: Option<&str>,
        _automation_id: Option<&str>,
        _timeout_ms: u64,
    ) -> Result<FoundElement> {
        Self::unsupported()
    }

    fn find_elements(
        &self,
        _window_title: Option<&str>,
        _process_name: Option<&str>,
        _element_name: Option<&str>,
        _element_type: Option<&str>,
        _automation_id: Option<&str>,
        _timeout_ms: u64,
        _max_results: u32,
    ) -> Result<Vec<FoundElement>> {
        Self::unsupported()
    }

    fn click_element(&self, _element_ref: &str) -> Result<()> {
        Self::unsupported()
    }

    fn type_text(&self, _element_ref: &str, _text: &str) -> Result<()> {
        Self::unsupported()
    }

    fn read_text(&self, _element_ref: &str) -> Result<String> {
        Self::unsupported()
    }

    fn get_tree(
        &self,
        _window_title: Option<&str>,
        _process_name: Option<&str>,
        _max_depth: u32,
        _compact: bool,
    ) -> Result<UiElementTree> {
        Self::unsupported()
    }

    fn get_subtree(&self, _element_ref: &str, _max_depth: u32) -> Result<UiElementTree> {
        Self::unsupported()
    }

    fn close_window(&self, _window_title: Option<&str>, _process_name: Option<&str>) -> Result<()> {
        Self::unsupported()
    }

    fn is_password_field(&self, _element_ref: &str) -> Result<bool> {
        Self::unsupported()
    }

    fn screenshot_window(
        &self,
        _window_title: Option<&str>,
        _process_name: Option<&str>,
    ) -> Result<ScreenshotResult> {
        Self::unsupported()
    }

    fn press_keys(
        &self,
        _window_title: Option<&str>,
        _process_name: Option<&str>,
        _keys: &str,
    ) -> Result<()> {
        Self::unsupported()
    }

    fn scroll(
        &self,
        _window_title: Option<&str>,
        _process_name: Option<&str>,
        _element_ref: Option<&str>,
        _amount: i32,
    ) -> Result<()> {
        Self::unsupported()
    }

    fn focus_window(&self, _window_title: Option<&str>, _process_name: Option<&str>) -> Result<()> {
        Self::unsupported()
    }
}
