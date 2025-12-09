//! GCopy Desktop - Rust Backend
//!
//! This module provides the Tauri commands and state management for the desktop app.

mod clipboard;
mod config;
mod sync;
mod tray;

pub use clipboard::*;
pub use config::*;
pub use sync::*;
pub use tray::*;

use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

/// Application state shared across all windows and commands
pub struct AppState {
    pub config: Arc<Mutex<AppConfig>>,
    pub sync_state: Arc<SyncState>,
}

impl AppState {
    pub fn new() -> Self {
        let config = AppConfig::load().unwrap_or_default();
        let sync_state = SyncState::new(config.server_url.clone());

        Self {
            config: Arc::new(Mutex::new(config)),
            sync_state: Arc::new(sync_state),
        }
    }
}

/// Initialize the Tauri application with all plugins and commands
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .setup(|app| {
            // Initialize application state
            let state = AppState::new();
            app.manage(state);

            // Setup system tray
            tray::setup_tray(app.handle())?;

            // Start clipboard monitoring
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                clipboard::start_clipboard_monitor(handle);
            });

            // Start background sync
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                sync::start_background_sync(handle).await;
            });

            log::info!("GCopy Desktop started successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Clipboard commands
            clipboard::read_clipboard,
            clipboard::write_clipboard,
            // Config commands
            config::get_config,
            config::save_config,
            // Sync commands
            sync::sync_now,
            sync::toggle_auto_sync,
            sync::get_sync_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
