#![cfg(windows)]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use uiautomation::patterns::{
    UIInvokePattern, UISelectionItemPattern, UITextPattern, UITogglePattern, UIValuePattern,
    UIWindowPattern,
};
use uiautomation::types::ControlType;
use uiautomation::{UIAutomation, UIElement, UITreeWalker};

use super::platform::UiAutomationBackend;
use super::types::*;
use crate::error::{LlmError, Result};

/// Maximum elements returned in a tree walk (prevents LLM context overflow).
const MAX_TREE_ELEMENTS: u32 = 500;

/// Maximum search depth for find_element / find_elements / decode_and_find.
const FIND_ELEMENT_DEPTH: u32 = 15;

/// Maximum depth for get_tree / get_subtree (hard cap, users can request less).
const MAX_TREE_DEPTH: u32 = 8;

/// Windows UI Automation backend.
///
/// Each method creates its own `UIAutomation` COM instance because COM apartments
/// are thread-affine and we run inside `tokio::task::spawn_blocking` which may use
/// different threads across calls.
pub struct WindowsUiaBackend;

impl WindowsUiaBackend {
    pub fn new() -> Self {
        Self
    }

    /// Create a new UIAutomation COM instance for this thread.
    fn create_automation() -> Result<UIAutomation> {
        UIAutomation::new()
            .map_err(|e| LlmError::ToolCall(format!("Failed to initialize UI Automation: {e}")))
    }

    /// Hash a window title for element reference encoding.
    /// Uses 32 bits to reduce collision probability.
    fn hash_title(title: &str) -> String {
        let mut hasher = DefaultHasher::new();
        title.hash(&mut hasher);
        format!("{:08x}", hasher.finish() & 0xFFFFFFFF)
    }

    /// Encode an element reference from its search criteria.
    /// Format: `win:{title_hash}|name:{name}|type:{type}|aid:{aid}|idx:{idx}`
    fn encode_element_ref(
        window_title: &str,
        name: &str,
        control_type: &str,
        automation_id: &str,
        index: u32,
    ) -> String {
        format!(
            "win:{}|name:{}|type:{}|aid:{}|idx:{}",
            Self::hash_title(window_title),
            name.replace('|', "\\|"),
            control_type.replace('|', "\\|"),
            automation_id.replace('|', "\\|"),
            index
        )
    }

