//! Configuration management
//!
//! Handles loading, saving, and accessing application configuration.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    /// Server URL for clipboard sync
    pub server_url: String,

    /// Enable automatic sync
    pub auto_sync: bool,

    /// Sync interval in seconds
    pub sync_interval: u64,

    /// Enable auto-start on system boot
    pub auto_start: bool,

    /// Sync content types
    pub sync_types: SyncTypes,

    /// Keyboard shortcuts
    pub shortcuts: Shortcuts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncTypes {
    pub text: bool,
    pub screenshot: bool,
    pub file: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shortcuts {
    pub manual_sync: String,
    pub toggle_window: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_url: "https://gcopy.llaoj.cn".into(),
            auto_sync: true,
            sync_interval: 3,
            auto_start: false,
            sync_types: SyncTypes {
                text: true,
                screenshot: true,
                file: true,
            },
            shortcuts: Shortcuts {
                manual_sync: "CmdOrCtrl+Shift+V".into(),
                toggle_window: "CmdOrCtrl+Shift+G".into(),
            },
        }
    }
}

impl AppConfig {
    /// Load configuration from file
    pub fn load() -> Result<Self, String> {
        let path = Self::config_path();

        if path.exists() {
            let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
            serde_json::from_str(&content).map_err(|e| e.to_string())
        } else {
            Ok(Self::default())
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, content).map_err(|e| e.to_string())
    }

    /// Get configuration file path
    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gcopy")
            .join("config.json")
    }
}

/// Tauri command: Get current configuration
#[tauri::command]
pub fn get_config() -> Result<AppConfig, String> {
    AppConfig::load()
}

/// Tauri command: Save configuration
#[tauri::command]
pub fn save_config(config: AppConfig) -> Result<(), String> {
    config.save()
}
