#![cfg(windows)]

use uiautomation::patterns::UIValuePattern;
use uiautomation::types::ControlType;
use uiautomation::UITreeWalker;
use uiautomation::UIElement;

use super::super::types::*;
use super::element_ref;
use super::helpers;

/// Maximum elements returned in a tree walk (prevents LLM context overflow).
pub const MAX_TREE_ELEMENTS: u32 = 500;

/// Maximum depth for get_tree / get_subtree (hard cap).
pub const MAX_TREE_DEPTH: u32 = 8;

/// Walk the UI tree recursively with depth and element count limits.
pub fn walk_tree_recursive(
    walker: &UITreeWalker,
    element: &UIElement,
    window_hwnd: isize,
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
    let ct_name = helpers::control_type_name(ct).to_string();
    let aid = element.get_automation_id().unwrap_or_default();
    let is_enabled = element.is_enabled().unwrap_or(false);
    let is_pwd = element.is_password().unwrap_or(false);

    let value = if is_pwd {
        Some("[REDACTED]".to_string())
    } else {
        element
            .get_pattern::<UIValuePattern>()
            .ok()
            .and_then(|vp| vp.get_value().ok())
    };

    let element_ref = if compact {
        String::new()
    } else {
        element_ref::encode_element_ref(element, window_hwnd, &name, &ct_name, &aid, *count)
    };
    *count += 1;

    let children = if current_depth < max_depth {
        let mut kids = Vec::new();
        if let Some(child_elements) = walker.get_children(element) {
            for child in &child_elements {
                if *count >= max_elements {
                    break;
                }
                if let Some(node) = walk_tree_recursive(
                    walker,
                    child,
                    window_hwnd,
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
pub fn build_tree_result(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_depth_constants() {
        assert_eq!(MAX_TREE_DEPTH, 8);
        assert!(super::super::element_ref::FIND_ELEMENT_DEPTH >= MAX_TREE_DEPTH);
    }

    #[test]
    fn test_element_cap_constant() {
        assert_eq!(MAX_TREE_ELEMENTS, 500);
    }

    #[test]
    fn test_build_tree_result_no_truncation() {
        let root = UiTreeNode {
            name: "Root".to_string(),
            control_type: "Window".to_string(),
            automation_id: String::new(),
            element_ref: String::new(),
            is_enabled: true,
            value: None,
            children: vec![],
        };
        let tree = build_tree_result("Test".to_string(), "test.exe".to_string(), root, 5, 2, 4);
        assert!(!tree.truncated);
        assert!(tree.truncation_reason.is_none());
        assert_eq!(tree.total_elements, 5);
        assert_eq!(tree.depth_reached, 2);
    }

    #[test]
    fn test_build_tree_result_element_cap_truncation() {
        let root = UiTreeNode {
            name: "Root".to_string(),
            control_type: "Window".to_string(),
            automation_id: String::new(),
            element_ref: String::new(),
            is_enabled: true,
            value: None,
            children: vec![],
        };
        let tree = build_tree_result("Test".to_string(), "test.exe".to_string(), root, MAX_TREE_ELEMENTS, 4, 4);
        assert!(tree.truncated);
        let reason = tree.truncation_reason.unwrap();
        assert!(reason.contains("element cap"), "Expected element cap message, got: {}", reason);
    }

    #[test]
    fn test_build_tree_result_depth_limit_truncation() {
        let root = UiTreeNode {
            name: "Root".to_string(),
            control_type: "Window".to_string(),
            automation_id: String::new(),
            element_ref: String::new(),
            is_enabled: true,
            value: None,
            children: vec![],
        };
        // depth_reached == requested_depth but element count is below cap
        let tree = build_tree_result("Test".to_string(), "test.exe".to_string(), root, 50, 4, 4);
        assert!(!tree.truncated); // truncated is only true for element cap
        let reason = tree.truncation_reason.unwrap();
        assert!(reason.contains("depth limit"), "Expected depth limit message, got: {}", reason);
    }
}
