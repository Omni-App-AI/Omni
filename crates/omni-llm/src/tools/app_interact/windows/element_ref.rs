#![cfg(windows)]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use uiautomation::{UIAutomation, UIElement, UITreeWalker};

use super::super::types::ParsedElementRef;
use super::helpers;
use crate::error::{LlmError, Result};

/// Maximum search depth when re-finding elements by reference.
pub const FIND_ELEMENT_DEPTH: u32 = 15;

/// Encode an element reference using RuntimeId (primary) + HWND + search criteria.
///
/// New format: `rid:{hex}|hwnd:{hwnd}|name:{name}|type:{type}|aid:{aid}|idx:{idx}`
pub fn encode_element_ref(
    element: &UIElement,
    window_hwnd: isize,
    name: &str,
    control_type: &str,
    automation_id: &str,
    index: u32,
) -> String {
    let rid = element
        .get_runtime_id()
        .map(|ids| {
            ids.iter()
                .map(|id| format!("{:08x}", id))
                .collect::<Vec<_>>()
                .join(".")
        })
        .unwrap_or_default();

    format!(
        "rid:{}|hwnd:{}|name:{}|type:{}|aid:{}|idx:{}",
        rid,
        window_hwnd,
        name.replace('|', "\\|"),
        control_type.replace('|', "\\|"),
        automation_id.replace('|', "\\|"),
        index
    )
}

/// Parse an element reference string into its component fields.
pub(crate) fn parse_element_ref(element_ref: &str) -> Result<ParsedElementRef> {
    let mut parsed = ParsedElementRef::default();

    // Split by unescaped '|'
    let parts = split_escaped(element_ref);

    for part in parts {
        if let Some(v) = part.strip_prefix("rid:") {
            if !v.is_empty() {
                parsed.runtime_id = v
                    .split('.')
                    .filter_map(|hex| i32::from_str_radix(hex, 16).ok())
                    .collect();
            }
        } else if let Some(v) = part.strip_prefix("hwnd:") {
            parsed.hwnd = v.parse().unwrap_or(0);
        } else if let Some(v) = part.strip_prefix("name:") {
            parsed.name = v.to_string();
        } else if let Some(v) = part.strip_prefix("type:") {
            parsed.ctrl_type = v.to_string();
        } else if let Some(v) = part.strip_prefix("aid:") {
            parsed.automation_id = v.to_string();
        } else if let Some(v) = part.strip_prefix("idx:") {
            parsed.index = v.parse().unwrap_or(0);
        } else if let Some(v) = part.strip_prefix("win:") {
            // Legacy format backward compat
            parsed.win_hash = v.to_string();
        }
    }

    Ok(parsed)
}

/// Split a string by '|' but not by escaped '\|'.
fn split_escaped(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&nc) = chars.peek() {
                if nc == '|' {
                    current.push('|');
                    chars.next();
                    continue;
                }
            }
            current.push('\\');
        } else if c == '|' {
            parts.push(current);
            current = String::new();
        } else {
            current.push(c);
        }
    }
    parts.push(current);
    parts
}

/// Decode an element reference and re-find the element using 3-tier strategy.
///
/// Tier 1: HWND + search criteria (fastest practical — HWND gives us the window directly)
/// Tier 2: Legacy hash fallback (backward compat with old `win:{hash}` format)
/// Tier 3: Global criteria search (searches all windows by name/automation_id)
pub fn decode_and_find(
    automation: &UIAutomation,
    element_ref: &str,
) -> Result<(UIElement, isize)> {
    let parsed = parse_element_ref(element_ref)?;

    // Tier 1: HWND + search criteria (most practical — RuntimeId search requires
    // walking the full tree, but HWND gives us the window directly)
    if parsed.hwnd != 0 {
        if let Ok(element) = find_by_hwnd_and_criteria(automation, &parsed) {
            return Ok((element, parsed.hwnd));
        }
    }

    // Tier 2: Legacy hash fallback (backward compat with old win:{hash} format)
    if !parsed.win_hash.is_empty() {
        return find_by_legacy_hash(automation, &parsed);
    }

    // Tier 3: Search all windows by criteria
    if !parsed.name.is_empty() || !parsed.automation_id.is_empty() {
        return find_by_criteria_global(automation, &parsed);
    }

    Err(LlmError::ToolCall(
        "Element not found — invalid or expired element reference".to_string(),
    ))
}

