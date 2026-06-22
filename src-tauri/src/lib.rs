use std::path::{Path, PathBuf};
use std::process::Command;

use base64::Engine;
use serde::Serialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

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

/// Locate the `.icns` icon inside an `.app` bundle.
fn find_icns(app_path: &str) -> Option<PathBuf> {
    let resources = Path::new(app_path).join("Contents/Resources");
    let info = Path::new(app_path).join("Contents/Info.plist");
    if let Ok(plist::Value::Dictionary(dict)) = plist::Value::from_file(&info) {
        if let Some(name) = dict.get("CFBundleIconFile").and_then(|v| v.as_string()) {
            let mut p = resources.join(name);
            if p.extension().is_none() {
                p.set_extension("icns");
            }
            if p.exists() {
                return Some(p);
            }
        }
    }
    // Fallback: first .icns in Resources.
    for entry in std::fs::read_dir(&resources).ok()?.flatten() {
        let p = entry.path();
        if p.extension().map(|x| x == "icns").unwrap_or(false) {
            return Some(p);
        }
    }
    None
}

/// Extract an app's icon as a PNG data URL. Returns None if the bundle has no
/// `.icns` (e.g. apps using asset catalogs) — the UI falls back to a placeholder.
#[tauri::command]
fn app_icon(path: String) -> Option<String> {
    let icns_path = find_icns(&path)?;
    let file = std::io::BufReader::new(std::fs::File::open(&icns_path).ok()?);
    let family = icns::IconFamily::read(file).ok()?;
    let icon_type = family
        .available_icons()
        .into_iter()
        .max_by_key(|t| t.pixel_width() * t.pixel_height())?;
    let image = family.get_icon_with_type(icon_type).ok()?;
    let mut png: Vec<u8> = Vec::new();
    image.write_png(&mut png).ok()?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
    Some(format!("data:image/png;base64,{b64}"))
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

fn toggle_window(app: &AppHandle) {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let toggle = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Space);

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if shortcut == &toggle && event.state() == ShortcutState::Pressed {
                        toggle_window(app);
                    }
                })
                .build(),
        )
        .setup(move |app| {
            app.global_shortcut().register(toggle)?;

            // macOS: behave as a tray/agent app — no Dock icon.
            #[cfg(target_os = "macos")]
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // System tray icon + menu.
            let toggle_item = MenuItem::with_id(app, "toggle", "显示 / 隐藏", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出 pc-tool", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle_item, &quit_item])?;
            let mut builder = TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "toggle" => toggle_window(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_window(tray.app_handle());
                    }
                });
            if let Some(icon) = app.default_window_icon().cloned() {
                builder = builder.icon(icon);
            }
            builder.build(app)?;
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
            app_icon,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_icon_from_real_apps() {
        // At least one app under the system app dirs should yield a PNG icon.
        let apps = list_apps();
        assert!(!apps.is_empty(), "no apps found");
        let got = apps
            .iter()
            .take(40)
            .filter_map(|a| app_icon(a.path.clone()))
            .next();
        let url = got.expect("no icon extracted from any app");
        assert!(url.starts_with("data:image/png;base64,"), "bad data url");
        // sanity: decodes to a PNG (magic bytes)
        let b64 = url.trim_start_matches("data:image/png;base64,");
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .expect("bad base64");
        assert_eq!(&bytes[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
    }
}
