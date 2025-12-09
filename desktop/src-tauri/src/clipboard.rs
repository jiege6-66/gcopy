//! Clipboard monitoring and operations
//!
//! This module handles native clipboard access using the `arboard` crate.

use arboard::Clipboard;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Content types that can be stored in clipboard
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClipboardContent {
    Text(String),
    Image(Vec<u8>), // PNG format
}

/// Global clipboard state for change detection
static LAST_CLIPBOARD_HASH: AtomicU64 = AtomicU64::new(0);
static CLIPBOARD_MUTEX: Mutex<()> = Mutex::new(());

/// Calculate a simple hash of clipboard content for change detection
fn hash_content(content: &ClipboardContent) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    match content {
        ClipboardContent::Text(text) => {
            "text".hash(&mut hasher);
            text.hash(&mut hasher);
        }
        ClipboardContent::Image(data) => {
            "image".hash(&mut hasher);
            data.hash(&mut hasher);
        }
    }
    hasher.finish()
}

/// Read current clipboard content
#[tauri::command]
pub fn read_clipboard() -> Result<ClipboardContent, String> {
    let _lock = CLIPBOARD_MUTEX.lock().map_err(|e| e.to_string())?;

    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;

    // Try to get image first
    if let Ok(img) = clipboard.get_image() {
        // Convert to PNG
        let png_data = image_to_png(&img).map_err(|e| e.to_string())?;
        return Ok(ClipboardContent::Image(png_data));
    }

    // Fall back to text
    if let Ok(text) = clipboard.get_text() {
        if !text.is_empty() {
            return Ok(ClipboardContent::Text(text));
        }
    }

    Err("Clipboard is empty or contains unsupported format".into())
}

/// Write content to clipboard
#[tauri::command]
pub fn write_clipboard(content: ClipboardContent) -> Result<(), String> {
    let _lock = CLIPBOARD_MUTEX.lock().map_err(|e| e.to_string())?;

    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;

    match content {
        ClipboardContent::Text(text) => {
            clipboard.set_text(&text).map_err(|e| e.to_string())?;
        }
        ClipboardContent::Image(data) => {
            let img = png_to_image(&data).map_err(|e| e.to_string())?;
            clipboard.set_image(img).map_err(|e| e.to_string())?;
        }
    }

    // Update hash to prevent re-triggering sync
    if let Ok(current) = read_clipboard_internal() {
        let hash = hash_content(&current);
        LAST_CLIPBOARD_HASH.store(hash, Ordering::SeqCst);
    }

    Ok(())
}

/// Internal clipboard read without locking (for use within locked context)
fn read_clipboard_internal() -> Result<ClipboardContent, String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;

    if let Ok(img) = clipboard.get_image() {
        let png_data = image_to_png(&img).map_err(|e| e.to_string())?;
        return Ok(ClipboardContent::Image(png_data));
    }

    if let Ok(text) = clipboard.get_text() {
        if !text.is_empty() {
            return Ok(ClipboardContent::Text(text));
        }
    }

    Err("Clipboard is empty".into())
}

/// Convert arboard ImageData to PNG bytes
fn image_to_png(img: &arboard::ImageData) -> Result<Vec<u8>, String> {
    use std::io::Cursor;

    let width = img.width as u32;
    let height = img.height as u32;

    // arboard returns RGBA data
    let mut png_data = Vec::new();
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut png_data), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer
            .write_image_data(&img.bytes)
            .map_err(|e| e.to_string())?;
    }

    Ok(png_data)
}

/// Convert PNG bytes to arboard ImageData
fn png_to_image(data: &[u8]) -> Result<arboard::ImageData<'static>, String> {
    use std::io::Cursor;

    let decoder = png::Decoder::new(Cursor::new(data));
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;

    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;

    // Ensure RGBA format
    let bytes = if info.color_type == png::ColorType::Rgba {
        buf[..info.buffer_size()].to_vec()
    } else {
        // Convert to RGBA if needed (simplified - assumes RGB)
        let rgb = &buf[..info.buffer_size()];
        let mut rgba = Vec::with_capacity(info.width as usize * info.height as usize * 4);
        for chunk in rgb.chunks(3) {
            rgba.extend_from_slice(chunk);
            rgba.push(255); // Alpha
        }
        rgba
    };

    Ok(arboard::ImageData {
        width: info.width as usize,
        height: info.height as usize,
        bytes: bytes.into(),
    })
}

/// Start clipboard monitoring in a background thread
pub fn start_clipboard_monitor(app: AppHandle) {
    log::info!("Starting clipboard monitor");

    loop {
        std::thread::sleep(Duration::from_millis(500));

        let content = {
            let _lock = match CLIPBOARD_MUTEX.lock() {
                Ok(lock) => lock,
                Err(_) => continue,
            };

            match read_clipboard_internal() {
                Ok(content) => content,
                Err(_) => continue,
            }
        };

        let hash = hash_content(&content);
        let last_hash = LAST_CLIPBOARD_HASH.load(Ordering::SeqCst);

        if hash != last_hash {
            LAST_CLIPBOARD_HASH.store(hash, Ordering::SeqCst);

            // Emit event to frontend
            if let Err(e) = app.emit("clipboard-changed", &content) {
                log::error!("Failed to emit clipboard-changed event: {}", e);
            }

            log::debug!("Clipboard changed, new hash: {}", hash);
        }
    }
}
