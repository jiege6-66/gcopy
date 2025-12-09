# GCopy Desktop App - 技术设计文档

## 1. 概述

### 1.1 目标
将 GCopy 从网页版改造为跨平台桌面应用，支持：
- **Mac** 和 **Windows** 平台
- **后台运行**：系统托盘驻留，无需浏览器
- **自动同步**：监控剪贴板变化，自动推送/拉取
- **开机自启**：用户登录后自动启动

### 1.2 技术选型

| 技术 | 选择 | 理由 |
|------|------|------|
| 框架 | **Tauri 2.x** | 包体小(~3MB)、内存低(~40MB)、原生剪贴板支持 |
| 前端 | **React + TypeScript** | 复用现有 frontend/ 代码 |
| 后端 | **Rust** | Tauri 原生、高性能、跨平台 |
| 状态管理 | **Dexie (IndexedDB)** | 复用现有本地存储方案 |

### 1.3 与 Electron 对比

| 指标 | Tauri 2.x | Electron |
|------|-----------|----------|
| 安装包大小 | 2.5-3 MB | 85+ MB |
| 内存占用 | 30-40 MB | 200-300 MB |
| 启动时间 | < 500ms | 1-2s |
| 剪贴板监控 | 原生事件 | 需轮询 |
| 安全性 | 显式权限 | Node.js 全权限 |

---

## 2. 项目结构

```
gcopy/
├── frontend/              # 现有 Next.js 代码 (Web版)
├── desktop/               # 新增：Tauri 桌面应用
│   ├── src-tauri/         # Rust 后端
│   │   ├── Cargo.toml
│   │   ├── tauri.conf.json
│   │   ├── capabilities/
│   │   │   └── default.json
│   │   ├── icons/
│   │   └── src/
│   │       ├── main.rs           # 入口
│   │       ├── lib.rs            # Tauri 命令注册
│   │       ├── clipboard.rs      # 剪贴板监控
│   │       ├── tray.rs           # 系统托盘
│   │       ├── sync.rs           # 同步逻辑
│   │       ├── config.rs         # 配置管理
│   │       └── error.rs          # 错误处理
│   ├── src/               # React 前端 (从 frontend/ 复用)
│   │   ├── App.tsx
│   │   ├── components/
│   │   ├── lib/
│   │   └── hooks/
│   ├── package.json
│   ├── vite.config.ts
│   └── tsconfig.json
├── internal/              # Go 后端 (不变)
├── build/
│   └── desktop/           # 新增：桌面应用构建配置
│       └── Dockerfile
└── docs/
    └── desktop-app-design.md  # 本文档
```

---

## 3. 核心模块设计

### 3.1 剪贴板监控 (clipboard.rs)

#### 3.1.1 功能描述
监控系统剪贴板变化，支持文本、图片、文件三种类型。

#### 3.1.2 平台差异

| 平台 | 监控方式 | 库/API |
|------|---------|--------|
| Windows | 原生事件 | `WM_CLIPBOARDUPDATE` via `clipboard-rs` |
| macOS | 轮询 | `NSPasteboard::changeCount` (500ms 间隔) |
| Linux | 事件 | X11 selection via `clipboard-master` |

#### 3.1.3 实现方案

