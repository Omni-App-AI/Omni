#![cfg(windows)]

use std::ffi::c_void;

/// DPI information for coordinate translation.
#[derive(Debug, Clone, Copy)]
pub struct DpiInfo {
    pub dpi: u32,
    pub scale_factor: f64,
}

impl Default for DpiInfo {
    fn default() -> Self {
        Self {
            dpi: 96,
            scale_factor: 1.0,
        }
    }
}

/// Get DPI for a specific window (Windows 10 1607+).
pub fn get_dpi_for_window(hwnd: isize) -> DpiInfo {
    type HWND = *mut c_void;

    extern "system" {
        fn GetDpiForWindow(hwnd: HWND) -> u32;
    }

    unsafe {
        let dpi = GetDpiForWindow(hwnd as HWND);
        let dpi = if dpi == 0 { 96 } else { dpi };
        DpiInfo {
            dpi,
            scale_factor: dpi as f64 / 96.0,
        }
    }
}

/// Set the process to per-monitor DPI aware V2.
/// Call once at startup so UIA returns physical pixel coordinates
/// matching what SendInput expects.
pub fn set_dpi_awareness() {
    extern "system" {
        fn SetProcessDpiAwarenessContext(context: isize) -> i32;
    }

    const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;

    unsafe {
        // Returns 0 on failure (e.g., already set). That's fine.
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

/// Convert logical coordinates to physical coordinates using DPI info.
/// Only needed if the process is NOT per-monitor DPI aware.
pub fn logical_to_physical(x: i32, y: i32, dpi: &DpiInfo) -> (i32, i32) {
    let px = (x as f64 * dpi.scale_factor) as i32;
    let py = (y as f64 * dpi.scale_factor) as i32;
    (px, py)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_dpi_info() {
        let dpi = DpiInfo::default();
        assert_eq!(dpi.dpi, 96);
        assert!((dpi.scale_factor - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_logical_to_physical_no_scaling() {
        let dpi = DpiInfo {
            dpi: 96,
            scale_factor: 1.0,
        };
        assert_eq!(logical_to_physical(100, 200, &dpi), (100, 200));
    }

    #[test]
    fn test_logical_to_physical_150_percent() {
        let dpi = DpiInfo {
            dpi: 144,
            scale_factor: 1.5,
        };
        assert_eq!(logical_to_physical(100, 200, &dpi), (150, 300));
    }

    #[test]
    fn test_set_dpi_awareness_does_not_crash() {
        // Just ensure it doesn't panic -- may fail silently if already set
        set_dpi_awareness();
    }
}
