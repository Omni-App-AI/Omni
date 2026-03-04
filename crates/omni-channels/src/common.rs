//! Common helpers shared across channel plugins.

use std::sync::atomic::{AtomicU8, Ordering};

use crate::ConnectionStatus;

/// Split a message into chunks that fit within a platform's character limit.
/// Tries to break at newline boundaries when possible.
pub fn chunk_message(text: &str, max_len: usize) -> Vec<String> {
    if text.len() <= max_len {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if remaining.len() <= max_len {
            chunks.push(remaining.to_string());
            break;
        }

        let split_at = remaining[..max_len]
            .rfind('\n')
            .unwrap_or(max_len);

        let (chunk, rest) = remaining.split_at(split_at);
        chunks.push(chunk.to_string());
        remaining = rest.trim_start_matches('\n');
    }

    chunks
}

/// Store a `ConnectionStatus` into an `AtomicU8`.
pub fn set_status(atom: &AtomicU8, status: ConnectionStatus) {
    atom.store(status as u8, Ordering::Relaxed);
}

/// Load a `ConnectionStatus` from an `AtomicU8`.
pub fn get_status(atom: &AtomicU8) -> ConnectionStatus {
    match atom.load(Ordering::Relaxed) {
        0 => ConnectionStatus::Disconnected,
        1 => ConnectionStatus::Connecting,
        2 => ConnectionStatus::Connected,
        3 => ConnectionStatus::Reconnecting,
        _ => ConnectionStatus::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_short() {
        let chunks = chunk_message("Hello", 100);
        assert_eq!(chunks, vec!["Hello"]);
    }

    #[test]
    fn test_chunk_long() {
        let long = "x".repeat(5000);
        let chunks = chunk_message(&long, 2000);
        assert!(chunks.len() >= 3);
        for chunk in &chunks {
            assert!(chunk.len() <= 2000);
        }
    }

    #[test]
    fn test_chunk_newline_boundary() {
        let msg = "Line 1\nLine 2\nLine 3";
        let chunks = chunk_message(msg, 10);
        assert!(chunks.len() > 1);
    }

    #[test]
    fn test_status_roundtrip() {
        let atom = AtomicU8::new(0);
        assert_eq!(get_status(&atom), ConnectionStatus::Disconnected);
        set_status(&atom, ConnectionStatus::Connected);
        assert_eq!(get_status(&atom), ConnectionStatus::Connected);
        set_status(&atom, ConnectionStatus::Error);
        assert_eq!(get_status(&atom), ConnectionStatus::Error);
    }
}