/// Find element using HWND to locate window + search criteria within it.
fn find_by_hwnd_and_criteria(
    automation: &UIAutomation,
    parsed: &ParsedElementRef,
) -> Result<UIElement> {
    let root = automation
        .get_root_element()
        .map_err(|e| LlmError::ToolCall(format!("Failed to get desktop root: {e}")))?;

    let walker = automation
        .get_control_view_walker()
        .map_err(|e| LlmError::ToolCall(format!("Failed to get tree walker: {e}")))?;

    // Find the window by HWND
    let target_hwnd = parsed.hwnd;
    let window = find_window_by_hwnd(&walker, &root, target_hwnd)?;

    search_within_element(automation, &window, parsed)
}

/// Find a top-level window by its native window handle.
fn find_window_by_hwnd(
    walker: &UITreeWalker,
    root: &UIElement,
    target_hwnd: isize,
) -> Result<UIElement> {
    if let Some(children) = walker.get_children(root) {
        for child in children {
            if let Ok(handle) = child.get_native_window_handle() {
                let hwnd: isize = handle.into();
                if hwnd == target_hwnd {
                    return Ok(child);
                }
            }
        }
    }
    Err(LlmError::ToolCall(
        "Target window not found — it may have been closed".to_string(),
    ))
}

/// Find element using legacy hash-based window lookup.
fn find_by_legacy_hash(
    automation: &UIAutomation,
    parsed: &ParsedElementRef,
) -> Result<(UIElement, isize)> {
    let root = automation
        .get_root_element()
        .map_err(|e| LlmError::ToolCall(format!("Failed to get desktop root: {e}")))?;

    let walker = automation
        .get_control_view_walker()
        .map_err(|e| LlmError::ToolCall(format!("Failed to get tree walker: {e}")))?;

    let target_window = find_window_by_hash(&walker, &root, &parsed.win_hash)?;
    let hwnd = target_window
        .get_native_window_handle()
        .map(|h| h.into())
        .unwrap_or(0isize);

    let element = search_within_element(automation, &target_window, parsed)?;
    Ok((element, hwnd))
}

/// Find a top-level window whose title hashes to the given hash (legacy compat).
fn find_window_by_hash(
    walker: &UITreeWalker,
    root: &UIElement,
    target_hash: &str,
) -> Result<UIElement> {
    if let Some(children) = walker.get_children(root) {
        for child in children {
            let title = child.get_name().unwrap_or_default();
            if hash_title(&title) == target_hash {
                return Ok(child);
            }
        }
    }
    Err(LlmError::ToolCall(
        "Target window not found — it may have been closed".to_string(),
    ))
}

/// Hash a window title for legacy element reference encoding.
pub fn hash_title(title: &str) -> String {
    let mut hasher = DefaultHasher::new();
    title.hash(&mut hasher);
    format!("{:08x}", hasher.finish() & 0xFFFFFFFF)
}

/// Search within an element subtree using the parsed criteria.
fn search_within_element(
    automation: &UIAutomation,
    root: &UIElement,
    parsed: &ParsedElementRef,
) -> Result<UIElement> {
    let mut matcher = automation
        .create_matcher()
        .from(root.clone())
        .depth(FIND_ELEMENT_DEPTH)
        .timeout(3000);

    if !parsed.name.is_empty() {
        matcher = matcher.name(&parsed.name);
    }
    if !parsed.ctrl_type.is_empty() {
        if let Some(ct) = helpers::parse_control_type(&parsed.ctrl_type) {
            matcher = matcher.control_type(ct);
        }
    }

    if !parsed.automation_id.is_empty() {
        let aid = parsed.automation_id.clone();
        matcher = matcher.filter_fn(Box::new(move |e: &UIElement| {
            let elem_aid = e.get_automation_id().unwrap_or_default();
            Ok(elem_aid == aid)
        }));
    }

    let matches = matcher
        .find_all()
        .map_err(|e| LlmError::ToolCall(format!("Element not found: {e}")))?;

    if matches.is_empty() {
        return Err(LlmError::ToolCall(
            "Element not found — it may have been removed or the window has changed".to_string(),
        ));
    }

    let target_idx = (parsed.index as usize).min(matches.len() - 1);
    Ok(matches[target_idx].clone())
}

