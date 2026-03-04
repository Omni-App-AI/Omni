use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Result of the full Guardian scanning pipeline.
#[derive(Debug, Clone, Serialize)]
pub struct ScanResult {
    pub blocked: bool,
    pub scan_id: Option<String>,
    pub layer: Option<String>,
    pub reason: Option<String>,
    pub confidence: f64,
    pub layer_results: Vec<LayerResult>,
    pub scan_duration: Duration,
}

impl ScanResult {
    pub fn pass(layer_results: Vec<LayerResult>, scan_duration: Duration) -> Self {
        Self {
            blocked: false,
            scan_id: None,
            layer: None,
            reason: None,
            confidence: 0.0,
            layer_results,
            scan_duration,
        }
    }

    pub fn block(
        scan_id: String,
        layer: &str,
        reason: &str,
        confidence: f64,
        layer_results: Vec<LayerResult>,
        scan_duration: Duration,
    ) -> Self {
        Self {
            blocked: true,
            scan_id: Some(scan_id),
            layer: Some(layer.to_string()),
            reason: Some(reason.to_string()),
            confidence,
            layer_results,
            scan_duration,
        }
    }
}

/// A pending Guardian block that can be overridden by the user.
#[derive(Debug, Clone, Serialize)]
pub struct PendingBlock {
    pub scan_id: String,
    pub scan_type: String,
    pub layer: String,
    pub reason: String,
    pub confidence: f64,
    pub content_preview: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Result of an individual scanning layer.
#[derive(Debug, Clone, Serialize)]
pub struct LayerResult {
    pub layer_name: String,
    pub passed: bool,
    pub score: f64,
    pub details: Option<String>,
    pub duration: Duration,
}

/// Type of content being scanned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanType {
    Input,
    PromptAssembly,
    OutputChunk,
    ToolParameters,
    ExtensionOutput,
}

impl ScanType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScanType::Input => "input",
            ScanType::PromptAssembly => "prompt_assembly",
            ScanType::OutputChunk => "output_chunk",
            ScanType::ToolParameters => "tool_parameters",
            ScanType::ExtensionOutput => "extension_output",
        }
    }
}

/// Guardian sensitivity preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sensitivity {
    Strict,
    Balanced,
    Permissive,
}

impl Sensitivity {
    pub fn thresholds(&self) -> Thresholds {
        match self {
            Sensitivity::Strict => Thresholds {
                signature: 0.50,
                heuristic: 0.40,
                ml: 0.70,
            },
            Sensitivity::Balanced => Thresholds {
                signature: 0.70,
                heuristic: 0.60,
                ml: 0.85,
            },
            Sensitivity::Permissive => Thresholds {
                signature: 0.90,
                heuristic: 0.80,
                ml: 0.95,
            },
        }
    }

    pub fn from_str_config(s: &str) -> Option<Self> {
        match s {
            "strict" => Some(Self::Strict),
            "balanced" => Some(Self::Balanced),
            "permissive" => Some(Self::Permissive),
            _ => None,
        }
    }
}

/// Per-layer blocking thresholds.
#[derive(Debug, Clone, Copy)]
pub struct Thresholds {
    pub signature: f64,
    pub heuristic: f64,
    pub ml: f64,
}

/// Atomic metrics for Guardian scanning performance.
pub struct GuardianMetrics {
    pub scan_count: AtomicU64,
    pub block_count: AtomicU64,
    pub signature_blocks: AtomicU64,
    pub heuristic_blocks: AtomicU64,
    pub ml_blocks: AtomicU64,
    pub policy_blocks: AtomicU64,
    pub total_scan_us: AtomicU64,
}

impl GuardianMetrics {
    pub fn new() -> Self {
        Self {
            scan_count: AtomicU64::new(0),
            block_count: AtomicU64::new(0),
            signature_blocks: AtomicU64::new(0),
            heuristic_blocks: AtomicU64::new(0),
            ml_blocks: AtomicU64::new(0),
            policy_blocks: AtomicU64::new(0),
            total_scan_us: AtomicU64::new(0),
        }
    }

