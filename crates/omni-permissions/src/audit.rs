use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use omni_core::database::{AuditEntry, AuditRecord, Database};
use omni_core::error::{OmniError, Result};
use omni_core::events::{EventBus, OmniEvent};

use crate::capability::Capability;
use crate::policy::PermissionDecision;

pub struct AuditLogger {
    db: Arc<Mutex<Database>>,
    event_bus: EventBus,
}

#[derive(Debug, Default)]
pub struct AuditQuery {
    pub extension_id: Option<String>,
    pub capability: Option<String>,
    pub decision: Option<String>,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    Json,
    Csv,
}

impl AuditLogger {
    pub fn new(db: Arc<Mutex<Database>>, event_bus: EventBus) -> Self {
        Self { db, event_bus }
    }

    pub async fn log_check(
        &self,
        extension_id: &str,
        capability: &Capability,
        decision: &PermissionDecision,
        session_id: Option<&str>,
    ) {
        let event_type = match decision {
            PermissionDecision::Allow => "permission.allowed",
            PermissionDecision::Deny { .. } => "permission.denied",
            PermissionDecision::Prompt { .. } => "permission.prompted",
        };

        let decision_str = match decision {
            PermissionDecision::Allow => "allow".to_string(),
            PermissionDecision::Deny { reason } => format!("deny: {reason}"),
            PermissionDecision::Prompt { reason, .. } => format!("prompt: {reason}"),
        };

        let details = serde_json::json!({
            "capability": capability.to_string(),
            "decision": decision_str,
            "timestamp": Utc::now().to_rfc3339(),
        });

        let entry = AuditEntry {
            event_type: event_type.to_string(),
            extension_id: Some(extension_id.to_string()),
            capability: Some(capability.to_string()),
            decision: Some(decision_str),
            details: Some(details.to_string()),
            session_id: session_id.map(|s| s.to_string()),
        };

        let db = self.db.clone();
        let _ = tokio::task::spawn_blocking(move || {
            let db = db.lock().unwrap();
            db.log_audit_event(&entry)
        })
        .await;

        self.event_bus.emit(OmniEvent::PermissionChecked {
            extension_id: extension_id.to_string(),
            capability: capability.to_string(),
            decision: event_type.to_string(),
        });
    }

    pub async fn query(&self, filters: AuditQuery) -> Result<Vec<AuditRecord>> {
        let db = self.db.clone();
        let limit = filters.limit.unwrap_or(100);
        let ext_id = filters.extension_id;
        let cap = filters.capability;
        let dec = filters.decision;
        let start = filters
            .time_range
            .as_ref()
            .map(|(s, _)| s.to_rfc3339());
        let end = filters
            .time_range
            .as_ref()
            .map(|(_, e)| e.to_rfc3339());

        tokio::task::spawn_blocking(move || {
            let db = db.lock().unwrap();
            db.query_audit_log_filtered(
                ext_id.as_deref(),
                cap.as_deref(),
                dec.as_deref(),
                start.as_deref(),
                end.as_deref(),
                limit,
            )
        })
        .await
        .unwrap_or(Err(OmniError::Other("spawn_blocking failed".to_string())))
    }

    pub async fn export(
        &self,
        format: ExportFormat,
        time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    ) -> Result<Vec<u8>> {
        let entries = self
            .query(AuditQuery {
                time_range,
                ..Default::default()
            })
            .await?;

        match format {
            ExportFormat::Json => serde_json::to_vec_pretty(&entries)
                .map_err(|e| OmniError::Other(e.to_string())),
            ExportFormat::Csv => {
                let mut wtr = csv::Writer::from_writer(vec![]);
                for entry in &entries {
                    wtr.serialize(entry)
                        .map_err(|e| OmniError::Csv(e.to_string()))?;
                }
                wtr.into_inner()
                    .map_err(|e| OmniError::Csv(e.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::Capability;
    use crate::policy::PermissionDecision;
    use tempfile::tempdir;

    fn test_logger() -> AuditLogger {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path, "test-key").unwrap();
        let db = Arc::new(Mutex::new(db));
        let bus = EventBus::new(64);
        std::mem::forget(dir);
        AuditLogger::new(db, bus)
    }

    #[tokio::test]
    async fn test_log_and_query() {
        let logger = test_logger();

        logger
            .log_check(
                "ext-1",
                &Capability::NetworkHttp(None),
                &PermissionDecision::Allow,
                None,
            )
            .await;

        logger
            .log_check(
                "ext-1",
                &Capability::ClipboardRead,
                &PermissionDecision::Deny {
                    reason: "test".to_string(),
                },
                None,
            )
            .await;

        let records = logger
            .query(AuditQuery {
                limit: Some(10),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(records.len(), 2);
    }

    #[tokio::test]
    async fn test_query_with_filter() {
        let logger = test_logger();

        logger
            .log_check(
                "ext-1",
                &Capability::NetworkHttp(None),
                &PermissionDecision::Allow,
                None,
            )
            .await;
        logger
            .log_check(
                "ext-2",
                &Capability::ClipboardRead,
                &PermissionDecision::Deny {
                    reason: "test".to_string(),
                },
                None,
            )
            .await;

        let records = logger
            .query(AuditQuery {
                extension_id: Some("ext-1".to_string()),
                limit: Some(10),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].extension_id.as_deref(), Some("ext-1"));
    }

    #[tokio::test]
    async fn test_export_json() {
        let logger = test_logger();

        logger
            .log_check(
                "ext-1",
                &Capability::NetworkHttp(None),
                &PermissionDecision::Allow,
                None,
            )
            .await;

        let data = logger.export(ExportFormat::Json, None).await.unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_slice(&data).unwrap();
        assert_eq!(parsed.len(), 1);
    }

    #[tokio::test]
    async fn test_export_csv() {
        let logger = test_logger();

        logger
            .log_check(
                "ext-1",
                &Capability::NetworkHttp(None),
                &PermissionDecision::Allow,
                None,
            )
            .await;

        let data = logger.export(ExportFormat::Csv, None).await.unwrap();
        let csv_str = String::from_utf8(data).unwrap();
        // Should have a header row + 1 data row
        let lines: Vec<&str> = csv_str.trim().lines().collect();
        assert_eq!(lines.len(), 2);
    }
}
