use omni_core::config::OmniConfig;
use omni_core::database::{AuditEntry, Database, NewMessage};
use omni_core::events::{EventBus, OmniEvent};
use tempfile::tempdir;

#[tokio::test]
async fn test_full_startup_sequence() {
    let dir = tempdir().unwrap();

    // 1. Write and load config
    let config_path = dir.path().join("config.toml");
    let config = OmniConfig::default();
    config.save(&config_path).unwrap();

    let loaded = OmniConfig::load(&config_path).unwrap();
    assert_eq!(loaded.general.log_level, "info");
    assert!(loaded.validate().is_empty());

    // 2. Open encrypted database
    let db_path = dir.path().join("test.db");
    let db = Database::open(&db_path, "integration-test-key").unwrap();

    // 3. Create event bus, emit and receive
    let bus = EventBus::new(64);
    let mut rx = bus.subscribe();

    bus.emit(OmniEvent::ConfigChanged);
    let event = rx.recv().await.unwrap();
    assert!(matches!(event, OmniEvent::ConfigChanged));

    // 4. Database CRUD
    let session_id = db.create_session(Some(r#"{"test": true}"#)).unwrap();
    let session = db.get_session(&session_id).unwrap().unwrap();
    assert_eq!(session.id, session_id);

    let msg_id = db
        .insert_message(&NewMessage {
            session_id: session_id.clone(),
            role: "user".to_string(),
            content: "Hello from integration test".to_string(),
            tool_call_id: None,
            tool_calls: None,
            token_count: Some(5),
        })
        .unwrap();
    assert!(!msg_id.is_empty());

    let messages = db.get_messages_for_session(&session_id).unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "Hello from integration test");

    db.log_audit_event(&AuditEntry {
        event_type: "test_event".to_string(),
        extension_id: None,
        capability: None,
        decision: None,
        details: Some(r#"{"integration": true}"#.to_string()),
        session_id: Some(session_id),
    })
    .unwrap();

    let audit = db.get_audit_log(10).unwrap();
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0].event_type, "test_event");
}
