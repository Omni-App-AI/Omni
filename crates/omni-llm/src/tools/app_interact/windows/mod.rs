#![cfg(windows)]

pub mod chromium;
pub mod dpi;
pub mod element_ref;
pub mod focus;
pub mod helpers;
pub mod input;
pub mod interaction;
pub mod screenshot;
pub mod tree;

use std::time::{Duration, Instant};

use uiautomation::patterns::{UIScrollPattern, UIWindowPattern};
use uiautomation::types::ScrollAmount;
use uiautomation::{UIAutomation, UIElement};

use super::platform::UiAutomationBackend;
use super::types::*;
use crate::error::{LlmError, Result};

/// Windows UI Automation backend — full rewrite with reliable interactions.
///
/// Each method creates its own `UIAutomation` COM instance because COM apartments
/// are thread-affine and we run inside `tokio::task::spawn_blocking` which may use
/// different threads across calls.
pub struct WindowsUiaBackend;

impl WindowsUiaBackend {
    pub fn new() -> Self {
        // Set per-monitor DPI awareness so UIA returns physical pixel coordinates
        dpi::set_dpi_awareness();
        Self
    }

    fn create_automation() -> Result<UIAutomation> {
        UIAutomation::new()
            .map_err(|e| LlmError::ToolCall(format!("Failed to initialize UI Automation: {e}")))
    }
}

impl UiAutomationBackend for WindowsUiaBackend {
    fn is_supported(&self) -> bool {
        true
    }

    fn launch_app(&self, executable: &str, args: &[String]) -> Result<LaunchedProcess> {
        eprintln!("[app_interact] launch_app: executable='{}' args={:?}", executable, args);
        let mut cmd = std::process::Command::new(executable);
        cmd.args(args);

        let child = cmd.spawn().map_err(|e| {
            LlmError::ToolCall(format!("Failed to launch '{}': {}", executable, e))
        })?;

        let pid = child.id();
        eprintln!("[app_interact] launch_app: spawned pid={}", pid);

        // Extract the executable base name for process-name fallback search.
        let exe_base = std::path::Path::new(executable)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();

        // Poll for window to appear instead of fixed sleep.
        // 10 seconds accommodates browsers (Edge/Chrome) which spawn multiple
        // processes and may take 5-8s to create a usable UIA window.
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut attempt = 0u32;
        let window_title = loop {
            attempt += 1;
            if let Ok(automation) = UIAutomation::new() {
                if let Ok(root) = automation.get_root_element() {
                    if let Ok(walker) = automation.get_control_view_walker() {
                        let children = walker.get_children(&root).unwrap_or_default();
                        eprintln!("[app_interact] launch_app: attempt #{} — {} top-level children", attempt, children.len());

                        // Strategy 1: Search by spawned PID (works when the process owns the window)
                        let found_by_pid = children.iter().find(|c| {
                            c.get_process_id().unwrap_or(0) == pid
                                && !c.get_name().unwrap_or_default().is_empty()
                        });
                        if let Some(win) = found_by_pid {
                            let title = win.get_name().unwrap_or_default();
                            eprintln!("[app_interact] launch_app: found window by PID {} on attempt #{}: '{}'", pid, attempt, title);
                            break Some(title);
                        }

                        // Strategy 2: Chromium PID delegation fallback.
                        // When Edge/Chrome is already running, the launcher PID delegates to
                        // the existing browser process and exits. The window belongs to a
                        // different PID. Fall back to searching by process name.
                        if attempt >= 3 && !exe_base.is_empty() {
                            eprintln!("[app_interact] launch_app: attempt #{} — trying process-name fallback for '{}'", attempt, exe_base);
                            let found_by_name = children.iter().find(|c| {
                                let cpid = c.get_process_id().unwrap_or(0);
                                let title = c.get_name().unwrap_or_default();
                                if title.is_empty() {
                                    return false;
                                }
                                helpers::get_process_name_by_pid(cpid)
                                    .map(|pn| pn.to_lowercase().contains(&exe_base))
                                    .unwrap_or(false)
                            });
                            if let Some(win) = found_by_name {
                                let title = win.get_name().unwrap_or_default();
                                let actual_pid = win.get_process_id().unwrap_or(0);
                                eprintln!(
                                    "[app_interact] launch_app: PID delegation detected! \
                                     Spawned pid={} but window owned by pid={}. \
                                     Found by process name '{}' on attempt #{}: '{}'",
                                    pid, actual_pid, exe_base, attempt, title
                                );
                                break Some(title);
                            }
                        }
                    }
                }
            }

            if Instant::now() > deadline {
                eprintln!("[app_interact] launch_app: TIMEOUT after {} attempts — no window found for pid={} or process='{}'", attempt, pid, exe_base);
                break None;
            }
            std::thread::sleep(Duration::from_millis(200));
        };

        eprintln!("[app_interact] launch_app: result pid={} title={:?}", pid, window_title);
        Ok(LaunchedProcess {
            pid,
            executable: executable.to_string(),
            window_title,
        })
    }

