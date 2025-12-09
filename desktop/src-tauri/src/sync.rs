//! Background sync engine
//!
//! Handles automatic clipboard synchronization with the server.

use crate::clipboard::{read_clipboard, write_clipboard, ClipboardContent};
use crate::config::AppConfig;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// Sync state management
pub struct SyncState {
    pub auto_sync_enabled: AtomicBool,
    pub last_server_index: AtomicU64,
    pub is_syncing: AtomicBool,
    pub server_url: String,
    client: Client,
}

impl SyncState {
    pub fn new(server_url: String) -> Self {
        Self {
            auto_sync_enabled: AtomicBool::new(true),
            last_server_index: AtomicU64::new(0),
            is_syncing: AtomicBool::new(false),
            server_url,
            client: Client::new(),
        }
    }
}

/// Sync status for frontend
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub auto_sync_enabled: bool,
    pub is_syncing: bool,
    pub last_server_index: u64,
}

/// Sync event types
#[derive(Clone, Serialize)]
#[serde(tag = "type")]
pub enum SyncEvent {
    Started,
    Pulled { content_type: String },
    Pushed { content_type: String },
    Error { message: String },
    Completed,
}

/// Start the background sync loop
pub async fn start_background_sync(app: AppHandle) {
    log::info!("Starting background sync");

    let config = AppConfig::load().unwrap_or_default();
    let interval = Duration::from_secs(config.sync_interval);

    loop {
        tokio::time::sleep(interval).await;

        let state = app.state::<crate::AppState>();

        if !state.sync_state.auto_sync_enabled.load(Ordering::SeqCst) {
            continue;
        }

        if state.sync_state.is_syncing.swap(true, Ordering::SeqCst) {
            continue; // Already syncing
        }

        // Try to pull from server
        if let Err(e) = pull_from_server(&app, &state.sync_state).await {
            log::debug!("Pull failed (might be normal): {}", e);
        }

        state.sync_state.is_syncing.store(false, Ordering::SeqCst);
    }
}

/// Pull clipboard content from server
async fn pull_from_server(app: &AppHandle, state: &SyncState) -> Result<(), String> {
    let current_index = state.last_server_index.load(Ordering::SeqCst);

    let resp = state
        .client
        .get(&format!("{}/api/v1/clipboard", state.server_url))
        .header("X-Index", current_index.to_string())
        .send()
        .await
        .map_err(|e| e.to_string())?;

    // 304 means no new data
    if resp.status() == 304 {
        return Ok(());
    }

    if !resp.status().is_success() {
        return Err(format!("Server error: {}", resp.status()));
    }

    // Parse response headers before consuming response
    let new_index: u64 = resp
        .headers()
        .get("x-index")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let content_type = resp
        .headers()
        .get("x-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("text")
        .to_string();

    // No new data
    if new_index == 0 || new_index == current_index {
        return Ok(());
    }

    // Get data (consumes response)
    let data = resp.bytes().await.map_err(|e| e.to_string())?;

    // Convert to clipboard content
    let content = match content_type.as_str() {
        "text" => ClipboardContent::Text(String::from_utf8_lossy(&data).to_string()),
        "screenshot" => ClipboardContent::Image(data.to_vec()),
        _ => return Ok(()), // Unsupported type
    };

    // Write to system clipboard
    write_clipboard(content)?;

    // Update index
    state.last_server_index.store(new_index, Ordering::SeqCst);

    // Notify frontend
    let _ = app.emit(
        "sync-event",
        SyncEvent::Pulled {
            content_type: content_type.clone(),
        },
    );

    log::info!("Pulled {} from server, index: {}", content_type, new_index);

    Ok(())
}

/// Push clipboard content to server
pub async fn push_to_server(app: &AppHandle, content: &ClipboardContent) -> Result<(), String> {
    let state = app.state::<crate::AppState>();
    let config = state.config.lock().await;

    let (data, content_type) = match content {
        ClipboardContent::Text(text) => (text.as_bytes().to_vec(), "text"),
        ClipboardContent::Image(img) => (img.clone(), "screenshot"),
    };

    let resp = state
        .sync_state
        .client
        .post(&format!("{}/api/v1/clipboard", config.server_url))
        .header("Content-Type", "application/octet-stream")
        .header("X-Type", content_type)
        .body(data)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("Server error: {}", resp.status()));
    }

    // Update index
    if let Some(index) = resp
        .headers()
        .get("x-index")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
    {
        state
            .sync_state
            .last_server_index
            .store(index, Ordering::SeqCst);
    }

    // Notify frontend
    let _ = app.emit(
        "sync-event",
        SyncEvent::Pushed {
            content_type: content_type.to_string(),
        },
    );

    log::info!("Pushed {} to server", content_type);

    Ok(())
}

/// Tauri command: Trigger manual sync
#[tauri::command]
pub async fn sync_now(app: AppHandle) -> Result<(), String> {
    let state = app.state::<crate::AppState>();

    if state.sync_state.is_syncing.swap(true, Ordering::SeqCst) {
        return Err("Sync already in progress".into());
    }

    let _ = app.emit("sync-event", SyncEvent::Started);

    // First try to push local clipboard
    if let Ok(content) = read_clipboard() {
        if let Err(e) = push_to_server(&app, &content).await {
            log::error!("Push failed: {}", e);
        }
    }

    // Then try to pull from server
    if let Err(e) = pull_from_server(&app, &state.sync_state).await {
        log::debug!("Pull failed: {}", e);
    }

    state.sync_state.is_syncing.store(false, Ordering::SeqCst);

    let _ = app.emit("sync-event", SyncEvent::Completed);

    Ok(())
}

/// Tauri command: Toggle auto sync
#[tauri::command]
pub fn toggle_auto_sync(app: AppHandle) -> bool {
    let state = app.state::<crate::AppState>();
    let current = state.sync_state.auto_sync_enabled.load(Ordering::SeqCst);
    let new_value = !current;
    state
        .sync_state
        .auto_sync_enabled
        .store(new_value, Ordering::SeqCst);
    new_value
}

/// Tauri command: Get sync status
#[tauri::command]
pub fn get_sync_status(app: AppHandle) -> SyncStatus {
    let state = app.state::<crate::AppState>();
    SyncStatus {
        auto_sync_enabled: state.sync_state.auto_sync_enabled.load(Ordering::SeqCst),
        is_syncing: state.sync_state.is_syncing.load(Ordering::SeqCst),
        last_server_index: state.sync_state.last_server_index.load(Ordering::SeqCst),
    }
}
