use super::types::*;
use crate::error::Result;

/// Platform-agnostic UI automation backend.
///
/// All methods are synchronous because they execute inside `tokio::task::spawn_blocking`.
/// Each platform provides its own implementation; unsupported platforms use `StubBackend`.
pub trait UiAutomationBackend: Send + Sync {
    /// Whether this platform is supported.
    fn is_supported(&self) -> bool;

    /// Launch an executable and return its PID.
    fn launch_app(&self, executable: &str, args: &[String]) -> Result<LaunchedProcess>;

    /// List all visible top-level windows, optionally filtered by process name.
    fn list_windows(&self, process_filter: Option<&str>) -> Result<Vec<WindowInfo>>;

    /// Find a UI element within a window by name, type, or automation ID.
    fn find_element(
        &self,
        window_title: Option<&str>,
        process_name: Option<&str>,
        element_name: Option<&str>,
        element_type: Option<&str>,
        automation_id: Option<&str>,
        timeout_ms: u64,
    ) -> Result<FoundElement>;

    /// Find multiple UI elements matching the criteria (up to `max_results`).
    fn find_elements(
        &self,
        window_title: Option<&str>,
        process_name: Option<&str>,
        element_name: Option<&str>,
        element_type: Option<&str>,
        automation_id: Option<&str>,
        timeout_ms: u64,
        max_results: u32,
    ) -> Result<Vec<FoundElement>>;

    /// Click an element via its semantic interaction pattern (InvokePattern, not coordinates).
    /// `retry_count` and `retry_delay_ms` control wait_for_ready behavior (defaults: 3 / 300).
    fn click_element(&self, element_ref: &str, retry_count: u32, retry_delay_ms: u64) -> Result<()>;

    /// Type text into an element. Returns error if the element is a password field.
    /// `retry_count` and `retry_delay_ms` control wait_for_ready behavior (defaults: 3 / 300).
    fn type_text(&self, element_ref: &str, text: &str, retry_count: u32, retry_delay_ms: u64) -> Result<()>;

    /// Read text from an element. Returns error if the element is a password field.
    /// `retry_count` and `retry_delay_ms` control wait_for_ready behavior (defaults: 3 / 300).
    fn read_text(&self, element_ref: &str, retry_count: u32, retry_delay_ms: u64) -> Result<String>;

    /// Get the UI element tree of a window, with depth limit and sensitive field redaction.
    /// When `compact` is true, element_ref strings are omitted to reduce output size for LLMs.
    fn get_tree(
        &self,
        window_title: Option<&str>,
        process_name: Option<&str>,
        max_depth: u32,
        compact: bool,
    ) -> Result<UiElementTree>;

    /// Get a subtree starting from a specific element reference.
    fn get_subtree(&self, element_ref: &str, max_depth: u32) -> Result<UiElementTree>;

    /// Close a window gracefully.
    fn close_window(&self, window_title: Option<&str>, process_name: Option<&str>) -> Result<()>;

    /// Check if an element is a password/sensitive field.
    fn is_password_field(&self, element_ref: &str) -> Result<bool>;

    /// Capture a screenshot of a window as PNG bytes.
    fn screenshot_window(
        &self,
        window_title: Option<&str>,
        process_name: Option<&str>,
    ) -> Result<ScreenshotResult>;

    /// Send keyboard shortcuts / key combos (e.g., "ctrl+a", "enter", "alt+f4").
    fn press_keys(
        &self,
        window_title: Option<&str>,
        process_name: Option<&str>,
        keys: &str,
    ) -> Result<()>;

    /// Scroll within a window or element. `amount` is in "clicks" (positive=up, negative=down).
    fn scroll(
        &self,
        window_title: Option<&str>,
        process_name: Option<&str>,
        element_ref: Option<&str>,
        amount: i32,
    ) -> Result<()>;

    /// Bring a window to the foreground and restore if minimized.
    fn focus_window(&self, window_title: Option<&str>, process_name: Option<&str>) -> Result<()>;

    /// Get information about a found element by its reference (for validation/retry).
    fn element_info(&self, element_ref: &str) -> Result<FoundElement>;

    /// Wait for an element to appear with configurable poll interval.
    fn wait_for_element(
        &self,
        window_title: Option<&str>,
        process_name: Option<&str>,
        element_name: Option<&str>,
        element_type: Option<&str>,
        automation_id: Option<&str>,
        timeout_ms: u64,
        poll_interval_ms: u64,
    ) -> Result<FoundElement>;

    /// Get the current title of the window that owns the given HWND.
    /// Used after state-changing actions to detect navigation (title changes).
    /// Waits `delay_ms` before checking to allow the title to update.
    fn get_window_title_by_hwnd(&self, hwnd: isize, delay_ms: u64) -> Result<String>;

    /// Capture a screenshot of the window identified by HWND.
    /// Used for automatic post-navigation screenshots where the title may have changed.
    fn screenshot_window_by_hwnd(&self, hwnd: isize) -> Result<ScreenshotResult>;
}
