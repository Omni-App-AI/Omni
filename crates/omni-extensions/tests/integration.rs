use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use omni_core::database::Database;
use omni_core::events::EventBus;
use omni_permissions::policy::{DefaultPolicy, PolicyEngine};

use omni_extensions::error::ExtensionError;
use omni_extensions::host::{ExtensionHost, ExtensionSource};
use omni_extensions::manifest::ExtensionManifest;
use omni_extensions::sandbox::{ResourceLimits, SandboxConfig, WasmSandbox};

/// Get the path to the test fixtures directory.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Create a test database in a temp directory.
fn test_db() -> (Arc<Mutex<Database>>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let db = Database::open(&db_path, "test-key").unwrap();
    (Arc::new(Mutex::new(db)), dir)
}

/// Compile the hello.wat fixture to WASM bytes.
fn compile_hello_wasm() -> Vec<u8> {
    let wat_path = fixtures_dir()
        .join("com.omni.test-hello")
        .join("hello.wat");
    let wat_content = std::fs::read_to_string(&wat_path).unwrap();
    wat::parse_str(&wat_content).unwrap()
}

/// Write compiled WASM to the fixture directory so the host can load it.
fn ensure_hello_wasm_exists() {
    let wasm_path = fixtures_dir()
        .join("com.omni.test-hello")
        .join("hello.wasm");
    if !wasm_path.exists() {
        let wasm_bytes = compile_hello_wasm();
        std::fs::write(&wasm_path, &wasm_bytes).unwrap();
    }
}

// ---- Manifest Tests ----

#[test]
fn test_load_hello_manifest() {
    let manifest_path = fixtures_dir()
        .join("com.omni.test-hello")
        .join("omni-extension.toml");
    let manifest = ExtensionManifest::load(&manifest_path).unwrap();

    assert_eq!(manifest.extension.id, "com.omni.test-hello");
    assert_eq!(manifest.extension.name, "Test Hello");
    assert_eq!(manifest.extension.version, "0.1.0");
    assert_eq!(manifest.runtime.entrypoint, "hello.wasm");
    assert_eq!(manifest.tools.len(), 1);
    assert_eq!(manifest.tools[0].name, "hello");
}

// ---- Sandbox Tests ----

#[test]
fn test_sandbox_call_tool() {
    let wasm_bytes = compile_hello_wasm();
    let (db, _dir) = test_db();

    let policy = Arc::new(PolicyEngine::new(db.clone(), DefaultPolicy::Deny));
    let sandbox = WasmSandbox::new(&SandboxConfig::default()).unwrap();

    let state = WasmSandbox::create_state("com.omni.test-hello", policy, db, 1024 * 1024);
    let limits = ResourceLimits {
        max_fuel: 10_000_000,
        max_memory_bytes: 1024 * 1024,
    };

    let mut instance = sandbox.instantiate(&wasm_bytes, state, &limits).unwrap();

    let result = WasmSandbox::call_tool(
        &mut instance,
        "hello",
        "{}",
        Duration::from_secs(5),
    )
    .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["result"], "hello");
}

