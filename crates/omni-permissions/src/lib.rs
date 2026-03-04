//! Omni Permission Gating System
//!
//! Capability-based permission system that all extension interactions pass through.
//! Provides deny-by-default security with granular scope validation and full audit logging.

pub mod audit;
pub mod capability;
pub mod policy;
pub mod scope;

use std::sync::{Arc, Mutex};

use omni_core::config::PermissionDefaults;
use omni_core::database::{AuditRecord, Database};
use omni_core::error::Result;
use omni_core::events::EventBus;

use crate::audit::{AuditLogger, AuditQuery, ExportFormat};
use crate::capability::{Capability, CapabilityRequest};
use crate::policy::{
    DefaultPolicy, PermissionDecision, PermissionDuration, PolicyEngine, StoredDecision,
};
use crate::scope::validate_scope;

/// High-level facade combining PolicyEngine, AuditLogger, and Scope Validation.
pub struct PermissionManager {
    policy_engine: PolicyEngine,
    audit_logger: AuditLogger,
}

impl PermissionManager {
    pub fn new(
        db: Arc<Mutex<Database>>,
        event_bus: EventBus,
        config: &PermissionDefaults,
    ) -> Self {
        let policy_engine = PolicyEngine::from_config(db.clone(), config);
        let audit_logger = AuditLogger::new(db, event_bus);
        Self {
            policy_engine,
            audit_logger,
        }
    }

    pub fn with_default_policy(
        db: Arc<Mutex<Database>>,
        event_bus: EventBus,
        default_policy: DefaultPolicy,
    ) -> Self {
        let policy_engine = PolicyEngine::new(db.clone(), default_policy);
        let audit_logger = AuditLogger::new(db, event_bus);
        Self {
            policy_engine,
            audit_logger,
        }
    }

    /// Main entry point: check permission with scope validation and audit logging.
    ///
    /// 1. Check policy (session cache → database → default)
    /// 2. If allowed, validate scope constraints
    /// 3. Log the decision to the audit trail
    pub async fn check_permission(
        &self,
        extension_id: &str,
        declared_capability: &Capability,
        request: &CapabilityRequest,
        session_id: Option<&str>,
    ) -> PermissionDecision {
        // 1. Policy check
        let decision = self
            .policy_engine
            .check(extension_id, declared_capability)
            .await;

        // 2. If allowed, validate scope
        if matches!(decision, PermissionDecision::Allow) {
            if let Err(violation) = validate_scope(declared_capability, request) {
                let deny = PermissionDecision::Deny {
                    reason: format!("Scope violation: {violation}"),
                };
                self.audit_logger
                    .log_check(extension_id, declared_capability, &deny, session_id)
                    .await;
                return deny;
            }
        }

        // 3. Audit log
        self.audit_logger
            .log_check(extension_id, declared_capability, &decision, session_id)
            .await;

        decision
    }

    /// Record a user's permission decision.
    pub async fn record_decision(
        &self,
        extension_id: &str,
        capability: &Capability,
        decision: StoredDecision,
        duration: PermissionDuration,
    ) -> Result<()> {
        self.policy_engine
            .record_decision(extension_id, capability, decision, duration)
            .await
    }

    /// Revoke all permissions for a specific extension.
    pub async fn revoke_all(&self, extension_id: &str) -> Result<u64> {
        self.policy_engine.revoke_all(extension_id).await
    }

    /// Emergency kill switch: revoke all permissions for all extensions.
    pub async fn revoke_everything(&self) -> Result<u64> {
        self.policy_engine.revoke_everything().await
    }

    /// Query the audit log with filters.
    pub async fn query_audit(&self, query: AuditQuery) -> Result<Vec<AuditRecord>> {
        self.audit_logger.query(query).await
    }

    /// Export the audit log in the specified format.
    pub async fn export_audit(
        &self,
        format: ExportFormat,
        time_range: Option<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>,
    ) -> Result<Vec<u8>> {
        self.audit_logger.export(format, time_range).await
    }
}