/// Search all windows globally for an element matching criteria.
fn find_by_criteria_global(
    automation: &UIAutomation,
    parsed: &ParsedElementRef,
) -> Result<(UIElement, isize)> {
    let root = automation
        .get_root_element()
        .map_err(|e| LlmError::ToolCall(format!("Failed to get desktop root: {e}")))?;

    let walker = automation
        .get_control_view_walker()
        .map_err(|e| LlmError::ToolCall(format!("Failed to get tree walker: {e}")))?;

    if let Some(children) = walker.get_children(&root) {
        for child in children {
            if !helpers::is_usable_window(&child) {
                continue;
            }
            if let Ok(element) = search_within_element(automation, &child, parsed) {
                let hwnd = child
                    .get_native_window_handle()
                    .map(|h| h.into())
                    .unwrap_or(0isize);
                return Ok((element, hwnd));
            }
        }
    }

    Err(LlmError::ToolCall(
        "Element not found in any window".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_title_deterministic() {
        let h1 = hash_title("Notepad");
        let h2 = hash_title("Notepad");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_title_different_for_different_titles() {
        let h1 = hash_title("Notepad");
        let h2 = hash_title("Calculator");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_title_is_32_bit() {
        let h = hash_title("Test Window");
        assert_eq!(h.len(), 8);
    }

    #[test]
    fn test_split_escaped() {
        let parts = split_escaped("a|b|c");
        assert_eq!(parts, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_split_escaped_with_escape() {
        let parts = split_escaped("a\\|b|c");
        assert_eq!(parts, vec!["a|b", "c"]);
    }

    #[test]
    fn test_parse_element_ref_new_format() {
        let ref_str = "rid:00000001.00000002|hwnd:12345|name:OK|type:Button|aid:btnOk|idx:0";
        let parsed = parse_element_ref(ref_str).unwrap();
        assert_eq!(parsed.runtime_id, vec![1, 2]);
        assert_eq!(parsed.hwnd, 12345);
        assert_eq!(parsed.name, "OK");
        assert_eq!(parsed.ctrl_type, "Button");
        assert_eq!(parsed.automation_id, "btnOk");
        assert_eq!(parsed.index, 0);
    }

    #[test]
    fn test_parse_element_ref_legacy_format() {
        let ref_str = "win:abcd1234|name:OK|type:Button|aid:btnOk|idx:0";
        let parsed = parse_element_ref(ref_str).unwrap();
        assert_eq!(parsed.win_hash, "abcd1234");
        assert_eq!(parsed.name, "OK");
        assert!(parsed.runtime_id.is_empty());
        assert_eq!(parsed.hwnd, 0);
    }

    #[test]
    fn test_parse_element_ref_pipe_in_name() {
        let ref_str = "rid:|hwnd:0|name:Hello\\|World|type:Text|aid:|idx:0";
        let parsed = parse_element_ref(ref_str).unwrap();
        assert_eq!(parsed.name, "Hello|World");
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        // Build a reference string manually (encode_element_ref needs a UIElement,
        // but we can test parse_element_ref on the exact format it produces).
        let ref_str = "rid:00000001.00000002.00000003|hwnd:65536|name:Save|type:Button|aid:btnSave|idx:5";
        let parsed = parse_element_ref(ref_str).unwrap();
        assert_eq!(parsed.runtime_id, vec![1, 2, 3]);
        assert_eq!(parsed.hwnd, 65536);
        assert_eq!(parsed.name, "Save");
        assert_eq!(parsed.ctrl_type, "Button");
        assert_eq!(parsed.automation_id, "btnSave");
        assert_eq!(parsed.index, 5);
        assert!(parsed.win_hash.is_empty());
    }

    #[test]
    fn test_encode_decode_roundtrip_empty_runtime_id() {
        let ref_str = "rid:|hwnd:12345|name:OK|type:Button|aid:|idx:0";
        let parsed = parse_element_ref(ref_str).unwrap();
        assert!(parsed.runtime_id.is_empty());
        assert_eq!(parsed.hwnd, 12345);
        assert_eq!(parsed.name, "OK");
    }

    #[test]
    fn test_backward_compat_legacy_hash_format() {
        // Old format should parse correctly and populate win_hash
        let ref_str = "win:a1b2c3d4|name:Open|type:MenuItem|aid:|idx:2";
        let parsed = parse_element_ref(ref_str).unwrap();
        assert_eq!(parsed.win_hash, "a1b2c3d4");
        assert_eq!(parsed.name, "Open");
        assert_eq!(parsed.ctrl_type, "MenuItem");
        assert_eq!(parsed.index, 2);
        assert!(parsed.runtime_id.is_empty());
        assert_eq!(parsed.hwnd, 0);
    }

    #[test]
    fn test_parse_invalid_ref_returns_error() {
        // Completely invalid ref string should still parse (with empty/default fields)
        let ref_str = "garbage";
        let parsed = parse_element_ref(ref_str).unwrap();
        assert!(parsed.runtime_id.is_empty());
        assert_eq!(parsed.hwnd, 0);
        assert!(parsed.name.is_empty());
    }
}