#[test]
fn test_sandbox_storage_roundtrip() {
    // Create a WASM module that uses storage_set and storage_get
    let wasm = wat::parse_str(
        r#"
        (module
            (import "omni" "storage_set" (func $storage_set (param i32 i32 i32 i32) (result i32)))
            (import "omni" "storage_get" (func $storage_get (param i32 i32) (result i64)))
            (memory (export "memory") 1)

            ;; Store key "k1" at offset 0 and value "v1" at offset 16
            (data (i32.const 0) "k1")
            (data (i32.const 16) "v1")

            ;; Export a function that sets k1=v1 and returns storage_set result
            (func (export "test_set") (result i32)
                (call $storage_set (i32.const 0) (i32.const 2) (i32.const 16) (i32.const 2))
            )

            ;; Export a function that gets k1 and returns the packed ptr|len
            (func (export "test_get") (result i64)
                (call $storage_get (i32.const 0) (i32.const 2))
            )
        )
        "#,
    )
    .unwrap();

    let (db, _dir) = test_db();
    let policy = Arc::new(PolicyEngine::new(db.clone(), DefaultPolicy::Deny));
    let sandbox = WasmSandbox::new(&SandboxConfig::default()).unwrap();

    let state = WasmSandbox::create_state("test-storage", policy, db.clone(), 1024 * 1024);
    let limits = ResourceLimits {
        max_fuel: 10_000_000,
        max_memory_bytes: 1024 * 1024,
    };

    let mut instance = sandbox.instantiate(&wasm, state, &limits).unwrap();

    // Call test_set -- should store k1=v1
    let set_func = instance
        .instance
        .get_typed_func::<(), i32>(&mut instance.store, "test_set")
        .unwrap();
    let result = set_func.call(&mut instance.store, ()).unwrap();
    assert_eq!(result, 0, "storage_set should return 0 (success)");

    // Verify via direct DB access
    let db_guard = db.lock().unwrap();
    let stored = db_guard
        .get_extension_state("test-storage", "k1")
        .unwrap();
    assert_eq!(stored, Some("v1".to_string()));
    drop(db_guard);

    // Call test_get -- should return packed pointer to "v1"
    let get_func = instance
        .instance
        .get_typed_func::<(), i64>(&mut instance.store, "test_get")
        .unwrap();
    let packed = get_func.call(&mut instance.store, ()).unwrap();
    assert_ne!(packed, -1, "storage_get should succeed");

    let ptr = (packed >> 32) as u32;
    let len = (packed & 0xFFFF_FFFF) as u32;
    assert_eq!(len, 2, "Value length should be 2");

    // Read the value from WASM memory
    let memory = instance
        .instance
        .get_memory(&mut instance.store, "memory")
        .unwrap();
    let mut buf = vec![0u8; len as usize];
    memory
        .read(&instance.store, ptr as usize, &mut buf)
        .unwrap();
    assert_eq!(String::from_utf8(buf).unwrap(), "v1");
}

// ---- Host Lifecycle Tests ----

#[tokio::test]
async fn test_host_discover() {
    let (db, _dir) = test_db();
    let policy = Arc::new(PolicyEngine::new(db.clone(), DefaultPolicy::Deny));
    let event_bus = EventBus::new(16);

    // Use the fixtures dir as the extensions dir (it has a "com.omni.test-hello" subfolder)
    // But ExtensionHost expects bundled/ and user/ subdirectories
    let extensions_dir = tempfile::tempdir().unwrap();
    let user_dir = extensions_dir.path().join("user");
    std::fs::create_dir_all(&user_dir).unwrap();

    // Copy the test fixture to user/
    let fixture_src = fixtures_dir().join("com.omni.test-hello");
    let fixture_dst = user_dir.join("com.omni.test-hello");
    copy_dir_recursive(&fixture_src, &fixture_dst);

    let host = ExtensionHost::new(
        policy,
        event_bus,
        db,
        extensions_dir.path().to_path_buf(),
    )
    .unwrap();

    let registered = host.discover_and_register().await.unwrap();
    assert_eq!(registered.len(), 1);
    assert_eq!(registered[0], "com.omni.test-hello");
}

#[tokio::test]
async fn test_host_full_lifecycle() {
    ensure_hello_wasm_exists();

    let (db, _dir) = test_db();
    let policy = Arc::new(PolicyEngine::new(db.clone(), DefaultPolicy::Deny));
    let event_bus = EventBus::new(16);

    let extensions_dir = tempfile::tempdir().unwrap();
    let host = ExtensionHost::new(
        policy,
        event_bus,
        db,
        extensions_dir.path().to_path_buf(),
    )
    .unwrap();

    // 1. Install from path
    let source = ExtensionSource::Path(fixtures_dir().join("com.omni.test-hello"));
    let ext_id = host.install(&source).await.unwrap();
    assert_eq!(ext_id, "com.omni.test-hello");

    // 2. Verify installed
    let installed = host.list_installed().await;
    assert!(installed.contains(&"com.omni.test-hello".to_string()));

    // 3. Activate
    host.activate("com.omni.test-hello").await.unwrap();
    assert!(host.is_active("com.omni.test-hello").await);

    // 4. Invoke tool
    let params = serde_json::json!({});
    let result = host
        .invoke_tool("com.omni.test-hello", "hello", &params)
        .await
        .unwrap();
    assert_eq!(result["result"], "hello");

    // 5. Get all tools
    let tools = host.get_all_tools().await;
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].0, "com.omni.test-hello::default");
    assert_eq!(tools[0].1.name, "hello");

    // 6. Deactivate
    host.deactivate("com.omni.test-hello").await.unwrap();
    assert!(!host.is_active("com.omni.test-hello").await);

    // 7. Uninstall
    host.uninstall("com.omni.test-hello").await.unwrap();
    let installed = host.list_installed().await;
    assert!(!installed.contains(&"com.omni.test-hello".to_string()));
}

