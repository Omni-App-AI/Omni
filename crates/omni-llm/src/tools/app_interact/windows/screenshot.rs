#![cfg(windows)]

use std::ffi::c_void;

use crate::error::{LlmError, Result};
use super::super::types::ScreenshotResult;
use super::helpers;

type HDC = *mut c_void;
type HBITMAP = *mut c_void;
type HGDIOBJ = *mut c_void;
type HWND = *mut c_void;

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

extern "system" {
    fn GetWindowRect(hwnd: HWND, rect: *mut Rect) -> i32;
    fn GetDC(hwnd: HWND) -> HDC;
    fn CreateCompatibleDC(hdc: HDC) -> HDC;
    fn CreateCompatibleBitmap(hdc: HDC, width: i32, height: i32) -> HBITMAP;
    fn SelectObject(hdc: HDC, obj: HGDIOBJ) -> HGDIOBJ;
    fn BitBlt(
        dest: HDC, x: i32, y: i32, w: i32, h: i32,
        src: HDC, sx: i32, sy: i32, rop: u32,
    ) -> i32;
    fn PrintWindow(hwnd: HWND, hdc: HDC, flags: u32) -> i32;
    fn GetDIBits(
        hdc: HDC, bmp: HBITMAP, start: u32, lines: u32,
        bits: *mut u8, info: *mut BitmapInfo, usage: u32,
    ) -> i32;
    fn DeleteObject(obj: HGDIOBJ) -> i32;
    fn DeleteDC(hdc: HDC) -> i32;
    fn ReleaseDC(hwnd: HWND, hdc: HDC) -> i32;
}

const SRCCOPY: u32 = 0x00CC0020;
const PW_CLIENTONLY: u32 = 0x01;
const PW_RENDERFULLCONTENT: u32 = 0x02;
const BI_RGB: u32 = 0;
const DIB_RGB_COLORS: u32 = 0;

/// Capture a window screenshot as a base64-encoded PNG.
pub fn capture_screenshot(
    automation: &uiautomation::UIAutomation,
    window_title: Option<&str>,
    process_name: Option<&str>,
) -> Result<ScreenshotResult> {
    let window = helpers::find_window(automation, window_title, process_name)?;
    let win_title = window.get_name().unwrap_or_default();
    let handle = window
        .get_native_window_handle()
        .map_err(|e| LlmError::ToolCall(format!("Failed to get window handle: {e}")))?;
    let hwnd: isize = handle.into();

    let (width, height, rgba_pixels) = capture_window_pixels(hwnd)?;
    let png_bytes = encode_png(width, height, &rgba_pixels)?;
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png_bytes);

    Ok(ScreenshotResult {
        image_base64: b64,
        mime_type: "image/png".to_string(),
        width,
        height,
        window_title: win_title,
    })
}

/// Capture window pixels using a 3-step GDI cascade:
/// 1. PrintWindow with PW_RENDERFULLCONTENT (best for DWM-composed windows)
/// 2. PrintWindow with PW_CLIENTONLY (classic apps fallback)
/// 3. BitBlt (visible windows only)
pub(crate) fn capture_window_pixels(hwnd: isize) -> Result<(u32, u32, Vec<u8>)> {
    unsafe {
        let hwnd_ptr = hwnd as HWND;

        let mut rect = Rect {
            left: 0, top: 0, right: 0, bottom: 0,
        };
        if GetWindowRect(hwnd_ptr, &mut rect) == 0 {
            return Err(LlmError::ToolCall("Failed to get window rect".to_string()));
        }

        let width = (rect.right - rect.left).max(1) as u32;
        let height = (rect.bottom - rect.top).max(1) as u32;

        if width > 3840 || height > 2160 {
            return Err(LlmError::ToolCall(format!(
                "Window too large for screenshot: {}x{} (max 3840x2160)",
                width, height
            )));
        }

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

        // 3-step capture cascade
        let mut captured = PrintWindow(hwnd_ptr, mem_dc, PW_RENDERFULLCONTENT);
        if captured == 0 {
            captured = PrintWindow(hwnd_ptr, mem_dc, PW_CLIENTONLY);
            if captured == 0 {
                BitBlt(
                    mem_dc, 0, 0, width as i32, height as i32,
                    screen_dc, 0, 0, SRCCOPY,
                );
            }
        }

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
            mem_dc, bitmap, 0, height,
            pixels.as_mut_ptr(), &mut bmi, DIB_RGB_COLORS,
        );

        SelectObject(mem_dc, old_bitmap);
        DeleteObject(bitmap);
        DeleteDC(mem_dc);
        ReleaseDC(hwnd_ptr, screen_dc);

        if lines == 0 {
            return Err(LlmError::ToolCall(
                "Failed to get bitmap pixels".to_string(),
            ));
        }

        // Convert BGRA → RGBA
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }

        Ok((width, height, pixels))
    }
}

/// Encode raw RGBA pixels as a PNG.
pub(crate) fn encode_png(width: u32, height: u32, rgba_pixels: &[u8]) -> Result<Vec<u8>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_png_valid_pixels() {
        // 2x2 red RGBA image
        let pixels: Vec<u8> = vec![
            255, 0, 0, 255,  255, 0, 0, 255,
            255, 0, 0, 255,  255, 0, 0, 255,
        ];
        let png = encode_png(2, 2, &pixels).unwrap();
        // Verify PNG magic bytes
        assert!(png.len() > 8);
        assert_eq!(&png[0..4], &[0x89, 0x50, 0x4E, 0x47]); // PNG signature
    }

    #[test]
    fn test_encode_png_1x1() {
        let pixels = vec![0u8, 128, 255, 255]; // 1 pixel RGBA
        let png = encode_png(1, 1, &pixels).unwrap();
        assert_eq!(&png[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn test_screenshot_constants() {
        assert_eq!(PW_RENDERFULLCONTENT, 0x02);
        assert_eq!(PW_CLIENTONLY, 0x01);
        assert_eq!(SRCCOPY, 0x00CC0020);
    }

    #[test]
    fn test_bgra_to_rgba_conversion() {
        // Simulate the BGRA→RGBA swap that capture_window_pixels does
        let mut pixels: Vec<u8> = vec![
            0, 128, 255, 255,   // BGRA: B=0, G=128, R=255, A=255
            50, 100, 200, 128,  // BGRA: B=50, G=100, R=200, A=128
        ];
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }
        // After swap: RGBA
        assert_eq!(pixels[0], 255); // R
        assert_eq!(pixels[1], 128); // G
        assert_eq!(pixels[2], 0);   // B
        assert_eq!(pixels[3], 255); // A

        assert_eq!(pixels[4], 200); // R
        assert_eq!(pixels[5], 100); // G
        assert_eq!(pixels[6], 50);  // B
        assert_eq!(pixels[7], 128); // A
    }
}
