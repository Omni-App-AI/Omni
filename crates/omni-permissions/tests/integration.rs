use std::sync::{Arc, Mutex};

use omni_core::database::Database;
use omni_core::events::EventBus;
use omni_permissions::audit::{AuditQuery, ExportFormat};
use omni_permissions::capability::*;
use omni_permissions::policy::{DefaultPolicy, PermissionDecision, PermissionDuration, StoredDecision};
use omni_permissions::PermissionManager;
use tempfile::tempdir;

fn setup() -> PermissionManager {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let db = Database::open(&db_path, "test-key").unwrap();
    let db = Arc::new(Mutex::new(db));
    let bus = EventBus::new(64);
    std::mem::forget(dir);
    PermissionManager::with_default_policy(db, bus, DefaultPolicy::Deny)
}

#[tokio::test]
async fn test_full_permission_flow() {
    let mgr = setup();

    let cap = Capability::NetworkHttp(Some(NetworkScope {
        domains: Some(vec!["api.example.com".to_string()]),
        methods: Some(vec!["GET".to_string()]),
        ports: None,
    }));

    let allowed_request = CapabilityRequest::HttpRequest {
        url: url::Url::parse("https://api.example.com/data").unwrap(),
        method: "GET".to_string(),
        body_size: None,
    };

    let disallowed_request = CapabilityRequest::HttpRequest {
        url: url::Url::parse("https://evil.com/steal").unwrap(),
        method: "GET".to_string(),
        body_size: None,
    };

    // 1. Initially denied (no policy)
    let decision = mgr
        .check_permission("ext-1", &cap, &allowed_request, None)
        .await;
    assert!(
        matches!(decision, PermissionDecision::Deny { .. }),
        "Expected deny, got {:?}",
        decision
    );

    // 2. Record allow with session duration
    mgr.record_decision("ext-1", &cap, StoredDecision::Allow, PermissionDuration::Session)
        .await
        .unwrap();

    // 3. Check allowed request -- should pass (policy + scope)
    let decision = mgr
        .check_permission("ext-1", &cap, &allowed_request, None)
        .await;
    assert!(
        matches!(decision, PermissionDecision::Allow),
        "Expected allow, got {:?}",
        decision
    );

    // 4. Check disallowed domain -- policy allows but scope fails
    let decision = mgr
        .check_permission("ext-1", &cap, &disallowed_request, None)
        .await;
    assert!(
        matches!(decision, PermissionDecision::Deny { .. }),
        "Expected scope violation deny, got {:?}",
        decision
    );

    // 5. Revoke all for ext-1
    mgr.revoke_all("ext-1").await.unwrap();

    // 6. Check returns deny again
    let decision = mgr
        .check_permission("ext-1", &cap, &allowed_request, None)
        .await;
    assert!(matches!(decision, PermissionDecision::Deny { .. }));

    // 7. Query audit log -- should have all checks recorded
    let records = mgr
        .query_audit(AuditQuery {
            limit: Some(100),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(records.len() >= 4, "Expected at least 4 audit records, got {}", records.len());

    // 8. Export as JSON
    let json_data = mgr.export_audit(ExportFormat::Json, None).await.unwrap();
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&json_data).unwrap();
    assert!(!parsed.is_empty());

    // 9. Export as CSV
    let csv_data = mgr.export_audit(ExportFormat::Csv, None).await.unwrap();
    let csv_str = String::from_utf8(csv_data).unwrap();
    let lines: Vec<&str> = csv_str.trim().lines().collect();
    assert!(lines.len() >= 2, "Expected header + data rows");
}

#[tokio::test]
async fn test_kill_switch() {
    let mgr = setup();

    // Set up policies for multiple extensions
    mgr.record_decision(
        "ext-1",
        &Capability::NetworkHttp(None),
        StoredDecision::Allow,
        PermissionDuration::Always,
    )
    .await
    .unwrap();

    mgr.record_decision(
        "ext-2",
        &Capability::ClipboardRead,
        StoredDecision::Allow,
        PermissionDuration::Always,
    )
    .await
    .unwrap();

    mgr.record_decision(
        "ext-3",
        &Capability::DeviceCamera,
        StoredDecision::Allow,
        PermissionDuration::Always,
    )
    .await
    .unwrap();

    // Kill switch
    let start = std::time::Instant::now();
    let count = mgr.revoke_everything().await.unwrap();
    let elapsed = start.elapsed();

    assert_eq!(count, 3);
    assert!(
        elapsed.as_millis() < 10,
        "Kill switch took {:?}, expected < 10ms",
        elapsed
    );

    // All should be denied now
    let req = CapabilityRequest::HttpRequest {
        url: url::Url::parse("https://any.com").unwrap(),
        method: "GET".to_string(),
        body_size: None,
    };

    let d1 = mgr
        .check_permission("ext-1", &Capability::NetworkHttp(None), &req, None)
        .await;
    let d2 = mgr
        .check_permission("ext-2", &Capability::ClipboardRead, &CapabilityRequest::ClipboardRead, None)
        .await;
    let d3 = mgr
        .check_permission("ext-3", &Capability::DeviceCamera, &CapabilityRequest::AccessCamera, None)
        .await;

    assert!(matches!(d1, PermissionDecision::Deny { .. }));
    assert!(matches!(d2, PermissionDecision::Deny { .. }));
    assert!(matches!(d3, PermissionDecision::Deny { .. }));
}