    pub fn record_scan(&self, duration: Duration, blocked: bool, layer: Option<&str>) {
        self.scan_count.fetch_add(1, Ordering::Relaxed);
        self.total_scan_us
            .fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
        if blocked {
            self.block_count.fetch_add(1, Ordering::Relaxed);
            match layer {
                Some("signature") => {
                    self.signature_blocks.fetch_add(1, Ordering::Relaxed);
                }
                Some("heuristic") => {
                    self.heuristic_blocks.fetch_add(1, Ordering::Relaxed);
                }
                Some("ml") => {
                    self.ml_blocks.fetch_add(1, Ordering::Relaxed);
                }
                Some("output_policy") => {
                    self.policy_blocks.fetch_add(1, Ordering::Relaxed);
                }
                _ => {}
            }
        }
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            scan_count: self.scan_count.load(Ordering::Relaxed),
            block_count: self.block_count.load(Ordering::Relaxed),
            signature_blocks: self.signature_blocks.load(Ordering::Relaxed),
            heuristic_blocks: self.heuristic_blocks.load(Ordering::Relaxed),
            ml_blocks: self.ml_blocks.load(Ordering::Relaxed),
            policy_blocks: self.policy_blocks.load(Ordering::Relaxed),
            total_scan_us: self.total_scan_us.load(Ordering::Relaxed),
        }
    }
}

impl Default for GuardianMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    pub scan_count: u64,
    pub block_count: u64,
    pub signature_blocks: u64,
    pub heuristic_blocks: u64,
    pub ml_blocks: u64,
    pub policy_blocks: u64,
    pub total_scan_us: u64,
}

/// Result from the signature scanning layer.
#[derive(Debug, Clone)]
pub struct SignatureScanResult {
    pub matched: bool,
    pub score: f64,
    pub matched_id: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
}

/// Result from the heuristic scanning layer.
#[derive(Debug, Clone)]
pub struct HeuristicScanResult {
    pub score: f64,
    pub rule_scores: Vec<(String, f64)>,
}

/// Result from the ML classifier layer.
#[derive(Debug, Clone)]
pub struct MlClassifyResult {
    pub injection_probability: f64,
    pub benign_probability: f64,
}

/// Validation result for a tool call.
#[derive(Debug, Clone)]
pub enum ToolCallValidation {
    Allowed,
    Blocked { reason: String },
}

/// Info about a tool call to validate.
#[derive(Debug, Clone)]
pub struct ToolCallInfo {
    pub name: String,
    pub arguments: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensitivity_thresholds() {
        let strict = Sensitivity::Strict.thresholds();
        assert!((strict.signature - 0.50).abs() < 0.001);
        assert!((strict.heuristic - 0.40).abs() < 0.001);
        assert!((strict.ml - 0.70).abs() < 0.001);

        let balanced = Sensitivity::Balanced.thresholds();
        assert!((balanced.signature - 0.70).abs() < 0.001);

        let permissive = Sensitivity::Permissive.thresholds();
        assert!((permissive.signature - 0.90).abs() < 0.001);
    }

    #[test]
    fn test_sensitivity_from_str() {
        assert_eq!(Sensitivity::from_str_config("strict"), Some(Sensitivity::Strict));
        assert_eq!(Sensitivity::from_str_config("balanced"), Some(Sensitivity::Balanced));
        assert_eq!(Sensitivity::from_str_config("permissive"), Some(Sensitivity::Permissive));
        assert_eq!(Sensitivity::from_str_config("unknown"), None);
    }

    #[test]
    fn test_scan_result_pass() {
        let result = ScanResult::pass(vec![], Duration::from_millis(5));
        assert!(!result.blocked);
        assert!(result.layer.is_none());
    }

    #[test]
    fn test_scan_result_block() {
        let result = ScanResult::block("scan-123".to_string(), "signature", "Matched SIG-001", 0.95, vec![], Duration::from_millis(1));
        assert!(result.blocked);
        assert_eq!(result.scan_id.as_deref(), Some("scan-123"));
        assert_eq!(result.layer.as_deref(), Some("signature"));
        assert!((result.confidence - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_metrics() {
        let metrics = GuardianMetrics::new();
        metrics.record_scan(Duration::from_millis(5), false, None);
        metrics.record_scan(Duration::from_millis(3), true, Some("signature"));
        metrics.record_scan(Duration::from_millis(4), true, Some("heuristic"));

        let snap = metrics.snapshot();
        assert_eq!(snap.scan_count, 3);
        assert_eq!(snap.block_count, 2);
        assert_eq!(snap.signature_blocks, 1);
        assert_eq!(snap.heuristic_blocks, 1);
    }

    #[test]
    fn test_scan_type_as_str() {
        assert_eq!(ScanType::Input.as_str(), "input");
        assert_eq!(ScanType::ToolParameters.as_str(), "tool_parameters");
        assert_eq!(ScanType::ExtensionOutput.as_str(), "extension_output");
    }
}