    /// Decode an element reference and re-find the element.
    fn decode_and_find(
        automation: &UIAutomation,
        element_ref: &str,
    ) -> Result<(UIElement, String)> {
        // Parse the ref fields. We need to split by '|' but NOT by '\|'.
        let mut win_hash = String::new();
        let mut name = String::new();
        let mut ctrl_type = String::new();
        let mut aid = String::new();
        let mut idx: u32 = 0;

        let mut parts = Vec::new();
        let mut current_part = String::new();
        let mut chars = element_ref.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(&nc) = chars.peek() {
                    if nc == '|' {
                        current_part.push('|');
                        chars.next();
                        continue;
                    }
                }
                current_part.push('\\');
            } else if c == '|' {
                parts.push(current_part);
                current_part = String::new();
            } else {
                current_part.push(c);
            }
        }
        parts.push(current_part);

        for part in parts {
            if let Some(v) = part.strip_prefix("win:") {
                win_hash = v.to_string();
            } else if let Some(v) = part.strip_prefix("name:") {
                name = v.to_string();
            } else if let Some(v) = part.strip_prefix("type:") {
                ctrl_type = v.to_string();
            } else if let Some(v) = part.strip_prefix("aid:") {
                aid = v.to_string();
            } else if let Some(v) = part.strip_prefix("idx:") {
                idx = v.parse().unwrap_or(0);
            }
        }

        // Find the parent window by walking top-level windows
        let root = automation
            .get_root_element()
            .map_err(|e| LlmError::ToolCall(format!("Failed to get desktop root: {e}")))?;

        let walker = automation
            .get_control_view_walker()
            .map_err(|e| LlmError::ToolCall(format!("Failed to get tree walker: {e}")))?;

        let target_window = Self::find_window_by_hash(&walker, &root, &win_hash)?;
        let window_title = target_window.get_name().unwrap_or_default();

        // Search for the element within the window
        let name_owned = name;
        let ctrl_type_owned = ctrl_type;
        let aid_owned = aid;

        let mut matcher = automation
            .create_matcher()
            .from(target_window.clone())
            .depth(FIND_ELEMENT_DEPTH)
            .timeout(3000);

        if !name_owned.is_empty() {
            matcher = matcher.name(&name_owned);
        }
        if !ctrl_type_owned.is_empty() {
            if let Some(ct) = Self::parse_control_type(&ctrl_type_owned) {
                matcher = matcher.control_type(ct);
            }
        }

        // If we have an automation ID, use filter_fn (UIMatcher doesn't have .automation_id())
        if !aid_owned.is_empty() {
            matcher = matcher.filter_fn(Box::new(move |e: &UIElement| {
                let elem_aid = e.get_automation_id().unwrap_or_default();
                Ok(elem_aid == aid_owned)
            }));
        }

        let matches = matcher
            .find_all()
            .map_err(|e| LlmError::ToolCall(format!("Element not found: {e}")))?;

        if matches.is_empty() {
            return Err(LlmError::ToolCall(
                "Element not found -- it may have been removed or the window has changed"
                    .to_string(),
            ));
        }

        let target_idx = (idx as usize).min(matches.len() - 1);
        Ok((matches[target_idx].clone(), window_title))
    }

    /// Find a top-level window whose title hashes to the given hash.
    fn find_window_by_hash(
        walker: &UITreeWalker,
        root: &UIElement,
        target_hash: &str,
    ) -> Result<UIElement> {
        if let Some(children) = walker.get_children(root) {
            for child in children {
                let title = child.get_name().unwrap_or_default();
                if Self::hash_title(&title) == target_hash {
                    return Ok(child);
                }
            }
        }
        Err(LlmError::ToolCall(
            "Target window not found -- it may have been closed".to_string(),
        ))
    }

    /// Known system window class names to skip when searching for user windows.
    const SYSTEM_WINDOW_CLASSES: &'static [&'static str] = &[
        "Progman",                    // Desktop program manager
        "Shell_TrayWnd",              // Taskbar
        "Shell_SecondaryTrayWnd",     // Secondary taskbar
        "WorkerW",                    // Desktop worker
        "DV2ControlHost",             // Start menu
        "Windows.UI.Core.CoreWindow", // Some system UWP overlays
        "ForegroundStaging",          // Transition window
        "MultitaskingViewFrame",      // Task view
    ];

    /// Check if a window element is a usable application window (not system/hidden/zero-size).
    fn is_usable_window(element: &UIElement) -> bool {
        let title = element.get_name().unwrap_or_default();
        if title.is_empty() {
            return false;
        }

        // Skip known system window classes
        let class = element.get_classname().unwrap_or_default();
        if Self::SYSTEM_WINDOW_CLASSES
            .iter()
            .any(|&c| c.eq_ignore_ascii_case(&class))
        {
            return false;
        }

        // Skip windows with zero or negative size
        if let Ok(rect) = element.get_bounding_rectangle() {
            let w = rect.get_right() - rect.get_left();
            let h = rect.get_bottom() - rect.get_top();
            if w <= 0 || h <= 0 {
                return false;
            }
        }

        true
    }

    /// Strip Unicode control/format characters for fuzzy window title matching.
    /// Edge inserts zero-width spaces (U+200B) etc. in its title.
    fn normalize_for_comparison(s: &str) -> String {
        s.chars()
            .filter(|c| !c.is_control() && !matches!(c, '\u{200B}'..='\u{200F}' | '\u{FEFF}' | '\u{00AD}' | '\u{2060}'..='\u{2064}' | '\u{2066}'..='\u{2069}' | '\u{FE00}'..='\u{FE0F}'))
            .collect::<String>()
            .to_lowercase()
    }

    /// Find a window by title substring or process name.
    /// Prefers visible, reasonably-sized windows over offscreen/system ones.
    /// Handles Unicode edge cases (e.g. zero-width spaces in Edge's title).
    fn find_window(
        automation: &UIAutomation,
        window_title: Option<&str>,
        process_name: Option<&str>,
    ) -> Result<UIElement> {
        let root = automation
            .get_root_element()
            .map_err(|e| LlmError::ToolCall(format!("Failed to get desktop root: {e}")))?;

        // First try: UIA's contains_name matcher (fast path)
        let windows = if let Some(title) = window_title {
            let matcher = automation
                .create_matcher()
                .from(root.clone())
                .depth(1)
                .timeout(3000)
                .contains_name(title);

            let found = matcher.find_all().unwrap_or_default();

            // If UIA matcher found nothing, do a broad fallback with normalized comparison
            if found.is_empty() {
                let all_matcher = automation
                    .create_matcher()
                    .from(root)
                    .depth(1)
                    .timeout(3000);

                let all_windows = all_matcher.find_all().map_err(|e| {
                    LlmError::ToolCall(format!("Failed to search for windows: {e}"))
                })?;

                let search_norm = Self::normalize_for_comparison(title);
                all_windows
                    .into_iter()
                    .filter(|w| {
                        let wname = w.get_name().unwrap_or_default();
                        let w_norm = Self::normalize_for_comparison(&wname);
                        w_norm.contains(&search_norm)
                    })
                    .collect()
            } else {
                found
            }
        } else {
            let matcher = automation
                .create_matcher()
                .from(root)
                .depth(1)
                .timeout(3000);

            matcher
                .find_all()
                .map_err(|e| LlmError::ToolCall(format!("Failed to search for windows: {e}")))?
        };

        // Filter to usable windows only
        let usable: Vec<&UIElement> = windows
            .iter()
            .filter(|w| Self::is_usable_window(w))
            .collect();

        // Filter by process name if specified
        if let Some(proc_name) = process_name {
            let proc_lower = proc_name.to_lowercase();
            let mut matched_windows = Vec::new();
            let candidates = if usable.is_empty() {
                &windows
            } else {
                &usable.iter().map(|w| (*w).clone()).collect::<Vec<_>>()
            };
            for win in candidates {
                let pid = win.get_process_id().unwrap_or(0);
                if let Some(pname) = Self::get_process_name_by_pid(pid) {
                    if pname.to_lowercase().contains(&proc_lower) {
                        matched_windows.push(win.clone());
                    }
                }
            }

            if matched_windows.is_empty() {
                return Err(LlmError::ToolCall(format!(
                    "No window found for process '{}'",
                    proc_name
                )));
            }

            // Prefer visible, non-offscreen window
            for win in &matched_windows {
                if !win.is_offscreen().unwrap_or(true) {
                    return Ok(win.clone());
                }
            }

            // Fallback: return the first one
            return Ok(matched_windows.into_iter().next().unwrap());
        }

        // No process filter -- prefer visible usable windows
        let candidates = if usable.is_empty() {
            windows.iter().collect::<Vec<_>>()
        } else {
            usable
        };

        let mut best_visible = None;
        let mut first_any = None;
        for win in candidates {
            if !win.is_offscreen().unwrap_or(true) {
                if best_visible.is_none() {
                    best_visible = Some(win.clone());
                }
            }
            if first_any.is_none() {
                first_any = Some(win.clone());
            }
        }

        best_visible.or(first_any).ok_or_else(|| {
            LlmError::ToolCall(format!(
                "No window found matching '{}'",
                window_title.unwrap_or("(no filter)")
            ))
        })
    }

    /// Get process name from PID using Windows API (OpenProcess + QueryFullProcessImageName).
    fn get_process_name_by_pid(pid: u32) -> Option<String> {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;

        const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

        extern "system" {
            fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
            fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
            fn QueryFullProcessImageNameW(
                process: *mut std::ffi::c_void,
                flags: u32,
                name: *mut u16,
                size: *mut u32,
            ) -> i32;
        }

        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                return None;
            }

            let mut buf = [0u16; 260]; // MAX_PATH
            let mut size = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
            CloseHandle(handle);

            if ok == 0 {
                return None;
            }

            let path = OsString::from_wide(&buf[..size as usize]);
            let path_str = path.to_string_lossy().to_string();
            std::path::Path::new(&path_str)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
        }
    }

    /// Parse a control type string to ControlType enum.
    fn parse_control_type(s: &str) -> Option<ControlType> {
        match s {
            "AppBar" => Some(ControlType::AppBar),
            "Button" => Some(ControlType::Button),
            "Calendar" => Some(ControlType::Calendar),
            "CheckBox" => Some(ControlType::CheckBox),
            "ComboBox" => Some(ControlType::ComboBox),
            "Custom" => Some(ControlType::Custom),
            "DataGrid" => Some(ControlType::DataGrid),
            "DataItem" => Some(ControlType::DataItem),
            "Document" => Some(ControlType::Document),
            "Edit" => Some(ControlType::Edit),
            "Group" => Some(ControlType::Group),
            "Header" => Some(ControlType::Header),
            "HeaderItem" => Some(ControlType::HeaderItem),
            "Hyperlink" => Some(ControlType::Hyperlink),
            "Image" => Some(ControlType::Image),
            "List" => Some(ControlType::List),
            "ListItem" => Some(ControlType::ListItem),
            "Menu" => Some(ControlType::Menu),
            "MenuBar" => Some(ControlType::MenuBar),
            "MenuItem" => Some(ControlType::MenuItem),
            "Pane" => Some(ControlType::Pane),
            "ProgressBar" => Some(ControlType::ProgressBar),
            "RadioButton" => Some(ControlType::RadioButton),
            "ScrollBar" => Some(ControlType::ScrollBar),
            "SemanticZoom" => Some(ControlType::SemanticZoom),
            "Separator" => Some(ControlType::Separator),
            "Slider" => Some(ControlType::Slider),
            "Spinner" => Some(ControlType::Spinner),
            "SplitButton" => Some(ControlType::SplitButton),
            "StatusBar" => Some(ControlType::StatusBar),
            "Tab" => Some(ControlType::Tab),
            "TabItem" => Some(ControlType::TabItem),
            "Table" => Some(ControlType::Table),
            "Text" => Some(ControlType::Text),
            "Thumb" => Some(ControlType::Thumb),
            "TitleBar" => Some(ControlType::TitleBar),
            "ToolBar" => Some(ControlType::ToolBar),
            "ToolTip" => Some(ControlType::ToolTip),
            "Tree" => Some(ControlType::Tree),
            "TreeItem" => Some(ControlType::TreeItem),
            "Window" => Some(ControlType::Window),
            _ => None,
        }
    }

    /// Convert ControlType to its string name.
    fn control_type_name(ct: ControlType) -> &'static str {
        match ct {
            ControlType::AppBar => "AppBar",
            ControlType::Button => "Button",
            ControlType::Calendar => "Calendar",
            ControlType::CheckBox => "CheckBox",
            ControlType::ComboBox => "ComboBox",
            ControlType::Custom => "Custom",
            ControlType::DataGrid => "DataGrid",
            ControlType::DataItem => "DataItem",
            ControlType::Document => "Document",
            ControlType::Edit => "Edit",
            ControlType::Group => "Group",
            ControlType::Header => "Header",
            ControlType::HeaderItem => "HeaderItem",
            ControlType::Hyperlink => "Hyperlink",
            ControlType::Image => "Image",
            ControlType::List => "List",
            ControlType::ListItem => "ListItem",
            ControlType::Menu => "Menu",
            ControlType::MenuBar => "MenuBar",
            ControlType::MenuItem => "MenuItem",
            ControlType::Pane => "Pane",
            ControlType::ProgressBar => "ProgressBar",
            ControlType::RadioButton => "RadioButton",
            ControlType::ScrollBar => "ScrollBar",
            ControlType::SemanticZoom => "SemanticZoom",
            ControlType::Separator => "Separator",
            ControlType::Slider => "Slider",
            ControlType::Spinner => "Spinner",
            ControlType::SplitButton => "SplitButton",
            ControlType::StatusBar => "StatusBar",
            ControlType::Tab => "Tab",
            ControlType::TabItem => "TabItem",
            ControlType::Table => "Table",
            ControlType::Text => "Text",
            ControlType::Thumb => "Thumb",
            ControlType::TitleBar => "TitleBar",
            ControlType::ToolBar => "ToolBar",
            ControlType::ToolTip => "ToolTip",
            ControlType::Tree => "Tree",
            ControlType::TreeItem => "TreeItem",
            ControlType::Window => "Window",
        }
    }

    /// Build a FoundElement from a UIElement.
    fn build_found_element(
        element: &UIElement,
        window_title: &str,
        index: u32,
    ) -> Result<FoundElement> {
        let name = element.get_name().unwrap_or_default();
        let ct = element.get_control_type().unwrap_or(ControlType::Custom);
        let ct_name = Self::control_type_name(ct).to_string();
        let aid = element.get_automation_id().unwrap_or_default();
        let is_enabled = element.is_enabled().unwrap_or(false);
        let is_pwd = element.is_password().unwrap_or(false);

        let rect = element.get_bounding_rectangle().ok();
        let bounds = rect.map(|r| WindowBounds {
            x: r.get_left(),
            y: r.get_top(),
            width: r.get_right() - r.get_left(),
            height: r.get_bottom() - r.get_top(),
        });

        // Detect supported patterns
        let mut patterns = Vec::new();
        if element.get_pattern::<UIInvokePattern>().is_ok() {
            patterns.push("Invoke".to_string());
        }
        if element.get_pattern::<UIValuePattern>().is_ok() {
            patterns.push("Value".to_string());
        }
        if element.get_pattern::<UITogglePattern>().is_ok() {
            patterns.push("Toggle".to_string());
        }
        if element.get_pattern::<UISelectionItemPattern>().is_ok() {
            patterns.push("SelectionItem".to_string());
        }
        if element.get_pattern::<UIWindowPattern>().is_ok() {
            patterns.push("Window".to_string());
        }
        if element.get_pattern::<UITextPattern>().is_ok() {
            patterns.push("Text".to_string());
        }
        if element
            .get_pattern::<uiautomation::patterns::UIExpandCollapsePattern>()
            .is_ok()
        {
            patterns.push("ExpandCollapse".to_string());
        }
        if element
            .get_pattern::<uiautomation::patterns::UIScrollPattern>()
            .is_ok()
        {
            patterns.push("Scroll".to_string());
        }

        // Read toggle state for checkboxes/toggle buttons
        let toggle_state = element
            .get_pattern::<UITogglePattern>()
            .ok()
            .and_then(|tp| tp.get_toggle_state().ok())
            .map(|state| {
                use uiautomation::types::ToggleState;
                match state {
                    ToggleState::Off => "Off".to_string(),
                    ToggleState::On => "On".to_string(),
                    ToggleState::Indeterminate => "Indeterminate".to_string(),
                }
            });

        let element_ref = Self::encode_element_ref(window_title, &name, &ct_name, &aid, index);

        Ok(FoundElement {
            element_ref,
            name,
            control_type: ct_name,
            automation_id: aid,
            is_enabled,
            is_password: is_pwd,
            bounding_rect: bounds,
            patterns,
            toggle_state,
        })
    }

    /// Walk the UI tree recursively with depth and element count limits.
    /// Tracks actual max depth reached via `max_depth_reached` output param.
    fn walk_tree_recursive(
        walker: &UITreeWalker,
        element: &UIElement,
        window_title: &str,
        current_depth: u32,
        max_depth: u32,
        count: &mut u32,
        max_elements: u32,
        max_depth_reached: &mut u32,
        compact: bool,
    ) -> Option<UiTreeNode> {
        if *count >= max_elements {
            return None;
        }

        if current_depth > *max_depth_reached {
            *max_depth_reached = current_depth;
        }

        let name = element.get_name().unwrap_or_default();
        let ct = element.get_control_type().unwrap_or(ControlType::Custom);
        let ct_name = Self::control_type_name(ct).to_string();
        let aid = element.get_automation_id().unwrap_or_default();
        let is_enabled = element.is_enabled().unwrap_or(false);
        let is_pwd = element.is_password().unwrap_or(false);

        // Read value, redacting password fields
        let value = if is_pwd {
            Some("[REDACTED]".to_string())
        } else {
            element
                .get_pattern::<UIValuePattern>()
                .ok()
                .and_then(|vp| vp.get_value().ok())
        };

        let element_ref = if compact {
            String::new() // omit element_ref in compact mode -- skip_serializing_if handles the rest
        } else {
            Self::encode_element_ref(window_title, &name, &ct_name, &aid, *count)
        };
        *count += 1;

        // Walk children if within depth limit
        let children = if current_depth < max_depth {
            let mut kids = Vec::new();
            if let Some(child_elements) = walker.get_children(element) {
                for child in &child_elements {
                    if *count >= max_elements {
                        break;
                    }
                    if let Some(node) = Self::walk_tree_recursive(
                        walker,
                        child,
                        window_title,
                        current_depth + 1,
                        max_depth,
                        count,
                        max_elements,
                        max_depth_reached,
                        compact,
                    ) {
                        kids.push(node);
                    }
                }
            }
            kids
        } else {
            Vec::new()
        };

        Some(UiTreeNode {
            name,
            control_type: ct_name,
            automation_id: aid,
            element_ref,
            is_enabled,
            value,
            children,
        })
    }

    /// Build a UiElementTree result with truncation tracking.
    fn build_tree_result(
        win_title: String,
        proc_name: String,
        root_node: UiTreeNode,
        count: u32,
        actual_depth: u32,
        requested_depth: u32,
    ) -> UiElementTree {
        let hit_element_cap = count >= MAX_TREE_ELEMENTS;
        let truncated = hit_element_cap;
        let truncation_reason = if hit_element_cap {
            Some(format!(
                "element cap reached ({}). Use get_subtree on specific elements to explore deeper, \
                 or use find_element/find_elements to search for specific controls.",
                MAX_TREE_ELEMENTS
            ))
        } else if actual_depth >= requested_depth && actual_depth > 0 {
            // Not truncated by element cap, but depth was limiting -- add a hint
            Some(format!(
                "depth limit reached ({}). Increase max_depth (up to {}) or use get_subtree to explore specific branches.",
                requested_depth, MAX_TREE_DEPTH
            ))
        } else {
            None
        };

        UiElementTree {
            window_title: win_title,
            process_name: proc_name,
            root: root_node,
            total_elements: count,
            depth_reached: actual_depth,
            truncated,
            truncation_reason,
        }
    }

    /// Perform a left-click at the given screen coordinates using SendInput.
    fn click_at_coordinates(x: i32, y: i32) -> Result<()> {
        #[repr(C)]
        struct MouseInput {
            dx: i32,
            dy: i32,
            mouse_data: u32,
            dw_flags: u32,
            time: u32,
            dw_extra_info: usize,
        }

        #[repr(C)]
        struct Input {
            input_type: u32,
            mi: MouseInput,
        }

        extern "system" {
            fn SendInput(count: u32, inputs: *const Input, size: i32) -> u32;
            fn SetCursorPos(x: i32, y: i32) -> i32;
            fn GetSystemMetrics(index: i32) -> i32;
        }

        const INPUT_MOUSE: u32 = 0;
        const MOUSEEVENTF_ABSOLUTE: u32 = 0x8000;
        const MOUSEEVENTF_MOVE: u32 = 0x0001;
        const MOUSEEVENTF_LEFTDOWN: u32 = 0x0002;
        const MOUSEEVENTF_LEFTUP: u32 = 0x0004;
        const SM_CXSCREEN: i32 = 0;
        const SM_CYSCREEN: i32 = 1;

        unsafe {
            // Move cursor to position first
            SetCursorPos(x, y);
            std::thread::sleep(std::time::Duration::from_millis(50));

            // Convert to normalized absolute coordinates (0-65535)
            let screen_w = GetSystemMetrics(SM_CXSCREEN).max(1);
            let screen_h = GetSystemMetrics(SM_CYSCREEN).max(1);
            let norm_x = ((x as i64) * 65535 / screen_w as i64) as i32;
            let norm_y = ((y as i64) * 65535 / screen_h as i64) as i32;

            let inputs = [
                Input {
                    input_type: INPUT_MOUSE,
                    mi: MouseInput {
                        dx: norm_x,
                        dy: norm_y,
                        mouse_data: 0,
                        dw_flags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE | MOUSEEVENTF_LEFTDOWN,
                        time: 0,
                        dw_extra_info: 0,
                    },
                },
                Input {
                    input_type: INPUT_MOUSE,
                    mi: MouseInput {
                        dx: norm_x,
                        dy: norm_y,
                        mouse_data: 0,
                        dw_flags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE | MOUSEEVENTF_LEFTUP,
                        time: 0,
                        dw_extra_info: 0,
                    },
                },
            ];

            let sent = SendInput(2, inputs.as_ptr(), std::mem::size_of::<Input>() as i32);

            if sent != 2 {
                return Err(LlmError::ToolCall(
                    "Failed to send mouse click input".to_string(),
                ));
            }
        }

        // Small delay for the click to register
        std::thread::sleep(std::time::Duration::from_millis(100));
        Ok(())
    }

    /// Send keyboard input using SendInput. Each key in `keys` is pressed in order.
    /// `keys` contains (virtual_key_code, scan_code, is_extended) tuples.
    fn send_input_keyboard(keys: &[(u16, bool)], press_down: bool) -> Result<()> {
        #[repr(C)]
        struct KeyboardInput {
            w_vk: u16,
            w_scan: u16,
            dw_flags: u32,
            time: u32,
            dw_extra_info: usize,
        }

        #[repr(C)]
        struct Input {
            input_type: u32,
            ki: KeyboardInput,
            _padding: [u8; 8], // Ensure proper alignment for union
        }

        extern "system" {
            fn SendInput(count: u32, inputs: *const Input, size: i32) -> u32;
            fn MapVirtualKeyW(code: u32, map_type: u32) -> u32;
        }

        const INPUT_KEYBOARD: u32 = 1;
        const KEYEVENTF_KEYUP: u32 = 0x0002;
        const KEYEVENTF_EXTENDEDKEY: u32 = 0x0001;
        const MAPVK_VK_TO_VSC: u32 = 0;

        let mut inputs: Vec<Input> = Vec::with_capacity(keys.len());

        for &(vk, is_extended) in keys {
            let scan = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) as u16 };
            let mut flags = if press_down { 0 } else { KEYEVENTF_KEYUP };
            if is_extended {
                flags |= KEYEVENTF_EXTENDEDKEY;
            }

            inputs.push(Input {
                input_type: INPUT_KEYBOARD,
                ki: KeyboardInput {
                    w_vk: vk,
                    w_scan: scan,
                    dw_flags: flags,
                    time: 0,
                    dw_extra_info: 0,
                },
                _padding: [0u8; 8],
            });
        }

        unsafe {
            let sent = SendInput(
                inputs.len() as u32,
                inputs.as_ptr(),
                std::mem::size_of::<Input>() as i32,
            );

            if sent != inputs.len() as u32 {
                return Err(LlmError::ToolCall(
                    "Failed to send keyboard input".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Parse a key combo string like "ctrl+a", "shift+enter", "alt+f4" into virtual key codes.
    /// Returns Vec of (vk_code, is_extended_key).
    fn parse_key_combo(combo: &str) -> Result<Vec<(u16, bool)>> {
        let parts: Vec<&str> = combo.split('+').map(|s| s.trim()).collect();
        let mut keys = Vec::new();

        for part in &parts {
            let lower = part.to_lowercase();
            let (vk, extended) = match lower.as_str() {
                // Modifiers
                "ctrl" | "control" => (0x11_u16, false),
                "shift" => (0x10, false),
                "alt" => (0x12, false),
                "win" | "super" | "meta" => (0x5B, false),
                // Navigation
                "enter" | "return" => (0x0D, false),
                "tab" => (0x09, false),
                "escape" | "esc" => (0x1B, false),
                "space" | " " => (0x20, false),
                "backspace" | "back" => (0x08, false),
                "delete" | "del" => (0x2E, true),
                "insert" | "ins" => (0x2D, true),
                "home" => (0x24, true),
                "end" => (0x23, true),
                "pageup" | "pgup" => (0x21, true),
                "pagedown" | "pgdn" => (0x22, true),
                // Arrow keys
                "up" | "uparrow" => (0x26, true),
                "down" | "downarrow" => (0x28, true),
                "left" | "leftarrow" => (0x25, true),
                "right" | "rightarrow" => (0x27, true),
                // Function keys
                "f1" => (0x70, false),
                "f2" => (0x71, false),
                "f3" => (0x72, false),
                "f4" => (0x73, false),
                "f5" => (0x74, false),
                "f6" => (0x75, false),
                "f7" => (0x76, false),
                "f8" => (0x77, false),
                "f9" => (0x78, false),
                "f10" => (0x79, false),
                "f11" => (0x7A, false),
                "f12" => (0x7B, false),
                // Single characters A-Z, 0-9
                s if s.len() == 1 => {
                    let c = s.chars().next().unwrap().to_ascii_uppercase();
                    if c.is_ascii_alphanumeric() {
                        (c as u16, false)
                    } else {
                        return Err(LlmError::ToolCall(format!(
                            "Unsupported key: '{}'. Use named keys (ctrl, shift, enter, etc.) or single alphanumeric characters.",
                            part
                        )));
                    }
                }
                _ => {
                    return Err(LlmError::ToolCall(format!(
                        "Unknown key: '{}'. Supported: ctrl, shift, alt, enter, tab, escape, space, \
                         backspace, delete, home, end, pageup, pagedown, up/down/left/right, f1-f12, a-z, 0-9",
                        part
                    )));
                }
            };
            keys.push((vk, extended));
        }

        Ok(keys)
    }

    /// Scroll at coordinates using SendInput mouse wheel.
    fn scroll_at_coordinates(x: i32, y: i32, amount: i32) -> Result<()> {
        #[repr(C)]
        struct MouseInput {
            dx: i32,
            dy: i32,
            mouse_data: u32,
            dw_flags: u32,
            time: u32,
            dw_extra_info: usize,
        }

        #[repr(C)]
        struct Input {
            input_type: u32,
            mi: MouseInput,
        }

        extern "system" {
            fn SendInput(count: u32, inputs: *const Input, size: i32) -> u32;
            fn SetCursorPos(x: i32, y: i32) -> i32;
            fn GetSystemMetrics(index: i32) -> i32;
        }

        const INPUT_MOUSE: u32 = 0;
        const MOUSEEVENTF_ABSOLUTE: u32 = 0x8000;
        const MOUSEEVENTF_MOVE: u32 = 0x0001;
        const MOUSEEVENTF_WHEEL: u32 = 0x0800;
        const SM_CXSCREEN: i32 = 0;
        const SM_CYSCREEN: i32 = 1;
        const WHEEL_DELTA: i32 = 120;

        unsafe {
            // Move cursor to target position
            SetCursorPos(x, y);
            std::thread::sleep(std::time::Duration::from_millis(50));

            let screen_w = GetSystemMetrics(SM_CXSCREEN).max(1);
            let screen_h = GetSystemMetrics(SM_CYSCREEN).max(1);
            let norm_x = ((x as i64) * 65535 / screen_w as i64) as i32;
            let norm_y = ((y as i64) * 65535 / screen_h as i64) as i32;

            // Scroll amount: positive = up, negative = down (in WHEEL_DELTA units)
            let wheel_data = (amount * WHEEL_DELTA) as u32;

            let input = Input {
                input_type: INPUT_MOUSE,
                mi: MouseInput {
                    dx: norm_x,
                    dy: norm_y,
                    mouse_data: wheel_data,
                    dw_flags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE | MOUSEEVENTF_WHEEL,
                    time: 0,
                    dw_extra_info: 0,
                },
            };

            let sent = SendInput(1, &input, std::mem::size_of::<Input>() as i32);
            if sent != 1 {
                return Err(LlmError::ToolCall(
                    "Failed to send scroll input".to_string(),
                ));
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
        Ok(())
    }

    /// Bring a window to the foreground and restore if minimized.
    fn set_foreground(hwnd: isize) -> Result<()> {
        type HWND = *mut std::ffi::c_void;
        extern "system" {
            fn SetForegroundWindow(hwnd: HWND) -> i32;
            fn ShowWindow(hwnd: HWND, cmd: i32) -> i32;
            fn IsIconic(hwnd: HWND) -> i32;
        }

        const SW_RESTORE: i32 = 9;

        unsafe {
            let hwnd_ptr = hwnd as HWND;

            // Restore if minimized
            if IsIconic(hwnd_ptr) != 0 {
                ShowWindow(hwnd_ptr, SW_RESTORE);
                std::thread::sleep(std::time::Duration::from_millis(200));
            }

            if SetForegroundWindow(hwnd_ptr) == 0 {
                return Err(LlmError::ToolCall(
                    "Failed to bring window to foreground".to_string(),
                ));
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
        Ok(())
    }

    /// Capture a window screenshot using Windows GDI.
    /// Returns (width, height, BGRA pixel data).
    fn capture_window_pixels(hwnd: isize) -> Result<(u32, u32, Vec<u8>)> {
        #[repr(C)]
        struct Rect {
            left: i32,
            top: i32,
            right: i32,
            bottom: i32,
        }

        #[repr(C)]
        #[allow(non_snake_case)]
        struct BitmapInfoHeader {
            biSize: u32,
            biWidth: i32,
            biHeight: i32,
            biPlanes: u16,
            biBitCount: u16,
            biCompression: u32,
            biSizeImage: u32,
            biXPelsPerMeter: i32,
            biYPelsPerMeter: i32,
            biClrUsed: u32,
            biClrImportant: u32,
        }

        #[repr(C)]
        #[allow(non_snake_case)]
        struct BitmapInfo {
            bmiHeader: BitmapInfoHeader,
            bmiColors: [u32; 1],
        }

        type HDC = *mut std::ffi::c_void;
        type HBITMAP = *mut std::ffi::c_void;
        type HGDIOBJ = *mut std::ffi::c_void;
        type HWND = *mut std::ffi::c_void;

        extern "system" {
            fn GetWindowRect(hwnd: HWND, rect: *mut Rect) -> i32;
            fn GetDC(hwnd: HWND) -> HDC;
            fn CreateCompatibleDC(hdc: HDC) -> HDC;
            fn CreateCompatibleBitmap(hdc: HDC, width: i32, height: i32) -> HBITMAP;
            fn SelectObject(hdc: HDC, obj: HGDIOBJ) -> HGDIOBJ;
            fn BitBlt(
                dest: HDC,
                x: i32,
                y: i32,
                w: i32,
                h: i32,
                src: HDC,
                sx: i32,
                sy: i32,
                rop: u32,
            ) -> i32;
            fn PrintWindow(hwnd: HWND, hdc: HDC, flags: u32) -> i32;
            fn GetDIBits(
                hdc: HDC,
                bmp: HBITMAP,
                start: u32,
                lines: u32,
                bits: *mut u8,
                info: *mut BitmapInfo,
                usage: u32,
            ) -> i32;
            fn DeleteObject(obj: HGDIOBJ) -> i32;
            fn DeleteDC(hdc: HDC) -> i32;
            fn ReleaseDC(hwnd: HWND, hdc: HDC) -> i32;
        }

        const SRCCOPY: u32 = 0x00CC0020;
        const PW_CLIENTONLY: u32 = 1;
        const BI_RGB: u32 = 0;
        const DIB_RGB_COLORS: u32 = 0;

        unsafe {
            let hwnd_ptr = hwnd as HWND;

            // Get window dimensions
            let mut rect = Rect {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if GetWindowRect(hwnd_ptr, &mut rect) == 0 {
                return Err(LlmError::ToolCall("Failed to get window rect".to_string()));
            }

            let width = (rect.right - rect.left).max(1) as u32;
            let height = (rect.bottom - rect.top).max(1) as u32;

            // Cap at 4K to prevent excessive memory usage
            if width > 3840 || height > 2160 {
                return Err(LlmError::ToolCall(format!(
                    "Window too large for screenshot: {}x{} (max 3840x2160)",
                    width, height
                )));
            }

            // Create memory DC and bitmap
            let screen_dc = GetDC(hwnd_ptr);
            if screen_dc.is_null() {
                return Err(LlmError::ToolCall("Failed to get window DC".to_string()));
            }

            let mem_dc = CreateCompatibleDC(screen_dc);
            if mem_dc.is_null() {
                ReleaseDC(hwnd_ptr, screen_dc);
                return Err(LlmError::ToolCall("Failed to create memory DC".to_string()));
            }

            let bitmap = CreateCompatibleBitmap(screen_dc, width as i32, height as i32);
            if bitmap.is_null() {
                DeleteDC(mem_dc);
                ReleaseDC(hwnd_ptr, screen_dc);
                return Err(LlmError::ToolCall("Failed to create bitmap".to_string()));
            }

            let old_bitmap = SelectObject(mem_dc, bitmap);

            // Try PrintWindow first (works for occluded windows)
            let captured = PrintWindow(hwnd_ptr, mem_dc, PW_CLIENTONLY);
            if captured == 0 {
                // Fallback to BitBlt (only works for visible windows)
                BitBlt(
                    mem_dc,
                    0,
                    0,
                    width as i32,
                    height as i32,
                    screen_dc,
                    0,
                    0,
                    SRCCOPY,
                );
            }

            // Extract pixel data
            let mut bmi = BitmapInfo {
                bmiHeader: BitmapInfoHeader {
                    biSize: std::mem::size_of::<BitmapInfoHeader>() as u32,
                    biWidth: width as i32,
                    biHeight: -(height as i32), // negative = top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [0],
            };

            let pixel_count = (width * height * 4) as usize;
            let mut pixels = vec![0u8; pixel_count];

            let lines = GetDIBits(
                mem_dc,
                bitmap,
                0,
                height,
                pixels.as_mut_ptr(),
                &mut bmi,
                DIB_RGB_COLORS,
            );

            // Cleanup GDI objects
            SelectObject(mem_dc, old_bitmap);
            DeleteObject(bitmap);
            DeleteDC(mem_dc);
            ReleaseDC(hwnd_ptr, screen_dc);

            if lines == 0 {
                return Err(LlmError::ToolCall(
                    "Failed to get bitmap pixels".to_string(),
                ));
            }

            // Convert BGRA → RGBA (Windows GDI uses BGRA byte order)
            for chunk in pixels.chunks_exact_mut(4) {
                chunk.swap(0, 2); // B ↔ R
            }

            Ok((width, height, pixels))
        }
    }

    /// Encode raw RGBA pixels as a PNG using a minimal encoder.
    fn encode_png(width: u32, height: u32, rgba_pixels: &[u8]) -> Result<Vec<u8>> {
        let mut png_buf = Vec::new();
        {
            let encoder = image::codecs::png::PngEncoder::new(&mut png_buf);
            image::ImageEncoder::write_image(
                encoder,
                rgba_pixels,
                width,
                height,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| LlmError::ToolCall(format!("PNG encoding failed: {e}")))?;
        }
        Ok(png_buf)
    }
}

impl UiAutomationBackend for WindowsUiaBackend {
    fn is_supported(&self) -> bool {
        true
    }

    fn launch_app(&self, executable: &str, args: &[String]) -> Result<LaunchedProcess> {
        let mut cmd = std::process::Command::new(executable);
        cmd.args(args);

        let child = cmd
            .spawn()
            .map_err(|e| LlmError::ToolCall(format!("Failed to launch '{}': {}", executable, e)))?;

        let pid = child.id();

        // Brief wait for window to appear
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Try to find the window title
        let window_title = if let Ok(automation) = UIAutomation::new() {
            let root = automation.get_root_element().ok();
            root.and_then(|r| {
                automation
                    .create_matcher()
                    .from(r)
                    .timeout(3000)
                    .filter_fn(Box::new(move |e: &UIElement| {
                        Ok(e.get_process_id().unwrap_or(0) == pid)
                    }))
                    .find_first()
                    .ok()
                    .and_then(|w| w.get_name().ok())
            })
        } else {
            None
        };

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

                let process_name = Self::get_process_name_by_pid(pid).unwrap_or_default();

                // Apply process filter
                if let Some(filter) = process_filter {
                    if !process_name.to_lowercase().contains(&filter.to_lowercase()) {
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
        let automation = Self::create_automation()?;
        let window = Self::find_window(&automation, window_title, process_name)?;
        let win_title = window.get_name().unwrap_or_default();

        // Polling retry: try every POLL_INTERVAL_MS until timeout_ms expires.
        // This handles dynamic UIs (Edge, Electron, etc.) where elements appear after load.
        const POLL_INTERVAL_MS: u64 = 500;
        let per_attempt_timeout = POLL_INTERVAL_MS.min(timeout_ms);
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms.max(1000));
        let mut last_err;

        loop {
            // Re-find the window each attempt in case title changed (e.g. page loaded)
            let search_root = Self::find_window(&automation, window_title, process_name)
                .unwrap_or_else(|_| window.clone());

            let mut matcher = automation
                .create_matcher()
                .from(search_root)
                .depth(FIND_ELEMENT_DEPTH)
                .timeout(per_attempt_timeout);

            if let Some(name) = element_name {
                matcher = matcher.contains_name(name);
            }

            if let Some(et) = element_type {
                if let Some(ct) = Self::parse_control_type(et) {
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
                    return Self::build_found_element(&element, &win_title, 0);
                }
                Err(e) => {
                    last_err = e.to_string();
                }
            }

            if std::time::Instant::now() >= deadline {
                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS));
        }

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
        let window = Self::find_window(&automation, window_title, process_name)?;
        let win_title = window.get_name().unwrap_or_default();

        let mut matcher = automation
            .create_matcher()
            .from(window)
            .depth(FIND_ELEMENT_DEPTH)
            .timeout(timeout_ms);

        if let Some(name) = element_name {
            matcher = matcher.contains_name(name);
        }

        if let Some(et) = element_type {
            if let Some(ct) = Self::parse_control_type(et) {
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
            results.push(Self::build_found_element(elem, &win_title, i as u32)?);
        }

        Ok(results)
    }

    fn click_element(&self, element_ref: &str) -> Result<()> {
        let automation = Self::create_automation()?;
        let (element, _) = Self::decode_and_find(&automation, element_ref)?;

        if !element.is_enabled().unwrap_or(false) {
            return Err(LlmError::ToolCall("Element is disabled".to_string()));
        }

        // Try InvokePattern first (semantic click, no mouse movement)
        if let Ok(invoke) = element.get_pattern::<UIInvokePattern>() {
            invoke
                .invoke()
                .map_err(|e| LlmError::ToolCall(format!("Invoke failed: {e}")))?;
            return Ok(());
        }

        // Try TogglePattern (checkboxes, toggle buttons)
        if let Ok(toggle) = element.get_pattern::<UITogglePattern>() {
            toggle
                .toggle()
                .map_err(|e| LlmError::ToolCall(format!("Toggle failed: {e}")))?;
            return Ok(());
        }

        // Try SelectionItemPattern (list items, radio buttons, tab items)
        if let Ok(sel_item) = element.get_pattern::<UISelectionItemPattern>() {
            sel_item
                .select()
                .map_err(|e| LlmError::ToolCall(format!("Selection failed: {e}")))?;
            return Ok(());
        }

        // Try ExpandCollapsePattern (tree items, combo boxes, menus)
        if let Ok(expand) = element.get_pattern::<uiautomation::patterns::UIExpandCollapsePattern>()
        {
            use uiautomation::types::ExpandCollapseState;
            let state = expand.get_state().unwrap_or(ExpandCollapseState::Collapsed);
            if state == ExpandCollapseState::Collapsed {
                expand
                    .expand()
                    .map_err(|e| LlmError::ToolCall(format!("Expand failed: {e}")))?;
            } else {
                expand
                    .collapse()
                    .map_err(|e| LlmError::ToolCall(format!("Collapse failed: {e}")))?;
            }
            return Ok(());
        }

        // Fallback: coordinate-based click using element bounding rect center
        let rect = element.get_bounding_rectangle().map_err(|e| {
            LlmError::ToolCall(format!(
                "Element has no interaction pattern and no bounding rect for click fallback: {e}"
            ))
        })?;

        let center_x = (rect.get_left() + rect.get_right()) / 2;
        let center_y = (rect.get_top() + rect.get_bottom()) / 2;

        if center_x <= 0 && center_y <= 0 {
            return Err(LlmError::ToolCall(
                "Element has no interaction pattern and bounding rect is offscreen".to_string(),
            ));
        }

        Self::click_at_coordinates(center_x, center_y)?;
        Ok(())
    }

    fn type_text(&self, element_ref: &str, text: &str) -> Result<()> {
        let automation = Self::create_automation()?;
        let (element, win_title) = Self::decode_and_find(&automation, element_ref)?;

        // Hard block: password fields
        if element.is_password().unwrap_or(false) {
            return Err(LlmError::ToolCall(
                "Cannot type into password fields (security restriction)".to_string(),
            ));
        }

        if !element.is_enabled().unwrap_or(false) {
            return Err(LlmError::ToolCall("Element is disabled".to_string()));
        }

        // Try ValuePattern first (most reliable)
        if let Ok(value_pattern) = element.get_pattern::<UIValuePattern>() {
            if value_pattern.is_readonly().unwrap_or(true) {
                return Err(LlmError::ToolCall("Element is read-only".to_string()));
            }
            value_pattern
                .set_value(text)
                .map_err(|e| LlmError::ToolCall(format!("Failed to set value: {e}")))?;
            return Ok(());
        }

        // Fallback using focus + send keys.
        // Fix: bring the parent window to foreground first to prevent race condition
        // where another window steals focus between set_focus and send_keys.
        if !element.is_keyboard_focusable().unwrap_or(false) {
            return Err(LlmError::ToolCall(
                "Element does not support text input (no ValuePattern and not keyboard-focusable)"
                    .to_string(),
            ));
        }

        // Find and foreground the parent window to reduce focus race risk
        if let Ok(window) = Self::find_window(&automation, Some(&win_title), None) {
            if let Ok(handle) = window.get_native_window_handle() {
                let hwnd: isize = handle.into();
                let _ = Self::set_foreground(hwnd);
            }
        }

        // Small delay after foregrounding
        std::thread::sleep(std::time::Duration::from_millis(50));

        element
            .set_focus()
            .map_err(|e| LlmError::ToolCall(format!("Failed to focus element: {e}")))?;

        // Brief pause to let focus settle
        std::thread::sleep(std::time::Duration::from_millis(30));

        element
            .send_keys(text, 10)
            .map_err(|e| LlmError::ToolCall(format!("Failed to send keys: {e}")))?;

        Ok(())
    }

    fn read_text(&self, element_ref: &str) -> Result<String> {
        let automation = Self::create_automation()?;
        let (element, _) = Self::decode_and_find(&automation, element_ref)?;

        // Hard block: password fields
        if element.is_password().unwrap_or(false) {
            return Err(LlmError::ToolCall(
                "Cannot read password fields (security restriction)".to_string(),
            ));
        }

        // Try ValuePattern
        if let Ok(value_pattern) = element.get_pattern::<UIValuePattern>() {
            return value_pattern
                .get_value()
                .map_err(|e| LlmError::ToolCall(format!("Failed to read value: {e}")));
        }

        // Try TextPattern for rich text controls (documents, etc.)
        if let Ok(text_pattern) = element.get_pattern::<UITextPattern>() {
            if let Ok(range) = text_pattern.get_document_range() {
                if let Ok(text) = range.get_text(10_000) {
                    return Ok(text);
                }
            }
        }

        // Fall back to element name
        element
            .get_name()
            .map_err(|e| LlmError::ToolCall(format!("Failed to read element text: {e}")))
    }

    fn get_tree(
        &self,
        window_title: Option<&str>,
        process_name: Option<&str>,
        max_depth: u32,
        compact: bool,
    ) -> Result<UiElementTree> {
        let automation = Self::create_automation()?;
        let window = Self::find_window(&automation, window_title, process_name)?;
        let win_title = window.get_name().unwrap_or_default();
        let win_pid = window.get_process_id().unwrap_or(0);
        let proc_name = Self::get_process_name_by_pid(win_pid).unwrap_or_default();

        let walker = automation
            .get_control_view_walker()
            .map_err(|e| LlmError::ToolCall(format!("Failed to get tree walker: {e}")))?;

        let clamped_depth = max_depth.min(MAX_TREE_DEPTH);
        let mut count = 0u32;
        let mut actual_depth = 0u32;

        let root_node = Self::walk_tree_recursive(
            &walker,
            &window,
            &win_title,
            0,
            clamped_depth,
            &mut count,
            MAX_TREE_ELEMENTS,
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

        Ok(Self::build_tree_result(
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
        let (element, win_title) = Self::decode_and_find(&automation, element_ref)?;

        let elem_name = element.get_name().unwrap_or_default();
        let pid = element.get_process_id().unwrap_or(0);
        let proc_name = Self::get_process_name_by_pid(pid).unwrap_or_default();

        let walker = automation
            .get_control_view_walker()
            .map_err(|e| LlmError::ToolCall(format!("Failed to get tree walker: {e}")))?;

        let clamped_depth = max_depth.min(MAX_TREE_DEPTH);
        let mut count = 0u32;
        let mut actual_depth = 0u32;

        let root_node = Self::walk_tree_recursive(
            &walker,
            &element,
            &win_title,
            0,
            clamped_depth,
            &mut count,
            MAX_TREE_ELEMENTS,
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

        Ok(Self::build_tree_result(
            win_title,
            proc_name,
            root_node,
            count,
            actual_depth,
            clamped_depth,
        ))
    }

    fn close_window(&self, window_title: Option<&str>, process_name: Option<&str>) -> Result<()> {
        let automation = Self::create_automation()?;
        let window = Self::find_window(&automation, window_title, process_name)?;

        let win_pattern = window
            .get_pattern::<UIWindowPattern>()
            .map_err(|e| LlmError::ToolCall(format!("Window does not support close: {e}")))?;
        win_pattern
            .close()
            .map_err(|e| LlmError::ToolCall(format!("Failed to close window: {e}")))?;
        Ok(())
    }

    fn is_password_field(&self, element_ref: &str) -> Result<bool> {
        let automation = Self::create_automation()?;
        let (element, _) = Self::decode_and_find(&automation, element_ref)?;
        Ok(element.is_password().unwrap_or(false))
    }

    fn screenshot_window(
        &self,
        window_title: Option<&str>,
        process_name: Option<&str>,
    ) -> Result<ScreenshotResult> {
        let automation = Self::create_automation()?;
        // Fix: single find_window call instead of separate get_window_hwnd + find_window
        let window = Self::find_window(&automation, window_title, process_name)?;
        let win_title = window.get_name().unwrap_or_default();
        let handle = window
            .get_native_window_handle()
            .map_err(|e| LlmError::ToolCall(format!("Failed to get window handle: {e}")))?;
        let hwnd: isize = handle.into();

        let (width, height, rgba_pixels) = Self::capture_window_pixels(hwnd)?;
        let png_bytes = Self::encode_png(width, height, &rgba_pixels)?;
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png_bytes);

        Ok(ScreenshotResult {
            image_base64: b64,
            mime_type: "image/png".to_string(),
            width,
            height,
            window_title: win_title,
        })
    }

    fn press_keys(
        &self,
        window_title: Option<&str>,
        process_name: Option<&str>,
        keys: &str,
    ) -> Result<()> {
        // Parse the key combo first to fail fast on invalid keys
        let key_codes = Self::parse_key_combo(keys)?;
        if key_codes.is_empty() {
            return Err(LlmError::ToolCall("No keys specified".to_string()));
        }

        // Focus the target window first
        let automation = Self::create_automation()?;
        let window = Self::find_window(&automation, window_title, process_name)?;
        let handle = window
            .get_native_window_handle()
            .map_err(|e| LlmError::ToolCall(format!("Failed to get window handle: {e}")))?;
        let hwnd: isize = handle.into();
        Self::set_foreground(hwnd)?;

        // Press all keys down in order
        Self::send_input_keyboard(&key_codes, true)?;

        // Small delay between press and release
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Release all keys in reverse order
        let reversed: Vec<(u16, bool)> = key_codes.iter().rev().copied().collect();
        Self::send_input_keyboard(&reversed, false)?;

        std::thread::sleep(std::time::Duration::from_millis(100));
        Ok(())
    }

    fn scroll(
        &self,
        window_title: Option<&str>,
        process_name: Option<&str>,
        element_ref: Option<&str>,
        amount: i32,
    ) -> Result<()> {
        let automation = Self::create_automation()?;

        // If element_ref provided, try UIScrollPattern first, then use element center for wheel
        if let Some(eref) = element_ref {
            let (element, _) = Self::decode_and_find(&automation, eref)?;

            // Try UIA ScrollPattern
            if let Ok(scroll_pattern) =
                element.get_pattern::<uiautomation::patterns::UIScrollPattern>()
            {
                use uiautomation::types::ScrollAmount;
                let scroll_amount = if amount > 0 {
                    ScrollAmount::LargeIncrement
                } else {
                    ScrollAmount::LargeDecrement
                };

                let repeat = amount.unsigned_abs();
                for _ in 0..repeat {
                    scroll_pattern
                        .scroll(uiautomation::types::ScrollAmount::NoAmount, scroll_amount)
                        .map_err(|e| LlmError::ToolCall(format!("Scroll failed: {e}")))?;
                }
                return Ok(());
            }

            // Fallback: scroll at element center using mouse wheel
            if let Ok(rect) = element.get_bounding_rectangle() {
                let cx = (rect.get_left() + rect.get_right()) / 2;
                let cy = (rect.get_top() + rect.get_bottom()) / 2;
                return Self::scroll_at_coordinates(cx, cy, amount);
            }
        }

        // No element -- scroll at center of window
        let window = Self::find_window(&automation, window_title, process_name)?;
        let rect = window
            .get_bounding_rectangle()
            .map_err(|e| LlmError::ToolCall(format!("Failed to get window rect: {e}")))?;
        let cx = (rect.get_left() + rect.get_right()) / 2;
        let cy = (rect.get_top() + rect.get_bottom()) / 2;
        Self::scroll_at_coordinates(cx, cy, amount)
    }

    fn focus_window(&self, window_title: Option<&str>, process_name: Option<&str>) -> Result<()> {
        let automation = Self::create_automation()?;
        let window = Self::find_window(&automation, window_title, process_name)?;
        let handle = window
            .get_native_window_handle()
            .map_err(|e| LlmError::ToolCall(format!("Failed to get window handle: {e}")))?;
        let hwnd: isize = handle.into();
        Self::set_foreground(hwnd)
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
    fn test_encode_element_ref() {
        let r = WindowsUiaBackend::encode_element_ref("Notepad", "OK", "Button", "btnOk", 0);
        assert!(r.starts_with("win:"));
        assert!(r.contains("|name:OK|"));
        assert!(r.contains("|type:Button|"));
        assert!(r.contains("|aid:btnOk|"));
        assert!(r.contains("|idx:0"));
    }

    #[test]
    fn test_hash_title_deterministic() {
        let h1 = WindowsUiaBackend::hash_title("Notepad");
        let h2 = WindowsUiaBackend::hash_title("Notepad");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_title_different_for_different_titles() {
        let h1 = WindowsUiaBackend::hash_title("Notepad");
        let h2 = WindowsUiaBackend::hash_title("Calculator");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_title_is_32_bit() {
        let h = WindowsUiaBackend::hash_title("Test Window");
        assert_eq!(
            h.len(),
            8,
            "Expected 8 hex chars (32 bits), got {}: {}",
            h.len(),
            h
        );
    }

    #[test]
    fn test_parse_control_type() {
        assert_eq!(
            WindowsUiaBackend::parse_control_type("Button"),
            Some(ControlType::Button)
        );
        assert_eq!(
            WindowsUiaBackend::parse_control_type("Edit"),
            Some(ControlType::Edit)
        );
        assert_eq!(
            WindowsUiaBackend::parse_control_type("Window"),
            Some(ControlType::Window)
        );
        assert_eq!(WindowsUiaBackend::parse_control_type("Unknown"), None);
    }

    #[test]
    fn test_parse_control_type_appbar_semanticzoom() {
        assert_eq!(
            WindowsUiaBackend::parse_control_type("AppBar"),
            Some(ControlType::AppBar)
        );
        assert_eq!(
            WindowsUiaBackend::parse_control_type("SemanticZoom"),
            Some(ControlType::SemanticZoom)
        );
    }

    #[test]
    fn test_control_type_name_roundtrip() {
        let types = vec![
            ControlType::Button,
            ControlType::Edit,
            ControlType::Window,
            ControlType::MenuItem,
            ControlType::Text,
            ControlType::AppBar,
            ControlType::SemanticZoom,
        ];
        for ct in types {
            let name = WindowsUiaBackend::control_type_name(ct);
            let parsed = WindowsUiaBackend::parse_control_type(name);
            assert_eq!(parsed, Some(ct));
        }
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
    fn test_get_process_name_by_pid_current_process() {
        let pid = std::process::id();
        let name = WindowsUiaBackend::get_process_name_by_pid(pid);
        assert!(name.is_some(), "Expected to read our own process name");
        let name = name.unwrap();
        assert!(!name.is_empty());
    }

    #[test]
    fn test_find_elements_empty_filter() {
        let backend = WindowsUiaBackend::new();
        // Search for a non-existent element name should return empty or error
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

    #[test]
    fn test_depth_constants() {
        assert_eq!(MAX_TREE_DEPTH, 8);
        assert!(FIND_ELEMENT_DEPTH >= MAX_TREE_DEPTH);
    }
}