```rust
// desktop/src-tauri/src/clipboard.rs

use clipboard_rs::{Clipboard, ClipboardContext, ClipboardHandler, ClipboardWatcher, ClipboardWatcherContext};
use std::sync::mpsc;
use tauri::{AppHandle, Emitter};

/// 剪贴板变化类型
#[derive(Clone, serde::Serialize)]
pub enum ClipboardContent {
    Text(String),
    Image(Vec<u8>),        // PNG 格式
    File(Vec<String>),     // 文件路径列表
}

/// 剪贴板监控器
pub struct ClipboardMonitor {
    watcher: ClipboardWatcherContext<ClipboardHandler>,
    app_handle: AppHandle,
}

struct Handler {
    tx: mpsc::Sender<ClipboardContent>,
}

impl ClipboardHandler for Handler {
    fn on_clipboard_change(&mut self) {
        let ctx = ClipboardContext::new().unwrap();

        // 优先检查文件
        if let Ok(files) = ctx.get_files() {
            let _ = self.tx.send(ClipboardContent::File(files));
            return;
        }

        // 检查图片
        if let Ok(img) = ctx.get_image() {
            let png_data = img.to_png().unwrap().get_bytes().to_vec();
            let _ = self.tx.send(ClipboardContent::Image(png_data));
            return;
        }

        // 检查文本
        if let Ok(text) = ctx.get_text() {
            let _ = self.tx.send(ClipboardContent::Text(text));
        }
    }
}

impl ClipboardMonitor {
    pub fn new(app_handle: AppHandle) -> Self {
        let (tx, rx) = mpsc::channel();
        let handler = Handler { tx };
        let watcher = ClipboardWatcherContext::new().unwrap();

        // 后台线程处理剪贴板变化
        let app = app_handle.clone();
        std::thread::spawn(move || {
            while let Ok(content) = rx.recv() {
                // 发送事件到前端
                let _ = app.emit("clipboard-changed", content.clone());
            }
        });

        Self { watcher, app_handle }
    }

    pub fn start(&mut self) {
        let handler = Handler { tx: self.tx.clone() };
        self.watcher.add_handler(handler).start_watch();
    }

    pub fn stop(&mut self) {
        self.watcher.stop();
    }
}

/// Tauri 命令：读取当前剪贴板
#[tauri::command]
pub fn read_clipboard() -> Result<ClipboardContent, String> {
    let ctx = ClipboardContext::new().map_err(|e| e.to_string())?;

    if let Ok(files) = ctx.get_files() {
        return Ok(ClipboardContent::File(files));
    }
    if let Ok(img) = ctx.get_image() {
        let png = img.to_png().map_err(|e| e.to_string())?;
        return Ok(ClipboardContent::Image(png.get_bytes().to_vec()));
    }
    if let Ok(text) = ctx.get_text() {
        return Ok(ClipboardContent::Text(text));
    }

    Err("Clipboard is empty or unsupported format".into())
}

/// Tauri 命令：写入剪贴板
#[tauri::command]
pub fn write_clipboard(content: ClipboardContent) -> Result<(), String> {
    let ctx = ClipboardContext::new().map_err(|e| e.to_string())?;

    match content {
        ClipboardContent::Text(text) => {
            ctx.set_text(text).map_err(|e| e.to_string())
        }
        ClipboardContent::Image(data) => {
            let img = clipboard_rs::RustImageData::from_bytes(&data)
                .map_err(|e| e.to_string())?;
            ctx.set_image(img).map_err(|e| e.to_string())
        }
        ClipboardContent::File(paths) => {
            ctx.set_files(paths).map_err(|e| e.to_string())
        }
    }
}
```

---

### 3.2 系统托盘 (tray.rs)

#### 3.2.1 功能描述
- 显示应用图标和同步状态
- 右键菜单操作
- 点击打开/关闭主窗口

#### 3.2.2 托盘菜单

```
┌─────────────────────┐
│  ✓ 自动同步         │  ← 开关
│  ─────────────────  │
│  🔄 立即同步        │
│  📋 查看历史        │  ← 打开主窗口
│  ─────────────────  │
│  ⚙️  设置           │
│  ─────────────────  │
│  🚪 退出            │
└─────────────────────┘
```

#### 3.2.3 实现方案

