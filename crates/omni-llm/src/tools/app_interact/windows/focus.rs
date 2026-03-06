#![cfg(windows)]

use std::ffi::c_void;
use std::time::Duration;

use crate::error::Result;

type HWND = *mut c_void;

extern "system" {
    fn SetForegroundWindow(hwnd: HWND) -> i32;
    fn ShowWindow(hwnd: HWND, cmd: i32) -> i32;
    fn IsIconic(hwnd: HWND) -> i32;
    fn GetForegroundWindow() -> HWND;
    fn BringWindowToTop(hwnd: HWND) -> i32;
    fn GetCurrentThreadId() -> u32;
    fn GetWindowThreadProcessId(hwnd: HWND, process_id: *mut u32) -> u32;
    fn AttachThreadInput(attach: u32, attach_to: u32, f_attach: i32) -> i32;
}

const SW_RESTORE: i32 = 9;

/// Bring a window to the foreground using a cascade of strategies.
///
/// Strategy order:
/// 1. ShowWindow(SW_RESTORE) if minimized
/// 2. Direct SetForegroundWindow (works if we already own foreground)
/// 3. AttachThreadInput trick (attach to fg thread, set foreground, detach)
/// 4. ALT keypress trick (send ALT to unlock SetForegroundWindow — most reliable)
/// 5. BringWindowToTop as final fallback
pub fn bring_to_foreground(hwnd: isize) -> Result<()> {
    eprintln!("[app_interact] bring_to_foreground: hwnd=0x{:x}", hwnd);
    unsafe {
        let hwnd_ptr = hwnd as HWND;

        // Step 1: Restore if minimized
        let is_minimized = IsIconic(hwnd_ptr) != 0;
        eprintln!("[app_interact] bring_to_foreground: step 1 — minimized={}", is_minimized);
        if is_minimized {
            ShowWindow(hwnd_ptr, SW_RESTORE);
            std::thread::sleep(Duration::from_millis(150));
        }

        // Step 2: Direct SetForegroundWindow
        let set_fg_result = SetForegroundWindow(hwnd_ptr);
        eprintln!("[app_interact] bring_to_foreground: step 2 — SetForegroundWindow={}", set_fg_result);
        if set_fg_result != 0 {
            std::thread::sleep(Duration::from_millis(50));
            if GetForegroundWindow() == hwnd_ptr {
                eprintln!("[app_interact] bring_to_foreground: SUCCESS at step 2");
                return Ok(());
            }
        }

        // Step 3: AttachThreadInput technique
        let our_thread = GetCurrentThreadId();
        let fg_window = GetForegroundWindow();
        let fg_thread = GetWindowThreadProcessId(fg_window, std::ptr::null_mut());
        eprintln!("[app_interact] bring_to_foreground: step 3 — our_thread={} fg_thread={}", our_thread, fg_thread);

        if fg_thread != 0 && fg_thread != our_thread {
            AttachThreadInput(our_thread, fg_thread, 1); // attach
            BringWindowToTop(hwnd_ptr);
            SetForegroundWindow(hwnd_ptr);
            AttachThreadInput(our_thread, fg_thread, 0); // detach
            std::thread::sleep(Duration::from_millis(50));

            if GetForegroundWindow() == hwnd_ptr {
                eprintln!("[app_interact] bring_to_foreground: SUCCESS at step 3");
                return Ok(());
            }
        }

        // Step 4: ALT keypress trick
        eprintln!("[app_interact] bring_to_foreground: step 4 — ALT key trick");
        send_alt_key();
        std::thread::sleep(Duration::from_millis(30));

        if SetForegroundWindow(hwnd_ptr) != 0 {
            std::thread::sleep(Duration::from_millis(50));
            if GetForegroundWindow() == hwnd_ptr {
                eprintln!("[app_interact] bring_to_foreground: SUCCESS at step 4");
                return Ok(());
            }
        }

        // Step 5: BringWindowToTop as final fallback
        eprintln!("[app_interact] bring_to_foreground: step 5 — final fallback");
        BringWindowToTop(hwnd_ptr);
        ShowWindow(hwnd_ptr, SW_RESTORE);
        SetForegroundWindow(hwnd_ptr);
        std::thread::sleep(Duration::from_millis(50));

        // We've tried everything. Even if GetForegroundWindow != hwnd_ptr,
        // the window should be visible/flashing. Don't fail the operation.
        let final_fg = GetForegroundWindow();
        if final_fg != hwnd_ptr {
            eprintln!("[app_interact] bring_to_foreground: WARNING — not foreground after all 5 steps. fg=0x{:x} target=0x{:x}", final_fg as isize, hwnd);
            tracing::warn!(
                "Window 0x{:x} brought to top but may not have full foreground focus",
                hwnd
            );
        } else {
            eprintln!("[app_interact] bring_to_foreground: SUCCESS at step 5");
        }

        Ok(())
    }
}

/// Send a single ALT key press+release to unlock SetForegroundWindow restrictions.
fn send_alt_key() {
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
        _padding: [u8; 8],
    }

    extern "system" {
        fn SendInput(count: u32, inputs: *const Input, size: i32) -> u32;
    }

    const INPUT_KEYBOARD: u32 = 1;
    const KEYEVENTF_KEYUP: u32 = 0x0002;
    const VK_MENU: u16 = 0x12; // ALT key

    unsafe {
        let inputs = [
            // ALT down
            Input {
                input_type: INPUT_KEYBOARD,
                ki: KeyboardInput {
                    w_vk: VK_MENU,
                    w_scan: 0,
                    dw_flags: 0,
                    time: 0,
                    dw_extra_info: 0,
                },
                _padding: [0u8; 8],
            },
            // ALT up
            Input {
                input_type: INPUT_KEYBOARD,
                ki: KeyboardInput {
                    w_vk: VK_MENU,
                    w_scan: 0,
                    dw_flags: KEYEVENTF_KEYUP,
                    time: 0,
                    dw_extra_info: 0,
                },
                _padding: [0u8; 8],
            },
        ];

        SendInput(2, inputs.as_ptr(), std::mem::size_of::<Input>() as i32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bring_to_foreground_invalid_hwnd_does_not_panic() {
        // Invalid HWND should not crash, just warn/succeed
        let result = bring_to_foreground(0);
        // Should not panic regardless of outcome
        let _ = result;
    }
}
