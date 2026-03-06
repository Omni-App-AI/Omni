#![cfg(windows)]

use crate::error::{LlmError, Result};

/// Translate our "ctrl+a" key combo format to uiautomation's "{ctrl}a" format,
/// then send it using the crate's Keyboard API.
pub fn press_key_combo(keys: &str) -> Result<()> {
    let translated = translate_key_combo(keys)?;

    let keyboard = uiautomation::inputs::Keyboard::new().interval(30);
    keyboard
        .send_keys(&translated)
        .map_err(|e| LlmError::ToolCall(format!("Key combo failed: {e}")))
}

/// Type text into the currently focused element using the crate's Keyboard API.
/// Translates `\n` to `{enter}` and `\t` to `{tab}` so the LLM can request
/// Enter/Tab presses within a text payload.
pub fn type_text(text: &str) -> Result<()> {
    let translated = text.replace('\n', "{enter}").replace('\t', "{tab}");
    let keyboard = uiautomation::inputs::Keyboard::new().interval(10);
    keyboard
        .send_keys(&translated)
        .map_err(|e| LlmError::ToolCall(format!("Keyboard input failed: {e}")))
}

/// Perform a left click at the given screen coordinates using the crate's Mouse API.
pub fn click_at(x: i32, y: i32) -> Result<()> {
    let point = uiautomation::types::Point::new(x, y);
    let mouse = uiautomation::inputs::Mouse::new();
    mouse
        .click(&point)
        .map_err(|e| LlmError::ToolCall(format!("Mouse click failed: {e}")))
}

/// Scroll at the given screen coordinates using SendInput mouse wheel.
/// The Mouse struct in uiautomation doesn't expose wheel scrolling,
/// so we keep a minimal SendInput call for this one case.
pub fn scroll_at(x: i32, y: i32, amount: i32) -> Result<()> {
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

    #[allow(clashing_extern_declarations)]
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
        SetCursorPos(x, y);
        std::thread::sleep(std::time::Duration::from_millis(50));

        let screen_w = GetSystemMetrics(SM_CXSCREEN).max(1);
        let screen_h = GetSystemMetrics(SM_CYSCREEN).max(1);
        let norm_x = ((x as i64) * 65535 / screen_w as i64) as i32;
        let norm_y = ((y as i64) * 65535 / screen_h as i64) as i32;

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

/// Translate our "ctrl+a" format to uiautomation's "{ctrl}a" format.
fn translate_key_combo(combo: &str) -> Result<String> {
    let parts: Vec<&str> = combo.split('+').map(|s| s.trim()).collect();
    let mut result = String::new();

    for part in &parts {
        let lower = part.to_lowercase();
        let mapped = match lower.as_str() {
            "ctrl" | "control" => "{ctrl}",
            "shift" => "{shift}",
            "alt" => "{alt}",
            "win" | "super" | "meta" => "{win}",
            "enter" | "return" => "{enter}",
            "tab" => "{tab}",
            "escape" | "esc" => "{esc}",
            "space" | " " => " ",
            "backspace" | "back" => "{backspace}",
            "delete" | "del" => "{delete}",
            "insert" | "ins" => "{insert}",
            "home" => "{home}",
            "end" => "{end}",
            "pageup" | "pgup" => "{pageup}",
            "pagedown" | "pgdn" => "{pagedown}",
            "up" | "uparrow" => "{up}",
            "down" | "downarrow" => "{down}",
            "left" | "leftarrow" => "{left}",
            "right" | "rightarrow" => "{right}",
            "f1" => "{F1}",
            "f2" => "{F2}",
            "f3" => "{F3}",
            "f4" => "{F4}",
            "f5" => "{F5}",
            "f6" => "{F6}",
            "f7" => "{F7}",
            "f8" => "{F8}",
            "f9" => "{F9}",
            "f10" => "{F10}",
            "f11" => "{F11}",
            "f12" => "{F12}",
            s if s.len() == 1 => {
                let c = s.chars().next().unwrap();
                if c.is_ascii_alphanumeric() {
                    result.push(c);
                    continue;
                } else {
                    return Err(LlmError::ToolCall(format!(
                        "Unsupported key: '{}'. Use named keys or single alphanumeric characters.",
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
        result.push_str(mapped);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_ctrl_a() {
        assert_eq!(translate_key_combo("ctrl+a").unwrap(), "{ctrl}a");
    }

    #[test]
    fn test_translate_ctrl_shift_s() {
        assert_eq!(
            translate_key_combo("ctrl+shift+s").unwrap(),
            "{ctrl}{shift}s"
        );
    }

    #[test]
    fn test_translate_alt_f4() {
        assert_eq!(translate_key_combo("alt+f4").unwrap(), "{alt}{F4}");
    }

    #[test]
    fn test_translate_enter() {
        assert_eq!(translate_key_combo("enter").unwrap(), "{enter}");
    }

    #[test]
    fn test_translate_unknown_key() {
        assert!(translate_key_combo("ctrl+unknown").is_err());
    }

    #[test]
    fn test_translate_single_char() {
        assert_eq!(translate_key_combo("a").unwrap(), "a");
    }

    #[test]
    fn test_translate_escape() {
        assert_eq!(translate_key_combo("esc").unwrap(), "{esc}");
    }
}