```rust
// desktop/src-tauri/src/tray.rs

use tauri::{
    AppHandle, Manager,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    menu::{Menu, MenuItem, PredefinedMenuItem, CheckMenuItem},
};

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    // 创建菜单项
    let auto_sync = CheckMenuItem::with_id(app, "auto_sync", "自动同步", true, true, None::<&str>)?;
    let sync_now = MenuItem::with_id(app, "sync_now", "立即同步", true, None::<&str>)?;
    let show_history = MenuItem::with_id(app, "show_history", "查看历史", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[
        &auto_sync,
        &PredefinedMenuItem::separator(app)?,
        &sync_now,
        &show_history,
        &PredefinedMenuItem::separator(app)?,
        &settings,
        &PredefinedMenuItem::separator(app)?,
        &quit,
    ])?;

    let tray = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("GCopy - 剪贴板同步")
        .on_menu_event(move |app, event| {
            match event.id.as_ref() {
                "auto_sync" => {
                    // 切换自动同步状态
                    let state = app.state::<SyncState>();
                    state.toggle_auto_sync();
                }
                "sync_now" => {
                    // 触发立即同步
                    let _ = app.emit("sync-now", ());
                }
                "show_history" => {
                    // 显示主窗口
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "settings" => {
                    // 打开设置页面
                    let _ = app.emit("open-settings", ());
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            // 点击托盘图标切换窗口显示
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
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

    Ok(())
}

/// 更新托盘图标状态
pub fn update_tray_icon(app: &AppHandle, status: SyncStatus) {
    if let Some(tray) = app.tray_by_id("main") {
        let icon_path = match status {
            SyncStatus::Idle => "icons/tray-idle.png",
            SyncStatus::Syncing => "icons/tray-syncing.png",
            SyncStatus::Error => "icons/tray-error.png",
        };
        // 更新图标逻辑
    }
}

#[derive(Clone, Copy)]
pub enum SyncStatus {
    Idle,
    Syncing,
    Error,
}
```

---

### 3.3 自动同步 (sync.rs)

#### 3.3.1 同步策略

```
┌─────────────────────────────────────────────────────────────┐
│                      同步流程图                              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐    剪贴板变化     ┌──────────────────┐    │
│  │ 剪贴板监控器  │ ───────────────► │  本地队列(Dexie)  │    │
│  └──────────────┘                   └────────┬─────────┘    │
│                                              │              │
│                                              ▼              │
│  ┌──────────────┐    定时触发       ┌──────────────────┐    │
│  │  定时器(3s)   │ ───────────────► │   同步引擎        │    │
│  └──────────────┘                   └────────┬─────────┘    │
│                                              │              │
│                                    ┌─────────┴─────────┐    │
│                                    ▼                   ▼    │
│                           ┌──────────────┐   ┌─────────────┐│
│                           │ Push (上传)   │   │ Pull (下载) ││
│                           └──────┬───────┘   └──────┬──────┘│
│                                  │                  │       │
│                                  ▼                  ▼       │
│                           ┌─────────────────────────────────┐
│                           │     POST/GET /api/v1/clipboard  │
│                           └─────────────────────────────────┘
└─────────────────────────────────────────────────────────────┘
```

#### 3.3.2 冲突解决

- **策略**：Last Write Wins (后写优先)
- **依据**：服务端 `x-index` 索引递增，取最大值
- **原因**：GCopy 数据 24 小时过期，无需复杂合并

#### 3.3.3 实现方案

