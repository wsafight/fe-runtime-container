use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectSettings {
    pub runtime: String,
    pub memory: String,
    pub last_used: u64,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct StorageData {
    pub projects: HashMap<String, ProjectSettings>,
}

pub struct Storage;

impl Storage {
    fn config_path() -> Result<PathBuf> {
        let config_dir =
            dirs::config_dir().ok_or_else(|| anyhow::anyhow!("Cannot find config directory"))?;
        Ok(config_dir.join("frc").join("config.json"))
    }

    pub fn load() -> Result<StorageData> {
        let path = Self::config_path()?;

        if !path.exists() {
            return Ok(StorageData::default());
        }

        let content = fs::read_to_string(&path)?;

        match serde_json::from_str::<StorageData>(&content) {
            Ok(data) => Ok(data),
            Err(_) => {
                eprintln!("⚠️  Old config format detected, recreating...");
                fs::remove_file(&path)?;
                Ok(StorageData::default())
            }
        }
    }

    pub fn save(data: &StorageData) -> Result<()> {
        let path = Self::config_path()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(data)?;
        fs::write(&path, content)?;
        Ok(())
    }
}
