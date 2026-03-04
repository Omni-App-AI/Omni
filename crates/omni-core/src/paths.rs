use std::path::PathBuf;

use crate::error::{OmniError, Result};

fn app_dir_name() -> &'static str {
    if cfg!(target_os = "linux") {
        "omni"
    } else {
        "Omni"
    }
}

pub struct OmniPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub log_dir: PathBuf,
}

impl OmniPaths {
    pub fn resolve() -> Result<Self> {
        let name = app_dir_name();

        let config_dir = dirs::config_dir()
            .ok_or_else(|| OmniError::Config("Cannot determine config directory".into()))?
            .join(name);

        let data_dir = dirs::data_dir()
            .ok_or_else(|| OmniError::Config("Cannot determine data directory".into()))?
            .join(name);

        let log_dir = data_dir.join("logs");

        Ok(Self {
            config_dir,
            data_dir,
            log_dir,
        })
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    pub fn database_file(&self) -> PathBuf {
        self.data_dir.join("omni.db")
    }

    pub fn log_file(&self) -> PathBuf {
        self.log_dir.join("omni.log")
    }

    pub fn extensions_dir(&self) -> PathBuf {
        self.data_dir.join("extensions")
    }

    pub fn ensure_dirs_exist(&self) -> Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(&self.log_dir)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_returns_nonempty_paths() {
        let paths = OmniPaths::resolve().unwrap();
        assert!(!paths.config_dir.as_os_str().is_empty());
        assert!(!paths.data_dir.as_os_str().is_empty());
        assert!(!paths.log_dir.as_os_str().is_empty());
    }

    #[test]
    fn test_config_file_ends_with_toml() {
        let paths = OmniPaths::resolve().unwrap();
        assert_eq!(paths.config_file().file_name().unwrap(), "config.toml");
    }

    #[test]
    fn test_database_file_ends_with_db() {
        let paths = OmniPaths::resolve().unwrap();
        assert_eq!(paths.database_file().file_name().unwrap(), "omni.db");
    }
}
