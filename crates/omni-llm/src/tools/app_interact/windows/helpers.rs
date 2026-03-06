#![cfg(windows)]

use uiautomation::patterns::{
    UIExpandCollapsePattern, UIInvokePattern, UIScrollPattern, UISelectionItemPattern,
    UITextPattern, UITogglePattern, UIValuePattern, UIWindowPattern,
};
use uiautomation::types::ControlType;
use uiautomation::{UIAutomation, UIElement};

use super::super::types::*;
use crate::error::{LlmError, Result};

/// Known system window class names to skip when searching for user windows.
pub const SYSTEM_WINDOW_CLASSES: &[&str] = &[
    "Progman",
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
    "WorkerW",
    "DV2ControlHost",
    "Windows.UI.Core.CoreWindow",
    "ForegroundStaging",
    "MultitaskingViewFrame",
];

/// Check if a window element is a usable application window.
/// Chromium-based windows (Edge, Chrome, Electron) are considered usable even
/// with zero bounding rect, since they may still be initializing their frame.
pub fn is_usable_window(element: &UIElement) -> bool {
    let title = element.get_name().unwrap_or_default();
    if title.is_empty() {
        return false;
    }

    let class = element.get_classname().unwrap_or_default();
    if SYSTEM_WINDOW_CLASSES
        .iter()
        .any(|&c| c.eq_ignore_ascii_case(&class))
    {
        eprintln!("[app_interact] is_usable_window SKIP system class='{}' title='{}'", class, title);
        return false;
    }

    // Skip bounding-rect check for Chromium windows — they may have zero-height
    // frames during startup but are still valid targets for UIA interaction.
    let is_chromium_class = super::chromium::CHROMIUM_CLASSES
        .iter()
        .any(|&c| c.eq_ignore_ascii_case(&class));

    if !is_chromium_class {
        if let Ok(rect) = element.get_bounding_rectangle() {
            let w = rect.get_right() - rect.get_left();
            let h = rect.get_bottom() - rect.get_top();
            if w <= 0 || h <= 0 {
                eprintln!("[app_interact] is_usable_window SKIP zero-size class='{}' title='{}' ({}x{})", class, title, w, h);
                return false;
            }
        }
    }

    true
}

/// Strip Unicode control/format characters for fuzzy window title matching.
/// Covers zero-width chars, directional overrides, variation selectors, and
/// bidirectional markers commonly injected by Edge/Chrome/Electron.
pub fn normalize_for_comparison(s: &str) -> String {
    s.chars()
        .filter(|c| {
            !c.is_control()
                && !matches!(
                    c,
                    '\u{00A0}'               // Non-breaking space
                        | '\u{00AD}'         // Soft hyphen
                        | '\u{200B}'..='\u{200F}' // Zero-width space, ZWNJ, ZWJ, LRM, RLM
                        | '\u{202A}'..='\u{202E}' // Bidi embedding (LRE, RLE, PDF, LRO, RLO)
                        | '\u{2060}'..='\u{2064}' // Word joiner, invisible times/separator/plus
                        | '\u{2066}'..='\u{2069}' // Bidi isolates (LRI, RLI, FSI, PDI)
                        | '\u{FEFF}'         // Zero-width no-break space (BOM)
                        | '\u{FE00}'..='\u{FE0F}' // Variation selectors
                        | '\u{FFF9}'..='\u{FFFB}' // Interlinear annotations
                )
        })
        .collect::<String>()
        .to_lowercase()
}