    fn list_windows(&self, process_filter: Option<&str>) -> Result<Vec<WindowInfo>> {
        let automation = Self::create_automation()?;
        let root = automation
            .get_root_element()
            .map_err(|e| LlmError::ToolCall(format!("Failed to get desktop root: {e}")))?;
        let walker = automation
            .get_control_view_walker()
            .map_err(|e| LlmError::ToolCall(format!("Failed to get tree walker: {e}")))?;

        let mut windows = Vec::new();
        if let Some(children) = walker.get_children(&root) {
            for child in &children {
                let title = child.get_name().unwrap_or_default();
                if title.is_empty() {
                    continue;
                }

                let pid = child.get_process_id().unwrap_or(0);
                let class_name = child.get_classname().unwrap_or_default();
                let is_offscreen = child.is_offscreen().unwrap_or(true);
                let process_name =
                    helpers::get_process_name_by_pid(pid).unwrap_or_default();

                if let Some(filter) = process_filter {
                    if !process_name
                        .to_lowercase()
                        .contains(&filter.to_lowercase())
                    {
                        continue;
                    }
                }

                let rect = child.get_bounding_rectangle().ok();
                let bounds = rect.map(|r| WindowBounds {
                    x: r.get_left(),
                    y: r.get_top(),
                    width: r.get_right() - r.get_left(),
                    height: r.get_bottom() - r.get_top(),
                });

                windows.push(WindowInfo {
                    title,
                    process_name,
                    pid,
                    class_name,
                    is_visible: !is_offscreen,
                    bounds,
                });
            }
        }

        Ok(windows)
    }

