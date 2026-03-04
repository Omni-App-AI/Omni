//! Omni Guardian Anti-Injection System
//!
//! A 4-layer scanning pipeline that inspects all data flowing between
//! users, extensions, and the LLM to detect prompt injection attacks.
//!
//! ## Layers
//! 1. **Signature Scanner** -- RegexSet pattern matching with encoding bypass detection
//! 2. **Heuristic Scanner** -- 5 weighted behavioral rules
//! 3. **ML Classifier** -- ONNX-based classifier (feature-gated behind `ml-classifier`)
//! 4. **Output Policy Validator** -- Tool call validation against schemas

pub mod error;
pub mod heuristics;
pub mod ml;
pub mod pipeline;
pub mod policy;
pub mod signatures;
pub mod types;

pub use error::{GuardianError, Result};
pub use heuristics::HeuristicScanner;
pub use ml::MlClassifier;
pub use pipeline::Guardian;
pub use policy::{OutputPolicyValidator, ToolInfo, ToolRegistry};
pub use signatures::{SignatureDatabase, SignatureEntry, SignatureScanner};
pub use types::{
    GuardianMetrics, HeuristicScanResult, LayerResult, MetricsSnapshot, MlClassifyResult,
    PendingBlock, ScanResult, ScanType, Sensitivity, SignatureScanResult, Thresholds, ToolCallInfo,
    ToolCallValidation,
};
