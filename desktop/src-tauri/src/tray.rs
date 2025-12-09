//! System tray implementation
//!
//! Creates and manages the system tray icon and menu.

use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

/// Setup the system tray
pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // Create menu items
    let auto_sync = CheckMenuItem::with_id(app, "auto_sync", "自动同步 Auto Sync", true, true, None::<&str>)?;
    let sync_now = MenuItem::with_id(app, "sync_now", "立即同步 Sync Now", true, None::<&str>)?;
    let show_window = MenuItem::with_id(app, "show_window", "显示窗口 Show Window", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置 Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 Quit", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &auto_sync,
            &PredefinedMenuItem::separator(app)?,
            &sync_now,
            &show_window,
            &PredefinedMenuItem::separator(app)?,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let _tray = TrayIconBuilder::with_id("main")
        .tooltip("GCopy - 剪贴板同步")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "auto_sync" => {
                let new_state = crate::toggle_auto_sync(app.clone());
                log::info!("Auto sync toggled: {}", new_state);
            }
            "sync_now" => {
                let app_clone = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = crate::sync_now(app_clone).await {
                        log::error!("Manual sync failed: {}", e);
                    }
                });
            }
            "show_window" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "settings" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                    let _ = window.emit("open-settings", ());
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    log::info!("System tray initialized");

    Ok(())
}