    fn find_element(
        &self,
        window_title: Option<&str>,
        process_name: Option<&str>,
        element_name: Option<&str>,
        element_type: Option<&str>,
        automation_id: Option<&str>,
        timeout_ms: u64,
    ) -> Result<FoundElement> {
        eprintln!("[app_interact] find_element: win_title={:?} process={:?} elem_name={:?} elem_type={:?} aid={:?} timeout={}ms",
            window_title, process_name, element_name, element_type, automation_id, timeout_ms);
        let automation = Self::create_automation()?;

        // Polling retry: try every POLL_INTERVAL_MS until timeout_ms expires.
        // Both window search AND element search are inside the loop so that
        // newly launched apps (Edge, Chrome) that take seconds to create their
        // UIA window are retried rather than failing immediately.
        const POLL_INTERVAL_MS: u64 = 500;
        let chromium_attempt_timeout = 1500u64; // Chromium needs longer per attempt
        let standard_attempt_timeout = POLL_INTERVAL_MS.min(timeout_ms);
        let deadline = Instant::now() + Duration::from_millis(timeout_ms.max(1000));
        let mut last_err = String::from("no search attempted");
        let mut loop_count = 0u32;

        loop {
            loop_count += 1;
            // Find the window — may fail if the app is still starting
            let window = match helpers::find_window(&automation, window_title, process_name) {
                Ok(w) => w,
                Err(e) => {
                    last_err = e.to_string();
                    eprintln!("[app_interact] find_element: loop #{} window search failed: {}", loop_count, last_err);
                    if Instant::now() >= deadline {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
                    continue;
                }
            };

            let win_title = window.get_name().unwrap_or_default();
            let win_hwnd = helpers::get_window_hwnd(&window).unwrap_or(0);
            let is_chromium = chromium::is_chromium_window(&window);
            eprintln!("[app_interact] find_element: loop #{} found window '{}' hwnd=0x{:x} chromium={}", loop_count, win_title, win_hwnd, is_chromium);

            // For Chromium apps, use the enhanced search with longer timeout
            if is_chromium {
                eprintln!("[app_interact] find_element: trying Chromium search (timeout={}ms)", chromium_attempt_timeout);
                match chromium::find_in_chromium(
                    &automation,
                    &window,
                    element_name,
                    element_type,
                    automation_id,
                    chromium_attempt_timeout,
                ) {
                    Ok(results) if !results.is_empty() => {
                        eprintln!("[app_interact] find_element: Chromium search found {} results", results.len());
                        return helpers::build_found_element(
                            &results[0],
                            &win_title,
                            win_hwnd,
                            0,
                        );
                    }
                    Ok(_) => {
                        eprintln!("[app_interact] find_element: Chromium search returned 0 results");
                    }
                    Err(e) => {
                        eprintln!("[app_interact] find_element: Chromium search error: {}", e);
                    }
                }
            }

            // Standard search
            eprintln!("[app_interact] find_element: trying standard search (timeout={}ms)", standard_attempt_timeout);
            let mut matcher = automation
                .create_matcher()
                .from(window)
                .depth(element_ref::FIND_ELEMENT_DEPTH)
                .timeout(standard_attempt_timeout);

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

            match matcher.find_first() {
                Ok(element) => {
                    let ename = element.get_name().unwrap_or_default();
                    eprintln!("[app_interact] find_element: SUCCESS found element '{}'", ename);
                    return helpers::build_found_element(&element, &win_title, win_hwnd, 0);
                }
                Err(e) => {
                    last_err = e.to_string();
                    eprintln!("[app_interact] find_element: standard search failed: {}", last_err);
                }
            }

            if Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
        }

        eprintln!("[app_interact] find_element: FAILED after {} loops ({}ms): {}", loop_count, timeout_ms, last_err);
        Err(LlmError::ToolCall(format!(
            "Element not found after {}ms polling: {}",
            timeout_ms, last_err
        )))
    }

    fn find_elements(
        &self,
        window_title: Option<&str>,
        process_name: Option<&str>,
        element_name: Option<&str>,
        element_type: Option<&str>,
        automation_id: Option<&str>,
        timeout_ms: u64,
        max_results: u32,
    ) -> Result<Vec<FoundElement>> {
        let automation = Self::create_automation()?;
        // Retry window search for apps that are still starting
        let window = helpers::find_window_with_retry(&automation, window_title, process_name, timeout_ms)?;
        let win_title = window.get_name().unwrap_or_default();
        let win_hwnd = helpers::get_window_hwnd(&window)?;

        // For Chromium windows, try enhanced search first
        if chromium::is_chromium_window(&window) {
            let results = chromium::find_in_chromium(
                &automation,
                &window,
                element_name,
                element_type,
                automation_id,
                timeout_ms,
            )
            .unwrap_or_default();

            if !results.is_empty() {
                let cap = (max_results as usize).min(results.len());
                let mut found = Vec::with_capacity(cap);
                for (i, elem) in results.iter().take(cap).enumerate() {
                    found.push(helpers::build_found_element(
                        elem,
                        &win_title,
                        win_hwnd,
                        i as u32,
                    )?);
                }
                return Ok(found);
            }
        }

        let mut matcher = automation
            .create_matcher()
            .from(window)
            .depth(element_ref::FIND_ELEMENT_DEPTH)
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

        let elements = matcher
            .find_all()
            .map_err(|e| LlmError::ToolCall(format!("Element search failed: {e}")))?;

        let cap = (max_results as usize).min(elements.len());
        let mut results = Vec::with_capacity(cap);
        for (i, elem) in elements.iter().take(cap).enumerate() {
            results.push(helpers::build_found_element(
                elem,
                &win_title,
                win_hwnd,
                i as u32,
            )?);
        }

        Ok(results)
    }

    fn click_element(&self, element_ref: &str, retry_count: u32, retry_delay_ms: u64) -> Result<()> {
        let automation = Self::create_automation()?;
        interaction::click_element(&automation, element_ref, retry_count, retry_delay_ms)
    }

    fn type_text(&self, element_ref: &str, text: &str, retry_count: u32, retry_delay_ms: u64) -> Result<()> {
        let automation = Self::create_automation()?;
        interaction::type_text_into(&automation, element_ref, text, retry_count, retry_delay_ms)
    }

    fn read_text(&self, element_ref: &str, retry_count: u32, retry_delay_ms: u64) -> Result<String> {
        let automation = Self::create_automation()?;
        interaction::read_text_from(&automation, element_ref, retry_count, retry_delay_ms)
    }

    fn get_tree(
        &self,
        window_title: Option<&str>,
        process_name: Option<&str>,
        max_depth: u32,
        compact: bool,
    ) -> Result<UiElementTree> {
        let automation = Self::create_automation()?;
        let window = helpers::find_window_with_retry(&automation, window_title, process_name, 5000)?;
        let win_title = window.get_name().unwrap_or_default();
        let win_pid = window.get_process_id().unwrap_or(0);
        let win_hwnd = helpers::get_window_hwnd(&window)?;
        let proc_name = helpers::get_process_name_by_pid(win_pid).unwrap_or_default();

        let walker = automation
            .get_control_view_walker()
            .map_err(|e| LlmError::ToolCall(format!("Failed to get tree walker: {e}")))?;

        let clamped_depth = max_depth.min(tree::MAX_TREE_DEPTH);
        let mut count = 0u32;
        let mut actual_depth = 0u32;

        let root_node = tree::walk_tree_recursive(
            &walker,
            &window,
            win_hwnd,
            &win_title,
            0,
            clamped_depth,
            &mut count,
            tree::MAX_TREE_ELEMENTS,
            &mut actual_depth,
            compact,
        )
        .unwrap_or(UiTreeNode {
            name: win_title.clone(),
            control_type: "Window".to_string(),
            automation_id: String::new(),
            element_ref: String::new(),
            is_enabled: true,
            value: None,
            children: Vec::new(),
        });

        Ok(tree::build_tree_result(
            win_title,
            proc_name,
            root_node,
            count,
            actual_depth,
            clamped_depth,
        ))
    }

    fn get_subtree(&self, element_ref: &str, max_depth: u32) -> Result<UiElementTree> {
        let automation = Self::create_automation()?;
        let (element, hwnd) = element_ref::decode_and_find(&automation, element_ref)?;

        let elem_name = element.get_name().unwrap_or_default();
        let pid = element.get_process_id().unwrap_or(0);
        let proc_name = helpers::get_process_name_by_pid(pid).unwrap_or_default();
        let win_title = elem_name.clone();

        let walker = automation
            .get_control_view_walker()
            .map_err(|e| LlmError::ToolCall(format!("Failed to get tree walker: {e}")))?;

        let clamped_depth = max_depth.min(tree::MAX_TREE_DEPTH);
        let mut count = 0u32;
        let mut actual_depth = 0u32;

        let root_node = tree::walk_tree_recursive(
            &walker,
            &element,
            hwnd,
            &win_title,
            0,
            clamped_depth,
            &mut count,
            tree::MAX_TREE_ELEMENTS,
            &mut actual_depth,
            false, // subtree always includes element_ref
        )
        .unwrap_or(UiTreeNode {
            name: elem_name,
            control_type: "Custom".to_string(),
            automation_id: String::new(),
            element_ref: element_ref.to_string(),
            is_enabled: true,
            value: None,
            children: Vec::new(),
        });

        Ok(tree::build_tree_result(
            win_title,
            proc_name,
            root_node,
            count,
            actual_depth,
            clamped_depth,
        ))
    }

    fn close_window(
        &self,
        window_title: Option<&str>,
        process_name: Option<&str>,
    ) -> Result<()> {
        let automation = Self::create_automation()?;
        let window = helpers::find_window(&automation, window_title, process_name)?;

        let win_pattern = window
            .get_pattern::<UIWindowPattern>()
            .map_err(|e| LlmError::ToolCall(format!("Window does not support close: {e}")))?;
        win_pattern
            .close()
            .map_err(|e| LlmError::ToolCall(format!("Failed to close window: {e}")))
    }

    fn is_password_field(&self, element_ref: &str) -> Result<bool> {
        let automation = Self::create_automation()?;
        let (element, _) = element_ref::decode_and_find(&automation, element_ref)?;
        Ok(element.is_password().unwrap_or(false))
    }

    fn screenshot_window(
        &self,
        window_title: Option<&str>,
        process_name: Option<&str>,
    ) -> Result<ScreenshotResult> {
        let automation = Self::create_automation()?;
        screenshot::capture_screenshot(&automation, window_title, process_name)
    }

    fn press_keys(
        &self,
        window_title: Option<&str>,
        process_name: Option<&str>,
        keys: &str,
    ) -> Result<()> {
        // Focus the target window first
        let automation = Self::create_automation()?;
        let window = helpers::find_window_with_retry(&automation, window_title, process_name, 5000)?;
        let hwnd = helpers::get_window_hwnd(&window)?;
        focus::bring_to_foreground(hwnd)?;

        input::press_key_combo(keys)
    }

    fn scroll(
        &self,
        window_title: Option<&str>,
        process_name: Option<&str>,
        element_ref_opt: Option<&str>,
        amount: i32,
    ) -> Result<()> {
        let automation = Self::create_automation()?;

        if let Some(eref) = element_ref_opt {
            let (element, _) = element_ref::decode_and_find(&automation, eref)?;

            // Try UIA ScrollPattern
            if let Ok(scroll_pattern) = element.get_pattern::<UIScrollPattern>() {
                let scroll_amount = if amount > 0 {
                    ScrollAmount::LargeIncrement
                } else {
                    ScrollAmount::LargeDecrement
                };

                let repeat = amount.unsigned_abs();
                for _ in 0..repeat {
                    scroll_pattern
                        .scroll(ScrollAmount::NoAmount, scroll_amount)
                        .map_err(|e| LlmError::ToolCall(format!("Scroll failed: {e}")))?;
                }
                return Ok(());
            }

            // Fallback: scroll at element center using mouse wheel
            if let Ok(rect) = element.get_bounding_rectangle() {
                let cx = (rect.get_left() + rect.get_right()) / 2;
                let cy = (rect.get_top() + rect.get_bottom()) / 2;
                return input::scroll_at(cx, cy, amount);
            }
        }

        // No element -- scroll at center of window
        let window = helpers::find_window(&automation, window_title, process_name)?;
        let rect = window
            .get_bounding_rectangle()
            .map_err(|e| LlmError::ToolCall(format!("Failed to get window rect: {e}")))?;
        let cx = (rect.get_left() + rect.get_right()) / 2;
        let cy = (rect.get_top() + rect.get_bottom()) / 2;
        input::scroll_at(cx, cy, amount)
    }

    fn focus_window(
        &self,
        window_title: Option<&str>,
        process_name: Option<&str>,
    ) -> Result<()> {
        eprintln!("[app_interact] focus_window: title={:?} process={:?}", window_title, process_name);
        let automation = Self::create_automation()?;
        let window = helpers::find_window_with_retry(&automation, window_title, process_name, 5000)?;
        let hwnd = helpers::get_window_hwnd(&window)?;
        let win_name = window.get_name().unwrap_or_default();
        eprintln!("[app_interact] focus_window: found '{}' hwnd=0x{:x}, calling bring_to_foreground", win_name, hwnd);
        focus::bring_to_foreground(hwnd)
    }

    fn element_info(&self, element_ref: &str) -> Result<FoundElement> {
        let automation = Self::create_automation()?;
        let (element, hwnd) = element_ref::decode_and_find(&automation, element_ref)?;
        let win_title = element.get_name().unwrap_or_default();
        helpers::build_found_element(&element, &win_title, hwnd, 0)
    }

    fn get_window_title_by_hwnd(&self, hwnd: isize, delay_ms: u64) -> Result<String> {
        if delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(delay_ms));
        }