```rust
// desktop/src-tauri/src/sync.rs

use reqwest::Client;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;

/// 同步状态
pub struct SyncState {
    auto_sync_enabled: AtomicBool,
    last_local_index: AtomicU64,
    last_server_index: AtomicU64,
    is_syncing: AtomicBool,
    client: Client,
    server_url: String,
}

impl SyncState {
    pub fn new(server_url: String) -> Self {
        Self {
            auto_sync_enabled: AtomicBool::new(true),
            last_local_index: AtomicU64::new(0),
            last_server_index: AtomicU64::new(0),
            is_syncing: AtomicBool::new(false),
            client: Client::new(),
            server_url,
        }
    }

    pub fn toggle_auto_sync(&self) -> bool {
        let current = self.auto_sync_enabled.load(Ordering::SeqCst);
        self.auto_sync_enabled.store(!current, Ordering::SeqCst);
        !current
    }
}

/// 同步引擎
pub struct SyncEngine {
    app: AppHandle,
    state: Arc<SyncState>,
}

impl SyncEngine {
    pub fn new(app: AppHandle, state: Arc<SyncState>) -> Self {
        Self { app, state }
    }

    /// 启动后台同步任务
    pub fn start(&self) {
        let app = self.app.clone();
        let state = self.state.clone();

        // 定时同步 (每 3 秒)
        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3));
            loop {
                interval.tick().await;

                if !state.auto_sync_enabled.load(Ordering::SeqCst) {
                    continue;
                }

                if state.is_syncing.swap(true, Ordering::SeqCst) {
                    continue; // 已在同步中
                }

                // 先尝试 pull
                if let Err(e) = Self::pull(&app, &state).await {
                    log::error!("Pull failed: {}", e);
                }

                state.is_syncing.store(false, Ordering::SeqCst);
            }
        });
    }

    /// 从服务器拉取最新数据
    async fn pull(app: &AppHandle, state: &SyncState) -> Result<(), String> {
        let current_index = state.last_server_index.load(Ordering::SeqCst);

        let resp = state.client
            .get(&format!("{}/api/v1/clipboard", state.server_url))
            .header("X-Index", current_index.to_string())
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.status() == 304 {
            // 无新数据
            return Ok(());
        }

        if !resp.status().is_success() {
            return Err(format!("Server error: {}", resp.status()));
        }

        // 解析响应头
        let new_index: u64 = resp.headers()
            .get("x-index")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let content_type = resp.headers()
            .get("x-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text");

        // 获取数据
        let data = resp.bytes().await.map_err(|e| e.to_string())?;

        // 写入系统剪贴板
        let content = match content_type {
            "text" => ClipboardContent::Text(String::from_utf8_lossy(&data).to_string()),
            "screenshot" => ClipboardContent::Image(data.to_vec()),
            "file" => {
                // 文件需要先下载到临时目录
                // TODO: 实现文件处理
                return Ok(());
            }
            _ => return Ok(()),
        };

        crate::clipboard::write_clipboard(content)?;

        // 更新索引
        state.last_server_index.store(new_index, Ordering::SeqCst);

        // 通知前端
        let _ = app.emit("sync-completed", SyncEvent::Pulled);

        Ok(())
    }

    /// 推送本地剪贴板到服务器
    pub async fn push(app: &AppHandle, state: &SyncState, content: ClipboardContent) -> Result<(), String> {
        let (data, content_type, filename) = match &content {
            ClipboardContent::Text(text) => (text.as_bytes().to_vec(), "text", None),
            ClipboardContent::Image(img) => (img.clone(), "screenshot", None),
            ClipboardContent::File(paths) => {
                // 读取第一个文件
                if let Some(path) = paths.first() {
                    let data = std::fs::read(path).map_err(|e| e.to_string())?;
                    let filename = std::path::Path::new(path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string());
                    (data, "file", filename)
                } else {
                    return Ok(());
                }
            }
        };

        let mut req = state.client
            .post(&format!("{}/api/v1/clipboard", state.server_url))
            .header("Content-Type", "application/octet-stream")
            .header("X-Type", content_type);

        if let Some(name) = filename {
            req = req.header("X-FileName", urlencoding::encode(&name).to_string());
        }

        let resp = req.body(data)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("Server error: {}", resp.status()));
        }

        // 更新本地索引
        if let Some(index) = resp.headers()
            .get("x-index")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
        {
            state.last_local_index.store(index, Ordering::SeqCst);
        }

        // 通知前端
        let _ = app.emit("sync-completed", SyncEvent::Pushed);

        Ok(())
    }
}

#[derive(Clone, serde::Serialize)]
pub enum SyncEvent {
    Pulled,
    Pushed,
}
```

---

### 3.4 开机自启 (autostart)

#### 3.4.1 使用 tauri-plugin-autostart

```toml
# desktop/src-tauri/Cargo.toml
[dependencies]
tauri-plugin-autostart = "2"
```

