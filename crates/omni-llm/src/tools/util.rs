//! Shared utilities for native tools.

/// Find the largest byte index <= `max` that is a valid char boundary.
/// Used for safe UTF-8 string truncation.
pub fn floor_char_boundary(s: &str, max: usize) -> usize {
    if max >= s.len() {
        return s.len();
    }
    let mut i = max;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_floor_char_boundary_ascii() {
        let s = "hello world";
        assert_eq!(floor_char_boundary(s, 5), 5);
        assert_eq!(floor_char_boundary(s, 100), s.len());
        assert_eq!(floor_char_boundary(s, 0), 0);
    }

    #[test]
    fn test_floor_char_boundary_multibyte() {
        // Each emoji is 4 bytes
        let s = "hello \u{1F600} world"; // "hello 😀 world"
        // "hello " = 6 bytes, then 😀 = 4 bytes (6..10), then " world" = 6 bytes
        assert_eq!(floor_char_boundary(s, 7), 6); // mid-emoji, backs up to 6
        assert_eq!(floor_char_boundary(s, 8), 6);
        assert_eq!(floor_char_boundary(s, 9), 6);
        assert_eq!(floor_char_boundary(s, 10), 10); // exactly at emoji end
    }

    #[test]
    fn test_floor_char_boundary_cjk() {
        // Each CJK char is 3 bytes
        let s = "你好世界"; // 12 bytes total
        assert_eq!(floor_char_boundary(s, 3), 3); // after 你
        assert_eq!(floor_char_boundary(s, 4), 3); // mid-好, backs up
        assert_eq!(floor_char_boundary(s, 5), 3); // mid-好, backs up
        assert_eq!(floor_char_boundary(s, 6), 6); // after 好
    }
}