#[tokio::test]
async fn test_invoke_not_active() {
    let (db, _dir) = test_db();
    let policy = Arc::new(PolicyEngine::new(db.clone(), DefaultPolicy::Deny));
    let event_bus = EventBus::new(16);
    let extensions_dir = tempfile::tempdir().unwrap();

    let host = ExtensionHost::new(
        policy,
        event_bus,
        db,
        extensions_dir.path().to_path_buf(),
    )
    .unwrap();

    let result = host
        .invoke_tool("nonexistent", "hello", &serde_json::json!({}))
        .await;
    assert!(matches!(result, Err(ExtensionError::NotActive(_))));
}

#[tokio::test]
async fn test_activate_not_installed() {
    let (db, _dir) = test_db();
    let policy = Arc::new(PolicyEngine::new(db.clone(), DefaultPolicy::Deny));
    let event_bus = EventBus::new(16);
    let extensions_dir = tempfile::tempdir().unwrap();

    let host = ExtensionHost::new(
        policy,
        event_bus,
        db,
        extensions_dir.path().to_path_buf(),
    )
    .unwrap();

    let result = host.activate("nonexistent").await;
    assert!(matches!(result, Err(ExtensionError::NotFound(_))));
}

// ---- Resource Limit Tests ----

#[test]
fn test_fuel_exhaustion_via_call_tool() {
    // Create a WASM module with handle_tool that loops forever
    let wasm = wat::parse_str(
        r#"
        (module
            (memory (export "memory") 1)
            (func (export "handle_tool") (param i32 i32 i32 i32) (result i64)
                (loop $l
                    (br $l)
                )
                (unreachable)
            )
        )
        "#,
    )
    .unwrap();

    let (db, _dir) = test_db();
    let policy = Arc::new(PolicyEngine::new(db.clone(), DefaultPolicy::Deny));
    let sandbox = WasmSandbox::new(&SandboxConfig::default()).unwrap();

    let state = WasmSandbox::create_state("test-fuel", policy, db, 1024 * 1024);
    let limits = ResourceLimits {
        max_fuel: 1000,
        max_memory_bytes: 1024 * 1024,
    };

    let mut instance = sandbox.instantiate(&wasm, state, &limits).unwrap();

    let result = WasmSandbox::call_tool(
        &mut instance,
        "test",
        "{}",
        Duration::from_secs(5),
    );
    assert!(result.is_err(), "Should fail due to fuel exhaustion");
}

// ---- Multi-Instance Tests ----

/// Helper: install the test-hello extension and return the host.
async fn setup_host_with_hello() -> (
    ExtensionHost,
    Arc<Mutex<Database>>,
    tempfile::TempDir,
    tempfile::TempDir,
) {
    ensure_hello_wasm_exists();
    let (db, db_dir) = test_db();
    let policy = Arc::new(PolicyEngine::new(db.clone(), DefaultPolicy::Deny));
    let event_bus = EventBus::new(16);
    let extensions_dir = tempfile::tempdir().unwrap();

    let host = ExtensionHost::new(
        policy,
        event_bus,
        db.clone(),
        extensions_dir.path().to_path_buf(),
    )
    .unwrap();

    let source = ExtensionSource::Path(fixtures_dir().join("com.omni.test-hello"));
    host.install(&source).await.unwrap();

    (host, db, db_dir, extensions_dir)
}

#[tokio::test]
async fn test_create_multiple_instances() {
    let (host, _db, _db_dir, _ext_dir) = setup_host_with_hello().await;
    let ext_id = "com.omni.test-hello";

    // Install auto-creates ::default -- create two more
    let id_a = host.create_instance(ext_id, "alpha", Some("Alpha Bot".to_string())).await.unwrap();
    let id_b = host.create_instance(ext_id, "beta", None).await.unwrap();

    assert_eq!(id_a, "com.omni.test-hello::alpha");
    assert_eq!(id_b, "com.omni.test-hello::beta");

    // List all instances for extension: should have default + alpha + beta
    let instances = host.list_instances(Some(ext_id)).await;
    assert_eq!(instances.len(), 3);

    let names: Vec<&str> = instances.iter().map(|(_, m)| m.instance_name.as_str()).collect();
    assert!(names.contains(&"default"));
    assert!(names.contains(&"alpha"));
    assert!(names.contains(&"beta"));

    // Verify display name
    let alpha = instances.iter().find(|(_, m)| m.instance_name == "alpha").unwrap();
    assert_eq!(alpha.1.display_name, Some("Alpha Bot".to_string()));
}