```rust
// desktop/src-tauri/src/main.rs
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]) // 启动参数
        ))
        .invoke_handler(tauri::generate_handler![
            // 命令
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

```typescript
// desktop/src/hooks/useAutostart.ts
import { enable, isEnabled, disable } from '@tauri-apps/plugin-autostart';

export function useAutostart() {
  const [enabled, setEnabled] = useState(false);

  useEffect(() => {
    isEnabled().then(setEnabled);
  }, []);

  const toggle = async () => {
    if (enabled) {
      await disable();
    } else {
      await enable();
    }
    setEnabled(!enabled);
  };

  return { enabled, toggle };
}
```

---

### 3.5 配置管理 (config.rs)

```rust
// desktop/src-tauri/src/config.rs

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 服务器地址
    pub server_url: String,

    /// 自动同步开关
    pub auto_sync: bool,

    /// 同步间隔 (秒)
    pub sync_interval: u64,

    /// 开机自启
    pub auto_start: bool,

    /// 快捷键设置
    pub shortcuts: Shortcuts,

    /// 同步内容类型
    pub sync_types: SyncTypes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shortcuts {
    pub manual_sync: String,      // 默认: "CmdOrCtrl+Shift+V"
    pub toggle_window: String,    // 默认: "CmdOrCtrl+Shift+G"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncTypes {
    pub text: bool,
    pub screenshot: bool,
    pub file: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_url: "https://gcopy.llaoj.cn".into(),
            auto_sync: true,
            sync_interval: 3,
            auto_start: false,
            shortcuts: Shortcuts {
                manual_sync: "CmdOrCtrl+Shift+V".into(),
                toggle_window: "CmdOrCtrl+Shift+G".into(),
            },
            sync_types: SyncTypes {
                text: true,
                screenshot: true,
                file: true,
            },
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            let content = fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, content).map_err(|e| e.to_string())
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gcopy")
            .join("config.json")
    }
}

/// Tauri 命令
#[tauri::command]
pub fn get_config() -> AppConfig {
    AppConfig::load()
}

#[tauri::command]
pub fn save_config(config: AppConfig) -> Result<(), String> {
    config.save()
}
```

---

## 4. 前端改造

### 4.1 从 Next.js 迁移到纯 React

由于 Tauri 需要静态文件，需将 Next.js 改为 Vite + React：

```json
// desktop/package.json
{
  "name": "gcopy-desktop",
  "version": "1.0.0",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "tauri": "tauri"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "@tauri-apps/plugin-autostart": "^2.0.0",
    "dexie": "^4.0.0",
    "dexie-react-hooks": "^1.1.0",
    "react": "^18.2.0",
    "react-dom": "^18.2.0"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.0.0",
    "@vitejs/plugin-react": "^4.0.0",
    "typescript": "^5.0.0",
    "vite": "^5.0.0"
  }
}
```

### 4.2 可复用的组件

| 原文件 | 复用度 | 改动说明 |
|--------|--------|----------|
| `components/sync-clipboard.tsx` | 70% | 移除浏览器 Clipboard API，改用 Tauri IPC |
| `components/history.tsx` | 100% | 完全复用 |
| `components/history-item.tsx` | 90% | 剪贴板写入改用 Tauri API |
| `lib/clipboard.ts` | 重写 | 改为调用 Rust 命令 |
| `models/db.ts` | 100% | 完全复用 (Dexie) |
| `lib/auth.ts` | 90% | API 地址可配置 |

### 4.3 Tauri API 封装

```typescript
// desktop/src/lib/clipboard.ts

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface ClipboardContent {
  type: 'text' | 'image' | 'file';
  data: string | number[] | string[];
}

// 读取系统剪贴板
export async function readClipboard(): Promise<ClipboardContent> {
  return invoke('read_clipboard');
}

// 写入系统剪贴板
export async function writeClipboard(content: ClipboardContent): Promise<void> {
  return invoke('write_clipboard', { content });
}