/// Find a window by title substring or process name.
///
/// Uses UITreeWalker.get_children() for fast, reliable enumeration of top-level
/// windows. The previous approach using create_matcher().find_all() was timing out
/// on systems with many windows (3s timeout exceeded by UIA element tree walk).
pub fn find_window(
    automation: &UIAutomation,
    window_title: Option<&str>,
    process_name: Option<&str>,
) -> Result<UIElement> {
    eprintln!("[app_interact] find_window called: title={:?} process={:?}", window_title, process_name);

    let root = automation
        .get_root_element()
        .map_err(|e| LlmError::ToolCall(format!("Failed to get desktop root: {e}")))?;

    let walker = automation
        .get_control_view_walker()
        .map_err(|e| LlmError::ToolCall(format!("Failed to get tree walker: {e}")))?;

    // Use tree walker to enumerate top-level windows (fast, no timeout issues)
    let all_children = walker.get_children(&root).unwrap_or_default();
    eprintln!("[app_interact] find_window: walker returned {} top-level children", all_children.len());

    // Collect windows with their titles for matching
    let mut windows: Vec<UIElement> = Vec::new();

    if let Some(title) = window_title {
        let search_lower = title.to_lowercase();
        let search_norm = normalize_for_comparison(title);

        for child in &all_children {
            let wname = child.get_name().unwrap_or_default();
            if wname.is_empty() {
                continue;
            }

            // Try direct case-insensitive substring match first
            let wname_lower = wname.to_lowercase();
            if wname_lower.contains(&search_lower) {
                let c = child.get_classname().unwrap_or_default();
                let pid = child.get_process_id().unwrap_or(0);
                eprintln!("[app_interact] find_window: direct match title='{}' class='{}' pid={}", wname, c, pid);
                windows.push(child.clone());
                continue;
            }

            // Fallback: normalized comparison for Unicode edge cases
            let w_norm = normalize_for_comparison(&wname);
            if w_norm.contains(&search_norm) {
                let c = child.get_classname().unwrap_or_default();
                let pid = child.get_process_id().unwrap_or(0);
                eprintln!("[app_interact] find_window: Unicode match title='{}' (norm='{}') class='{}' pid={}", wname, w_norm, c, pid);
                windows.push(child.clone());
            }
        }
        eprintln!("[app_interact] find_window: title search matched {} windows", windows.len());
    } else {
        // No title filter — use all children
        for child in &all_children {
            let wname = child.get_name().unwrap_or_default();
            if !wname.is_empty() {
                windows.push(child.clone());
            }
        }
        eprintln!("[app_interact] find_window: no title filter, {} named windows", windows.len());
    }

    // Filter to usable windows
    let usable: Vec<&UIElement> = windows.iter().filter(|w| is_usable_window(w)).collect();
    eprintln!("[app_interact] find_window: {} usable windows after filter", usable.len());

    // Filter by process name if specified
    if let Some(proc_name) = process_name {
        let proc_lower = proc_name.to_lowercase();
        let mut matched = Vec::new();
        let candidates = if usable.is_empty() {
            &windows
        } else {
            &usable.iter().map(|w| (*w).clone()).collect::<Vec<_>>()
        };
        eprintln!("[app_interact] find_window: checking {} candidates for process '{}'", candidates.len(), proc_name);
        for win in candidates {
            let pid = win.get_process_id().unwrap_or(0);
            let win_title = win.get_name().unwrap_or_default();
            if let Some(pname) = get_process_name_by_pid(pid) {
                let pname_lower = pname.to_lowercase();
                let is_match = pname_lower.contains(&proc_lower);
                eprintln!("[app_interact] find_window:   pid={} process='{}' title='{}' match={}", pid, pname, win_title, is_match);
                if is_match {
                    matched.push(win.clone());
                }
            } else {
                eprintln!("[app_interact] find_window:   pid={} process=<unknown> title='{}'", pid, win_title);
            }
        }

        if matched.is_empty() {
            eprintln!("[app_interact] find_window: NO matches for process '{}'", proc_name);
            return Err(LlmError::ToolCall(format!(
                "No window found for process '{}'",
                proc_name
            )));
        }

        eprintln!("[app_interact] find_window: {} process matches, picking best visible", matched.len());
        for win in &matched {
            if !win.is_offscreen().unwrap_or(true) {
                let t = win.get_name().unwrap_or_default();
                eprintln!("[app_interact] find_window: returning visible window '{}'", t);
                return Ok(win.clone());
            }
        }

        let t = matched[0].get_name().unwrap_or_default();
        eprintln!("[app_interact] find_window: no visible match, returning first: '{}'", t);
        return Ok(matched.into_iter().next().unwrap());
    }

    // No process filter
    let candidates = if usable.is_empty() {
        windows.iter().collect::<Vec<_>>()
    } else {
        usable
    };

    let mut best_visible = None;
    let mut first_any = None;
    for win in candidates {
        if !win.is_offscreen().unwrap_or(true) && best_visible.is_none() {
            best_visible = Some(win.clone());
        }
        if first_any.is_none() {
            first_any = Some(win.clone());
        }
    }

    best_visible.or(first_any).ok_or_else(|| {
        eprintln!("[app_interact] find_window: FAILED — no window found for title={:?} process={:?}", window_title, process_name);
        LlmError::ToolCall(format!(
            "No window found matching '{}'",
            window_title.unwrap_or("(no filter)")
        ))
    })
}