#[tokio::test]
async fn test_duplicate_instance_rejected() {
    let (host, _db, _db_dir, _ext_dir) = setup_host_with_hello().await;
    let ext_id = "com.omni.test-hello";

    host.create_instance(ext_id, "dup", None).await.unwrap();
    let result = host.create_instance(ext_id, "dup", None).await;
    assert!(result.is_err(), "Duplicate instance name should be rejected");
}

#[tokio::test]
async fn test_activate_deactivate_individual_instances() {
    let (host, _db, _db_dir, _ext_dir) = setup_host_with_hello().await;
    let ext_id = "com.omni.test-hello";

    host.create_instance(ext_id, "inst-a", None).await.unwrap();
    host.create_instance(ext_id, "inst-b", None).await.unwrap();

    let id_a = "com.omni.test-hello::inst-a";
    let id_b = "com.omni.test-hello::inst-b";

    // Activate both
    host.activate(id_a).await.unwrap();
    host.activate(id_b).await.unwrap();
    assert!(host.is_active(id_a).await);
    assert!(host.is_active(id_b).await);

    // Deactivate only A -- B should still be active
    host.deactivate(id_a).await.unwrap();
    assert!(!host.is_active(id_a).await);
    assert!(host.is_active(id_b).await);
}

#[tokio::test]
async fn test_invoke_tool_on_specific_instance() {
    let (host, _db, _db_dir, _ext_dir) = setup_host_with_hello().await;
    let ext_id = "com.omni.test-hello";

    host.create_instance(ext_id, "worker", None).await.unwrap();
    let instance_id = "com.omni.test-hello::worker";

    host.activate(instance_id).await.unwrap();

    let result = host
        .invoke_tool(instance_id, "hello", &serde_json::json!({}))
        .await
        .unwrap();
    assert_eq!(result["result"], "hello");

    // Default instance is NOT active, should fail
    let default_result = host
        .invoke_tool("com.omni.test-hello::default", "hello", &serde_json::json!({}))
        .await;
    assert!(matches!(default_result, Err(ExtensionError::NotActive(_))));
}

#[tokio::test]
async fn test_get_all_tools_returns_per_instance() {
    let (host, _db, _db_dir, _ext_dir) = setup_host_with_hello().await;
    let ext_id = "com.omni.test-hello";

    host.create_instance(ext_id, "one", None).await.unwrap();
    host.create_instance(ext_id, "two", None).await.unwrap();

    host.activate("com.omni.test-hello::one").await.unwrap();
    host.activate("com.omni.test-hello::two").await.unwrap();

    let tools = host.get_all_tools().await;
    // Each instance contributes its tools
    let instance_ids: Vec<&str> = tools.iter().map(|(id, _)| id.as_str()).collect();
    assert!(instance_ids.contains(&"com.omni.test-hello::one"));
    assert!(instance_ids.contains(&"com.omni.test-hello::two"));
    assert!(tools.len() >= 2);
}

#[tokio::test]
async fn test_separate_storage_per_instance() {
    let (host, db, _db_dir, _ext_dir) = setup_host_with_hello().await;
    let ext_id = "com.omni.test-hello";

    host.create_instance(ext_id, "store-a", None).await.unwrap();
    host.create_instance(ext_id, "store-b", None).await.unwrap();

    // Manually write distinct storage values for each instance namespace
    {
        let db_guard = db.lock().unwrap();
        db_guard
            .set_extension_state("com.omni.test-hello::store-a", "color", "red")
            .unwrap();
        db_guard
            .set_extension_state("com.omni.test-hello::store-b", "color", "blue")
            .unwrap();
    }

    // Verify isolation: each instance sees only its own value
    {
        let db_guard = db.lock().unwrap();
        let val_a = db_guard
            .get_extension_state("com.omni.test-hello::store-a", "color")
            .unwrap();
        let val_b = db_guard
            .get_extension_state("com.omni.test-hello::store-b", "color")
            .unwrap();
        assert_eq!(val_a, Some("red".to_string()));
        assert_eq!(val_b, Some("blue".to_string()));
    }
}