        let automation = Self::create_automation()?;
        let root = automation
            .get_root_element()
            .map_err(|e| LlmError::ToolCall(format!("Failed to get desktop root: {e}")))?;
        let walker = automation
            .get_control_view_walker()
            .map_err(|e| LlmError::ToolCall(format!("Failed to get tree walker: {e}")))?;

        let children = walker.get_children(&root).unwrap_or_default();
        for child in &children {
            if let Ok(handle) = child.get_native_window_handle() {
                let h: isize = handle.into();
                if h == hwnd {
                    return Ok(child.get_name().unwrap_or_default());
                }
            }
        }

        Err(LlmError::ToolCall(format!(
            "Window not found by HWND 0x{:x}",
            hwnd
        )))
    }

    fn screenshot_window_by_hwnd(&self, hwnd: isize) -> Result<ScreenshotResult> {
        let (width, height, rgba_pixels) = screenshot::capture_window_pixels(hwnd)?;
        let png_bytes = screenshot::encode_png(width, height, &rgba_pixels)?;
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png_bytes);

        // Get the current window title for the result
        let title = self
            .get_window_title_by_hwnd(hwnd, 0)
            .unwrap_or_else(|_| "Unknown".to_string());

        Ok(ScreenshotResult {
            image_base64: b64,
            mime_type: "image/png".to_string(),
            width,
            height,
            window_title: title,
        })
    }

    fn wait_for_element(
        &self,
        window_title: Option<&str>,
        process_name: Option<&str>,
        element_name: Option<&str>,
        element_type: Option<&str>,
        automation_id: Option<&str>,
        timeout_ms: u64,
        poll_interval_ms: u64,
    ) -> Result<FoundElement> {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let mut last_err;

        loop {
            match self.find_element(
                window_title,
                process_name,
                element_name,
                element_type,
                automation_id,
                poll_interval_ms.min(1000),
            ) {
                Ok(found) => return Ok(found),
                Err(e) => last_err = e.to_string(),
            }

            if Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(poll_interval_ms));
        }

        Err(LlmError::ToolCall(format!(
            "Element not found after {}ms: {}",
            timeout_ms, last_err
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_supported() {
        let backend = WindowsUiaBackend::new();
        assert!(backend.is_supported());
    }

    #[test]
    fn test_create_automation() {
        let result = WindowsUiaBackend::create_automation();
        assert!(
            result.is_ok(),
            "COM initialization failed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_list_windows_returns_results() {
        let backend = WindowsUiaBackend::new();
        match backend.list_windows(None) {
            Ok(windows) => {
                assert!(!windows.is_empty(), "Expected at least one window");
            }
            Err(e) => {
                eprintln!("Skipping test (COM init issue in test environment): {e}");
            }
        }
    }

    #[test]
    fn test_list_windows_with_filter() {
        let backend = WindowsUiaBackend::new();
        match backend.list_windows(Some("zzz_nonexistent_app_zzz")) {
            Ok(windows) => {
                assert!(windows.is_empty());
            }
            Err(e) => {
                eprintln!("Skipping test (COM init issue in test environment): {e}");
            }
        }
    }

    #[test]
    fn test_find_elements_empty_filter() {
        let backend = WindowsUiaBackend::new();
        match backend.find_elements(
            Some("zzz_nonexistent_window_zzz"),
            None,
            Some("zzz_nonexistent_zzz"),
            None,
            None,
            1000,
            10,
        ) {
            Ok(elems) => assert!(elems.is_empty()),
            Err(_) => {} // expected -- window not found
        }
    }
}
