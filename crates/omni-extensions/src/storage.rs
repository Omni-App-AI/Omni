use std::sync::{Arc, Mutex};

use omni_core::database::Database;
use omni_core::error::Result;

/// Storage interface for extensions to persist key-value data.
pub trait ExtensionStorage: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<String>>;
    fn set(&self, key: &str, value: &str) -> Result<()>;
    fn delete(&self, key: &str) -> Result<bool>;
    fn list_keys(&self) -> Result<Vec<String>>;
}

/// Database-backed implementation of ExtensionStorage.
pub struct DatabaseStorage {
    db: Arc<Mutex<Database>>,
    extension_id: String,
}

impl DatabaseStorage {
    pub fn new(db: Arc<Mutex<Database>>, extension_id: &str) -> Self {
        Self {
            db,
            extension_id: extension_id.to_string(),
        }
    }
}

impl ExtensionStorage for DatabaseStorage {
    fn get(&self, key: &str) -> Result<Option<String>> {
        let db = self.db.lock().unwrap();
        db.get_extension_state(&self.extension_id, key)
    }

    fn set(&self, key: &str, value: &str) -> Result<()> {
        let db = self.db.lock().unwrap();
        db.set_extension_state(&self.extension_id, key, value)
    }

    fn delete(&self, key: &str) -> Result<bool> {
        let db = self.db.lock().unwrap();
        db.delete_extension_state_key(&self.extension_id, key)
    }

    fn list_keys(&self) -> Result<Vec<String>> {
        let db = self.db.lock().unwrap();
        db.list_extension_state_keys(&self.extension_id)
    }
}
