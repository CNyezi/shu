use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;
use tauri::{Emitter, Manager, WindowEvent};
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
};

// ---------------------------------------------------------------------------
// App launcher (built-in core capability — NOT exposed to plugins)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct AppEntry {
    name: String,
    path: String,
}

fn app_dirs() -> Vec<PathBuf> {
    let mut v = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
    ];
    if let Some(home) = dirs::home_dir() {
        v.push(home.join("Applications"));
    }
    v
}

fn collect_apps(dir: &Path, depth: usize, out: &mut Vec<AppEntry>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".app") {
            out.push(AppEntry {
                name: name.trim_end_matches(".app").to_string(),
                path: path.to_string_lossy().to_string(),
            });
        } else if depth > 0 && path.is_dir() {
            collect_apps(&path, depth - 1, out);
        }
    }
}

#[tauri::command]
fn list_apps() -> Vec<AppEntry> {
    let mut out = Vec::new();
    for dir in app_dirs() {
        collect_apps(&dir, 1, &mut out);
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out.dedup_by(|a, b| a.path == b.path);
    out
}

#[tauri::command]
fn launch_app(path: String) -> Result<(), String> {
    Command::new("open")
        .arg(&path)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// System capabilities (called only by the host shell, after whitelist check)
// ---------------------------------------------------------------------------

#[tauri::command]
fn clipboard_read() -> serde_json::Value {
    match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
        Ok(text) => serde_json::json!({ "kind": "text", "text": text }),
        Err(_) => serde_json::json!({ "kind": "empty", "text": "" }),
    }
}

#[tauri::command]
fn clipboard_write(text: String) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut c| c.set_text(text))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    Command::new("open")
        .arg(&url)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    Command::new("open")
        .arg(&path)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Plugin loading
// ---------------------------------------------------------------------------

fn plugins_dir() -> PathBuf {
    #[cfg(debug_assertions)]
    {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map(|p| p.join("plugins"))
            .unwrap_or_default()
    }
    #[cfg(not(debug_assertions))]
    {
        dirs::config_dir()
            .map(|c| c.join("pc-tool/plugins"))
            .unwrap_or_default()
    }
}

#[tauri::command]
fn list_plugins() -> Vec<serde_json::Value> {
    let dir = plugins_dir();
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return out;
    };
    for entry in rd.flatten() {
        let manifest = entry.path().join("plugin.json");
        let Ok(text) = std::fs::read_to_string(&manifest) else {
            continue;
        };
        if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(name) = entry.file_name().to_str() {
                v["_dir"] = serde_json::Value::String(name.to_string());
            }
            out.push(v);
        }
    }
    out
}

#[tauri::command]
fn read_plugin_file(dir: String, rel: String) -> Result<String, String> {
    if dir.contains("..") || dir.contains('/') || rel.contains("..") {
        return Err("invalid path".into());
    }
    let path = plugins_dir().join(&dir).join(&rel);
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn hide_window(window: tauri::WebviewWindow) {
    let _ = window.hide();
}

// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let toggle = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Space);

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if shortcut == &toggle && event.state() == ShortcutState::Pressed {
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                                let _ = w.emit("pc:shown", ());
                            }
                        }
                    }
                })
                .build(),
        )
        .setup(move |app| {
            app.global_shortcut().register(toggle)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::Focused(false) = event {
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_apps,
            launch_app,
            clipboard_read,
            clipboard_write,
            open_url,
            open_path,
            list_plugins,
            read_plugin_file,
            hide_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