// 监听剪贴板变化
export function onClipboardChange(callback: (content: ClipboardContent) => void) {
  return listen<ClipboardContent>('clipboard-changed', (event) => {
    callback(event.payload);
  });
}
```

---

## 5. 构建与发布

### 5.1 Tauri 配置

```json
// desktop/src-tauri/tauri.conf.json
{
  "$schema": "https://v2.tauri.app/schema.json",
  "productName": "GCopy",
  "version": "1.0.0",
  "identifier": "cn.llaoj.gcopy",
  "build": {
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "GCopy",
        "width": 400,
        "height": 600,
        "resizable": true,
        "visible": false,
        "decorations": true
      }
    ],
    "trayIcon": {
      "iconPath": "icons/icon.png",
      "iconAsTemplate": true
    }
  },
  "bundle": {
    "active": true,
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "targets": ["dmg", "nsis"],
    "macOS": {
      "minimumSystemVersion": "10.13"
    },
    "windows": {
      "nsis": {
        "installMode": "currentUser"
      }
    }
  },
  "plugins": {
    "autostart": {
      "macOSLauncher": "LaunchAgent",
      "args": ["--minimized"]
    }
  }
}
```

### 5.2 构建命令

```bash
# 开发模式
cd desktop
npm run tauri dev

# 生产构建
npm run tauri build

# 输出目录
# Mac: desktop/src-tauri/target/release/bundle/dmg/GCopy_1.0.0_x64.dmg
# Windows: desktop/src-tauri/target/release/bundle/nsis/GCopy_1.0.0_x64-setup.exe
```

### 5.3 CI/CD (GitHub Actions)

```yaml
# .github/workflows/desktop-release.yml
name: Desktop Release

on:
  push:
    tags:
      - 'desktop-v*'

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: 20

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install dependencies
        working-directory: desktop
        run: npm ci

      - name: Build Tauri
        uses: tauri-apps/tauri-action@v0
        with:
          projectPath: desktop
          tagName: desktop-v__VERSION__
          releaseName: 'GCopy Desktop v__VERSION__'
          releaseBody: 'See the assets to download.'
          releaseDraft: true
          prerelease: false
```

---

## 6. 安全考虑

### 6.1 权限配置

```json
// desktop/src-tauri/capabilities/default.json
{
  "$schema": "https://v2.tauri.app/schema.json",
  "identifier": "default",
  "description": "GCopy default permissions",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "autostart:default",
    "clipboard-manager:default",
    {
      "identifier": "http:default",
      "allow": [
        { "url": "https://gcopy.llaoj.cn/**" },
        { "url": "http://localhost:*/**" }
      ]
    }
  ]
}
```

### 6.2 安全要点

1. **数据传输**：仅允许 HTTPS 连接到服务器
2. **本地存储**：使用系统 Keychain/Credential Manager 存储敏感数据
3. **代码签名**：Mac 需 Apple Developer ID，Windows 需 EV 证书
4. **自动更新**：集成 Tauri updater 插件，使用签名验证

---

## 7. 开发计划

### Phase 1: 基础框架 (1-2 周)
- [ ] 初始化 Tauri 项目
- [ ] 迁移 React 组件到 Vite
- [ ] 实现基本窗口和托盘

### Phase 2: 核心功能 (2-3 周)
- [ ] 实现 Rust 剪贴板监控
- [ ] 实现自动同步引擎
- [ ] 集成认证流程

### Phase 3: 完善体验 (1-2 周)
- [ ] 实现开机自启
- [ ] 添加设置界面
- [ ] 快捷键支持

### Phase 4: 发布准备 (1 周)
- [ ] 图标和品牌资源
- [ ] 代码签名
- [ ] CI/CD 配置
- [ ] 文档更新

---

## 8. 参考资料

- [Tauri 2.0 官方文档](https://v2.tauri.app/)
- [Tauri 系统托盘指南](https://v2.tauri.app/learn/system-tray/)
- [clipboard-rs Crate](https://crates.io/crates/clipboard-rs)
- [tauri-plugin-autostart](https://v2.tauri.app/plugin/autostart/)