/// Find a window with retry — polls until the window appears or timeout expires.
/// Essential for post-launch scenarios where the app may still be starting.
pub fn find_window_with_retry(
    automation: &uiautomation::UIAutomation,
    window_title: Option<&str>,
    process_name: Option<&str>,
    timeout_ms: u64,
) -> Result<uiautomation::UIElement> {
    eprintln!("[app_interact] find_window_with_retry: title={:?} process={:?} timeout={}ms", window_title, process_name, timeout_ms);
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms.max(2000));
    let mut last_err = String::new();
    let mut attempt = 0u32;

    loop {
        attempt += 1;
        eprintln!("[app_interact] find_window_with_retry: attempt #{}", attempt);
        match find_window(automation, window_title, process_name) {
            Ok(w) => {
                let t = w.get_name().unwrap_or_default();
                eprintln!("[app_interact] find_window_with_retry: SUCCESS on attempt #{} — '{}'", attempt, t);
                return Ok(w);
            }
            Err(e) => {
                last_err = e.to_string();
                eprintln!("[app_interact] find_window_with_retry: attempt #{} failed: {}", attempt, last_err);
                if std::time::Instant::now() >= deadline {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
    }

    eprintln!("[app_interact] find_window_with_retry: FAILED after {} attempts ({}ms): {}", attempt, timeout_ms, last_err);
    Err(LlmError::ToolCall(format!(
        "Window not found after {}ms: {}",
        timeout_ms, last_err
    )))
}

/// Get process name from PID using Windows API.
pub fn get_process_name_by_pid(pid: u32) -> Option<String> {
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

        let mut buf = [0u16; 260];
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
pub fn parse_control_type(s: &str) -> Option<ControlType> {
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
pub fn control_type_name(ct: ControlType) -> &'static str {
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
pub fn build_found_element(
    element: &UIElement,
    _window_title: &str,
    window_hwnd: isize,
    index: u32,
) -> Result<FoundElement> {
    let name = element.get_name().unwrap_or_default();
    let ct = element.get_control_type().unwrap_or(ControlType::Custom);
    let ct_name = control_type_name(ct).to_string();
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
    if element.get_pattern::<UIExpandCollapsePattern>().is_ok() {
        patterns.push("ExpandCollapse".to_string());
    }
    if element.get_pattern::<UIScrollPattern>().is_ok() {
        patterns.push("Scroll".to_string());
    }

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

    let element_ref =
        super::element_ref::encode_element_ref(element, window_hwnd, &name, &ct_name, &aid, index);

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

/// Get the native window handle (HWND) from a UIElement.
pub fn get_window_hwnd(element: &UIElement) -> Result<isize> {
    let handle = element
        .get_native_window_handle()
        .map_err(|e| LlmError::ToolCall(format!("Failed to get window handle: {e}")))?;
    Ok(handle.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_control_type() {
        assert_eq!(parse_control_type("Button"), Some(ControlType::Button));
        assert_eq!(parse_control_type("Edit"), Some(ControlType::Edit));
        assert_eq!(parse_control_type("Window"), Some(ControlType::Window));
        assert_eq!(parse_control_type("Unknown"), None);
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
            let name = control_type_name(ct);
            let parsed = parse_control_type(name);
            assert_eq!(parsed, Some(ct));
        }
    }

    #[test]
    fn test_normalize_for_comparison() {
        assert_eq!(normalize_for_comparison("Hello World"), "hello world");
        // Zero-width space should be stripped
        assert_eq!(normalize_for_comparison("He\u{200B}llo"), "hello");
    }

    #[test]
    fn test_get_process_name_current() {
        let pid = std::process::id();
        let name = get_process_name_by_pid(pid);
        assert!(name.is_some());
        assert!(!name.unwrap().is_empty());
    }
}
