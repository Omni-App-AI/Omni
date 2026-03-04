//! Omni Platform Abstraction Layer
//!
//! Cross-platform abstraction layer (currently a stub).

pub struct PlatformAbstraction;

impl PlatformAbstraction {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PlatformAbstraction {
    fn default() -> Self {
        Self::new()
    }
}
