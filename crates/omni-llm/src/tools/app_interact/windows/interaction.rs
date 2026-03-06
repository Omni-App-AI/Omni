#![cfg(windows)]

use std::time::Duration;

use uiautomation::patterns::{
    UIExpandCollapsePattern, UIInvokePattern, UILegacyIAccessiblePattern,
    UISelectionItemPattern, UITogglePattern, UIValuePattern, UITextPattern,
};
use uiautomation::types::ExpandCollapseState;
use uiautomation::{UIAutomation, UIElement};

use super::element_ref;
use super::focus;
use super::input;
use crate::error::{LlmError, Result};

/// Default max retries when waiting for an element to become interactive.
pub const DEFAULT_RETRY_COUNT: u32 = 3;

/// Default delay between retry attempts in milliseconds.
pub const DEFAULT_RETRY_DELAY_MS: u64 = 300;

/// Wait for an element to become interactive before performing an action.
/// Retries element lookup and checks is_enabled/is_offscreen.
/// `max_retries` and `retry_delay_ms` control retry behavior.
pub fn wait_for_ready(
    automation: &UIAutomation,
    element_ref: &str,
    max_retries: u32,
    retry_delay_ms: u64,
) -> Result<(UIElement, isize)> {
    let mut last_err = String::new();

    for attempt in 0..=max_retries {
        match element_ref::decode_and_find(automation, element_ref) {
            Ok((element, hwnd)) => {
                // Check if enabled
                if !element.is_enabled().unwrap_or(false) {
                    last_err = "Element is disabled".to_string();
                    if attempt < max_retries {
                        std::thread::sleep(Duration::from_millis(retry_delay_ms));
                        continue;
                    }
                    return Err(LlmError::ToolCall(last_err));
                }

                // If offscreen, try ScrollItemPattern to bring into view
                if element.is_offscreen().unwrap_or(false) {
                    if let Ok(scroll_item) =
                        element.get_pattern::<uiautomation::patterns::UIScrollItemPattern>()
                    {
                        let _ = scroll_item.scroll_into_view();
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }

                return Ok((element, hwnd));
            }
            Err(e) => {
                last_err = e.to_string();
                if attempt < max_retries {
                    std::thread::sleep(Duration::from_millis(retry_delay_ms));
                }
            }
        }
    }

    Err(LlmError::ToolCall(format!(
        "Element not ready after {} retries: {}",
        max_retries, last_err
    )))
}

/// Click an element using a 7-step pattern cascade with foreground management.
pub fn click_element(automation: &UIAutomation, element_ref: &str, retry_count: u32, retry_delay_ms: u64) -> Result<()> {
    let (element, hwnd) = wait_for_ready(automation, element_ref, retry_count, retry_delay_ms)?;

    // Step 1: Foreground the parent window
    focus::bring_to_foreground(hwnd)?;

    // Steps 2-6: Try semantic patterns in order. Each pattern attempt falls through
    // to the next on failure (e.g. ExpandCollapse on an Edit bar may report as available
    // but fail with "Unspecified error"). Only return on success.

    // Step 2: InvokePattern (semantic click — buttons, menu items)
    if let Ok(invoke) = element.get_pattern::<UIInvokePattern>() {
        if invoke.invoke().is_ok() {
            return Ok(());
        }
    }

    // Step 3: TogglePattern (checkboxes, toggle buttons)
    if let Ok(toggle) = element.get_pattern::<UITogglePattern>() {
        if toggle.toggle().is_ok() {
            return Ok(());
        }
    }

    // Step 4: SelectionItemPattern (list items, radio buttons, tabs)
    if let Ok(sel) = element.get_pattern::<UISelectionItemPattern>() {
        if sel.select().is_ok() {
            return Ok(());
        }
    }

    // Step 5: ExpandCollapsePattern (tree items, combo boxes, menus)
    if let Ok(expand) = element.get_pattern::<UIExpandCollapsePattern>() {
        let state = expand.get_state().unwrap_or(ExpandCollapseState::Collapsed);
        let result = if state == ExpandCollapseState::Collapsed {
            expand.expand()
        } else {
            expand.collapse()
        };
        if result.is_ok() {
            return Ok(());
        }
    }

    // Step 6: LegacyIAccessiblePattern (Chromium elements that lack InvokePattern)
    if let Ok(legacy) = element.get_pattern::<UILegacyIAccessiblePattern>() {
        if legacy.do_default_action().is_ok() {
            return Ok(());
        }
    }

    // Step 7: Coordinate-based click fallback using element's clickable point or center
    let (cx, cy) = get_click_point(&element)?;
    input::click_at(cx, cy)
}

/// Type text into an element with foreground management and focus verification.
pub fn type_text_into(
    automation: &UIAutomation,
    element_ref: &str,
    text: &str,
    retry_count: u32,
    retry_delay_ms: u64,
) -> Result<()> {
    let (element, hwnd) = wait_for_ready(automation, element_ref, retry_count, retry_delay_ms)?;

    // Hard block: password fields
    if element.is_password().unwrap_or(false) {
        return Err(LlmError::ToolCall(
            "Cannot type into password fields (security restriction)".to_string(),
        ));
    }

    // Strategy 1: ValuePattern.set_value() (most reliable, no focus needed)
    // Strip trailing \n — set_value() is programmatic and won't trigger navigation.
    // If the text ends with \n, we press Enter afterward via keyboard.
    let trailing_enter = text.ends_with('\n');
    let value_text = if trailing_enter { text.trim_end_matches('\n') } else { text };

    if let Ok(vp) = element.get_pattern::<UIValuePattern>() {
        if !vp.is_readonly().unwrap_or(true) {
            vp.set_value(value_text)
                .map_err(|e| LlmError::ToolCall(format!("Failed to set value: {e}")))?;
            if trailing_enter {
                // Need to focus the element + press Enter for navigation/submit
                focus::bring_to_foreground(hwnd)?;
                std::thread::sleep(Duration::from_millis(50));
                let _ = element.set_focus();
                std::thread::sleep(Duration::from_millis(30));
                input::press_key_combo("enter")?;
            }
            return Ok(());
        }
    }

    // Strategy 2: Focus + keyboard input
    if !element.is_keyboard_focusable().unwrap_or(false) {
        return Err(LlmError::ToolCall(
            "Element does not support text input (no ValuePattern and not keyboard-focusable)"
                .to_string(),
        ));
    }

    // Foreground the window first
    focus::bring_to_foreground(hwnd)?;
    std::thread::sleep(Duration::from_millis(50));

    // Focus the element
    element
        .set_focus()
        .map_err(|e| LlmError::ToolCall(format!("Failed to focus element: {e}")))?;
    std::thread::sleep(Duration::from_millis(50));

    // Verify focus landed on our element (RuntimeId comparison)
    if let Ok(focused) = automation.get_focused_element() {
        let focused_rid = focused.get_runtime_id().unwrap_or_default();
        let target_rid = element.get_runtime_id().unwrap_or_default();
        if !focused_rid.is_empty() && !target_rid.is_empty() && focused_rid != target_rid {
            tracing::warn!("Focus landed on wrong element, retrying set_focus...");
            let _ = element.set_focus();
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    // Type text using the crate's Keyboard API
    input::type_text(text)
}

/// Read text from an element with retry support.
pub fn read_text_from(automation: &UIAutomation, element_ref: &str, retry_count: u32, retry_delay_ms: u64) -> Result<String> {
    let (element, _hwnd) = wait_for_ready(automation, element_ref, retry_count, retry_delay_ms)?;

    // Hard block: password fields
    if element.is_password().unwrap_or(false) {
        return Err(LlmError::ToolCall(
            "Cannot read password fields (security restriction)".to_string(),
        ));
    }

    // Strategy 1: ValuePattern
    if let Ok(vp) = element.get_pattern::<UIValuePattern>() {
        if let Ok(val) = vp.get_value() {
            return Ok(val);
        }
    }

    // Strategy 2: TextPattern for rich text controls
    if let Ok(tp) = element.get_pattern::<UITextPattern>() {
        if let Ok(range) = tp.get_document_range() {
            if let Ok(text) = range.get_text(10_000) {
                return Ok(text);
            }
        }
    }

    // Strategy 3: LegacyIAccessible value (for Chromium elements)
    if let Ok(legacy) = element.get_pattern::<UILegacyIAccessiblePattern>() {
        if let Ok(val) = legacy.get_value() {
            if !val.is_empty() {
                return Ok(val);
            }
        }
    }

    // Strategy 4: Element name fallback
    element
        .get_name()
        .map_err(|e| LlmError::ToolCall(format!("Failed to read element text: {e}")))
}

/// Get the best click point for an element (clickable point or bounding rect center).
fn get_click_point(element: &UIElement) -> Result<(i32, i32)> {
    // Try get_clickable_point first (UIA provides the best point)
    if let Ok(Some(point)) = element.get_clickable_point() {
        if point.get_x() > 0 || point.get_y() > 0 {
            return Ok((point.get_x() as i32, point.get_y() as i32));
        }
    }

    // Fallback: bounding rect center
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

    Ok((center_x, center_y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_retry_constants() {
        assert_eq!(DEFAULT_RETRY_COUNT, 3);
        assert_eq!(DEFAULT_RETRY_DELAY_MS, 300);
    }

    #[test]
    fn test_wait_for_ready_invalid_ref_fails() {
        // wait_for_ready with a garbage element_ref should fail after retries
        let automation = UIAutomation::new();
        if automation.is_err() {
            // COM not available in CI — skip gracefully
            return;
        }
        let automation = automation.unwrap();
        let result = wait_for_ready(&automation, "garbage_ref", 1, 50);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("not ready") || err_msg.contains("not found") || err_msg.contains("invalid"));
    }

    #[test]
    fn test_wait_for_ready_zero_retries_fails_fast() {
        let automation = UIAutomation::new();
        if automation.is_err() {
            return;
        }
        let automation = automation.unwrap();
        let start = std::time::Instant::now();
        // Use a ref with hwnd:0, no win_hash, no name/aid — hits "invalid reference" immediately
        let _ = wait_for_ready(&automation, "rid:|hwnd:0|name:|type:|aid:|idx:0", 0, 300);
        // With 0 retries and instant failure path, should complete very quickly
        assert!(start.elapsed().as_secs() < 5);
    }

    #[test]
    fn test_wait_for_ready_custom_retry_params() {
        let automation = UIAutomation::new();
        if automation.is_err() {
            return;
        }
        let automation = automation.unwrap();
        // Test that custom retry count is honored: 1 retry with 100ms delay.
        // Use an invalid hwnd so it fails fast in find_window_by_hwnd.
        let result = wait_for_ready(&automation, "rid:|hwnd:99999999|name:x|type:Button|aid:|idx:0", 1, 100);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("1 retries") || err_msg.contains("not found") || err_msg.contains("not ready"),
            "Error should reference retry failure, got: {}", err_msg);
    }
}