#[tokio::test]
async fn test_delete_instance() {
    let (host, _db, _db_dir, _ext_dir) = setup_host_with_hello().await;
    let ext_id = "com.omni.test-hello";

    host.create_instance(ext_id, "ephemeral", None).await.unwrap();
    let instance_id = "com.omni.test-hello::ephemeral";

    // Activate then delete -- should deactivate and remove
    host.activate(instance_id).await.unwrap();
    assert!(host.is_active(instance_id).await);

    host.delete_instance(instance_id).await.unwrap();
    assert!(!host.is_active(instance_id).await);

    // Should no longer appear in list
    let instances = host.list_instances(Some(ext_id)).await;
    assert!(!instances.iter().any(|(id, _)| id == instance_id));
}

#[tokio::test]
async fn test_uninstall_cleans_all_instances() {
    let (host, _db, _db_dir, _ext_dir) = setup_host_with_hello().await;
    let ext_id = "com.omni.test-hello";

    host.create_instance(ext_id, "x", None).await.unwrap();
    host.create_instance(ext_id, "y", None).await.unwrap();
    host.activate("com.omni.test-hello::x").await.unwrap();
    host.activate("com.omni.test-hello::y").await.unwrap();

    // Uninstall should clean up all instances
    host.uninstall(ext_id).await.unwrap();

    assert!(!host.is_active("com.omni.test-hello::x").await);
    assert!(!host.is_active("com.omni.test-hello::y").await);
    assert!(!host.is_active("com.omni.test-hello::default").await);

    let instances = host.list_instances(Some(ext_id)).await;
    assert!(instances.is_empty());

    let installed = host.list_installed().await;
    assert!(!installed.contains(&ext_id.to_string()));
}

#[tokio::test]
async fn test_backward_compat_bare_id_resolves_to_default() {
    let (host, _db, _db_dir, _ext_dir) = setup_host_with_hello().await;

    // Activate with bare ID -- should activate the ::default instance
    host.activate("com.omni.test-hello").await.unwrap();
    assert!(host.is_active("com.omni.test-hello").await);
    assert!(host.is_active("com.omni.test-hello::default").await);

    // Invoke with bare ID
    let result = host
        .invoke_tool("com.omni.test-hello", "hello", &serde_json::json!({}))
        .await
        .unwrap();
    assert_eq!(result["result"], "hello");

    // Deactivate with bare ID
    host.deactivate("com.omni.test-hello").await.unwrap();
    assert!(!host.is_active("com.omni.test-hello::default").await);
}

#[tokio::test]
async fn test_update_instance_display_name() {
    let (host, _db, _db_dir, _ext_dir) = setup_host_with_hello().await;
    let ext_id = "com.omni.test-hello";

    host.create_instance(ext_id, "named", Some("Original".to_string())).await.unwrap();
    let instance_id = "com.omni.test-hello::named";

    // Verify original display name
    let instances = host.list_instances(Some(ext_id)).await;
    let meta = instances.iter().find(|(id, _)| id == instance_id).unwrap();
    assert_eq!(meta.1.display_name, Some("Original".to_string()));

    // Update
    host.update_instance(instance_id, Some("Updated".to_string())).await.unwrap();

    let instances = host.list_instances(Some(ext_id)).await;
    let meta = instances.iter().find(|(id, _)| id == instance_id).unwrap();
    assert_eq!(meta.1.display_name, Some("Updated".to_string()));
}

#[tokio::test]
async fn test_discover_creates_default_instance() {
    let (db, _db_dir) = test_db();
    let policy = Arc::new(PolicyEngine::new(db.clone(), DefaultPolicy::Deny));
    let event_bus = EventBus::new(16);

    ensure_hello_wasm_exists();
    let extensions_dir = tempfile::tempdir().unwrap();
    let user_dir = extensions_dir.path().join("user");
    std::fs::create_dir_all(&user_dir).unwrap();
    copy_dir_recursive(
        &fixtures_dir().join("com.omni.test-hello"),
        &user_dir.join("com.omni.test-hello"),
    );

    let host = ExtensionHost::new(
        policy,
        event_bus,
        db,
        extensions_dir.path().to_path_buf(),
    )
    .unwrap();

    host.discover_and_register().await.unwrap();

    // discover_and_register should auto-create a ::default instance
    let instances = host.list_instances(Some("com.omni.test-hello")).await;
    assert!(!instances.is_empty(), "discover_and_register should create a default instance");
    assert!(
        instances.iter().any(|(_, m)| m.instance_name == "default"),
        "Default instance should exist"
    );
}

// ---- Helper ----

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let ty = entry.file_type().unwrap();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path);
        } else {
            std::fs::copy(entry.path(), &dst_path).unwrap();
        }
    }
}
