mod plugins;

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

/// Render any file's native macOS icon (the one Finder shows) to PNG bytes via
/// NSWorkspace. Works for every app, including asset-catalog apps without .icns.
#[cfg(target_os = "macos")]
fn icon_png(path: &str) -> Option<Vec<u8>> {
    use core::ffi::c_void;
    use core::ptr::NonNull;
    use objc2::AnyThread;
    use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSWorkspace};
    use objc2_foundation::{NSData, NSDictionary, NSRange, NSString};

    objc2::rc::autoreleasepool(|_| unsafe {
        let workspace = NSWorkspace::sharedWorkspace();
        let image = workspace.iconForFile(&NSString::from_str(path));
        let tiff: objc2::rc::Retained<NSData> = image.TIFFRepresentation()?;
        let rep = NSBitmapImageRep::initWithData(NSBitmapImageRep::alloc(), &tiff)?;
        let props = NSDictionary::new();
        let png = rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &props)?;
        let len = png.length();
        if len == 0 {
            return None;
        }
        let mut buf = vec![0u8; len];
        png.getBytes_range(
            NonNull::new(buf.as_mut_ptr() as *mut c_void)?,
            NSRange::new(0, len),
        );
        Some(buf)
    })
}

fn icon_cache_path(app_path: &str) -> Option<PathBuf> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    app_path.hash(&mut h);
    Some(
        dirs::cache_dir()?
            .join("pc-tool/icons")
            .join(format!("{:x}.png", h.finish())),
    )
}

/// Decode a (large) PNG and re-encode it at 64x64 so icons stay small in
/// memory and cheap to transfer to the webview.
#[cfg(target_os = "macos")]
fn downscale_png(png: &[u8]) -> Option<Vec<u8>> {
    let img = image::load_from_memory(png).ok()?;
    let small = img.resize(64, 64, image::imageops::FilterType::Triangle);
    let mut out = Vec::new();
    small
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .ok()?;
    Some(out)
}

fn icon_data_url(path: &str) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let cache = icon_cache_path(path);
        // Fast path: a previously rendered 64px icon on disk.
        if let Some(c) = &cache {
            if let Ok(bytes) = std::fs::read(c) {
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                return Some(format!("data:image/png;base64,{b64}"));
            }
        }
        let full = icon_png(path)?;
        let small = downscale_png(&full).unwrap_or(full);
        if let Some(c) = &cache {
            if let Some(parent) = c.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(c, &small);
        }
        let b64 = base64::engine::general_purpose::STANDARD.encode(&small);
        Some(format!("data:image/png;base64,{b64}"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        None
    }
}

/// Extract an app's icon as a PNG data URL. Runs off the main thread
/// (`spawn_blocking`) — icon rendering is ~100ms each and would otherwise
/// freeze the UI, since synchronous Tauri commands run on the main thread.
#[tauri::command]
async fn app_icon(path: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || icon_data_url(&path))
        .await
        .ok()
        .flatten()
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

/// Return the file paths currently on the clipboard (empty if none).
#[cfg(target_os = "macos")]
#[tauri::command]
fn clipboard_read_files() -> Vec<String> {
    use objc2::ClassType;
    use objc2_app_kit::NSPasteboard;
    use objc2_foundation::{NSArray, NSURL};

    objc2::rc::autoreleasepool(|_| unsafe {
        let pb = NSPasteboard::generalPasteboard();
        let cls_array = NSArray::from_slice(&[NSURL::class()]);
        let Some(objects) = pb.readObjectsForClasses_options(&cls_array, None) else {
            return Vec::new();
        };
        let mut paths = Vec::new();
        for i in 0..objects.count() {
            let obj = objects.objectAtIndex(i);
            // SAFETY: we asked only for NSURL objects, so each element is NSURL.
            if let Ok(url) = obj.downcast::<NSURL>() {
                if url.isFileURL() {
                    if let Some(p) = url.path() {
                        paths.push(p.to_string());
                    }
                }
            }
        }
        paths
    })
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn clipboard_read_files() -> Vec<String> {
    Vec::new()
}

/// Write file paths to the clipboard as files (Cmd+V in Finder pastes them).
#[cfg(target_os = "macos")]
#[tauri::command]
fn clipboard_write_files(paths: Vec<String>) -> Result<(), String> {
    use objc2_app_kit::{NSPasteboard, NSPasteboardWriting};
    use objc2_foundation::{NSArray, NSString, NSURL};

    objc2::rc::autoreleasepool(|_| {
        let pb = NSPasteboard::generalPasteboard();
        pb.clearContents();
        let urls: Vec<objc2::rc::Retained<NSURL>> = paths
            .iter()
            .map(|p| NSURL::fileURLWithPath(&NSString::from_str(p)))
            .collect();
        // Cast each NSURL to a protocol object implementing NSPasteboardWriting.
        let writing: Vec<objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn NSPasteboardWriting>>> = urls
            .iter()
            .map(|u| objc2::runtime::ProtocolObject::from_retained(u.clone()))
            .collect();
        let array = NSArray::from_retained_slice(&writing);
        if pb.writeObjects(&array) {
            Ok(())
        } else {
            Err("writeObjects returned false".into())
        }
    })
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn clipboard_write_files(_paths: Vec<String>) -> Result<(), String> {
    Err("not supported on this platform".into())
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

#[tauri::command]
fn hosts_read() -> Result<String, String> {
    std::fs::read_to_string("/etc/hosts").map_err(|e| e.to_string())
}

/// Write /etc/hosts (root-owned) via a macOS admin auth prompt. Runs off the
/// main thread because the auth dialog is modal and blocking.
#[tauri::command]
async fn hosts_write(content: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let tmp = std::env::temp_dir().join("pc-tool-hosts.tmp");
        std::fs::write(&tmp, content.as_bytes()).map_err(|e| e.to_string())?;
        let script = format!(
            "do shell script \"cat '{}' > /etc/hosts\" with administrator privileges",
            tmp.to_string_lossy()
        );
        let status = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .status()
            .map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(&tmp);
        if status.success() {
            Ok(())
        } else {
            Err("已取消或写入失败".into())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

// ---------------------------------------------------------------------------
// Filesystem — SCOPED access. A plugin can only touch directories it declared
// and the user granted (downloads/desktop/documents/temp/home), plus its own
// private `plugin` dir. Every path is canonicalized and checked to be inside a
// granted scope root, defeating `..` / symlink escapes.
// ---------------------------------------------------------------------------

fn fs_scope_roots(plugin_id: &str) -> Vec<(&'static str, PathBuf)> {
    let home = dirs::home_dir().unwrap_or_default();
    let plugin = dirs::config_dir()
        .unwrap_or_default()
        .join("pc-tool/plugin-data")
        .join(plugin_id)
        .join("files");
    vec![
        ("plugin", plugin),
        ("downloads", home.join("Downloads")),
        ("desktop", home.join("Desktop")),
        ("documents", home.join("Documents")),
        ("temp", std::env::temp_dir()),
        ("home", home),
    ]
}

/// Resolve `path` to an absolute, symlink-free path. Rejects relative paths and
/// any `..` component up front; for a not-yet-existing leaf, canonicalizes the
/// deepest existing ancestor and re-appends the (normal-only) tail.
fn fs_safe_resolve(path: &str) -> Option<PathBuf> {
    let p = Path::new(path);
    if !p.is_absolute() {
        return None;
    }
    if p.components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return None;
    }
    if let Ok(c) = std::fs::canonicalize(p) {
        return Some(c);
    }
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let mut cur = p.to_path_buf();
    loop {
        if let Ok(base) = std::fs::canonicalize(&cur) {
            let mut result = base;
            for c in tail.iter().rev() {
                result.push(c);
            }
            return Some(result);
        }
        tail.push(cur.file_name()?.to_os_string());
        cur = cur.parent()?.to_path_buf();
    }
}

/// Authorize an fs operation: the path must resolve inside a scope root, and
/// (except for the plugin's own dir) the plugin must have the matching
/// `fs.<scope>.read` / `fs.<scope>.write` permission granted.
fn fs_guard(path: &str, plugin_id: &str, granted: &[String], write: bool) -> Result<PathBuf, String> {
    let target = fs_safe_resolve(path).ok_or("无效或越界的路径")?;
    for (name, root) in fs_scope_roots(plugin_id) {
        let root_c = std::fs::canonicalize(&root).unwrap_or(root);
        if target.starts_with(&root_c) {
            if name == "plugin" {
                let _ = std::fs::create_dir_all(&root_c);
                return Ok(target);
            }
            let perm = format!("fs.{}.{}", name, if write { "write" } else { "read" });
            return if granted.iter().any(|g| g == &perm) {
                Ok(target)
            } else {
                Err(format!("permission denied: {perm}"))
            };
        }
    }
    Err("路径不在任何已授权目录内".into())
}

#[tauri::command]
fn fs_scopes(granted: Vec<String>, plugin_id: String) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    for (name, root) in fs_scope_roots(&plugin_id) {
        let allowed = name == "plugin"
            || granted
                .iter()
                .any(|g| g == &format!("fs.{name}.read") || g == &format!("fs.{name}.write"));
        if allowed {
            if name == "plugin" {
                let _ = std::fs::create_dir_all(&root);
            }
            out.insert(name.to_string(), root.to_string_lossy().to_string());
        }
    }
    out
}

#[derive(Serialize)]
struct FsEntry {
    name: String,
    path: String,
    is_dir: bool,
}

#[tauri::command]
fn fs_list(path: String, granted: Vec<String>, plugin_id: String) -> Result<Vec<FsEntry>, String> {
    let dir = fs_guard(&path, &plugin_id, &granted, false)?;
    let mut out = Vec::new();
    for e in std::fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten() {
        let p = e.path();
        out.push(FsEntry {
            name: e.file_name().to_string_lossy().to_string(),
            path: p.to_string_lossy().to_string(),
            is_dir: p.is_dir(),
        });
    }
    Ok(out)
}

#[derive(Serialize)]
struct FsStat {
    is_dir: bool,
    is_file: bool,
    size: u64,
}

#[tauri::command]
fn fs_stat(path: String, granted: Vec<String>, plugin_id: String) -> Result<FsStat, String> {
    let p = fs_guard(&path, &plugin_id, &granted, false)?;
    let m = std::fs::metadata(&p).map_err(|e| e.to_string())?;
    Ok(FsStat { is_dir: m.is_dir(), is_file: m.is_file(), size: m.len() })
}

#[tauri::command]
fn fs_exists(path: String, granted: Vec<String>, plugin_id: String) -> Result<bool, String> {
    let p = fs_guard(&path, &plugin_id, &granted, false)?;
    Ok(p.exists())
}

#[tauri::command]
async fn fs_read_text(path: String, granted: Vec<String>, plugin_id: String) -> Result<String, String> {
    let p = fs_guard(&path, &plugin_id, &granted, false)?;
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::read_to_string(&p).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn fs_read_bytes(path: String, granted: Vec<String>, plugin_id: String) -> Result<String, String> {
    let p = fs_guard(&path, &plugin_id, &granted, false)?;
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = std::fs::read(&p).map_err(|e| e.to_string())?;
        Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn fs_write_text(
    path: String,
    content: String,
    granted: Vec<String>,
    plugin_id: String,
) -> Result<(), String> {
    let p = fs_guard(&path, &plugin_id, &granted, true)?;
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::write(&p, content.as_bytes()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn fs_write_bytes(
    path: String,
    base64_data: String,
    granted: Vec<String>,
    plugin_id: String,
) -> Result<(), String> {
    let p = fs_guard(&path, &plugin_id, &granted, true)?;
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&base64_data)
            .map_err(|e| e.to_string())?;
        std::fs::write(&p, &bytes).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn fs_mkdir(path: String, granted: Vec<String>, plugin_id: String) -> Result<(), String> {
    let p = fs_guard(&path, &plugin_id, &granted, true)?;
    std::fs::create_dir_all(&p).map_err(|e| e.to_string())
}

#[tauri::command]
fn fs_remove(path: String, granted: Vec<String>, plugin_id: String) -> Result<(), String> {
    let p = fs_guard(&path, &plugin_id, &granted, true)?;
    let r = if p.is_dir() {
        std::fs::remove_dir_all(&p)
    } else {
        std::fs::remove_file(&p)
    };
    r.map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Notification (gated by notification permission)
// ---------------------------------------------------------------------------

fn osa_quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

#[tauri::command]
fn notify(title: String, body: String) -> Result<(), String> {
    let script = format!(
        "display notification {} with title {}",
        osa_quote(&body),
        osa_quote(&title)
    );
    Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .status()
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Network (gated by network permission) — host-side HTTP, bypasses CORS
// ---------------------------------------------------------------------------

#[tauri::command]
async fn http_request(
    url: String,
    method: Option<String>,
    headers: Option<std::collections::HashMap<String, String>>,
    body: Option<String>,
) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let method = method.unwrap_or_else(|| "GET".into());
        let mut req = ureq::request(&method, &url);
        if let Some(h) = &headers {
            for (k, v) in h {
                req = req.set(k, v);
            }
        }
        let result = match &body {
            Some(b) => req.send_string(b),
            None => req.call(),
        };
        let resp = match result {
            Ok(r) => r,
            Err(ureq::Error::Status(_, r)) => r,
            Err(e) => return Err(e.to_string()),
        };
        let status = resp.status();
        let text = resp.into_string().map_err(|e| e.to_string())?;
        Ok(serde_json::json!({ "status": status, "body": text }))
    })
    .await
    .map_err(|e| e.to_string())?
}

// ---------------------------------------------------------------------------
// Clipboard image (gated by clipboard.readImage / clipboard.writeImage)
// ---------------------------------------------------------------------------

#[tauri::command]
async fn clipboard_read_image() -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        match cb.get_image() {
            Ok(img) => {
                let w = img.width as u32;
                let h = img.height as u32;
                let buf = image::RgbaImage::from_raw(w, h, img.bytes.into_owned())
                    .ok_or("bad image data")?;
                let mut png = Vec::new();
                image::DynamicImage::ImageRgba8(buf)
                    .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
                    .map_err(|e| e.to_string())?;
                Ok(Some(format!(
                    "data:image/png;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(&png)
                )))
            }
            Err(_) => Ok(None),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn clipboard_write_image(data_url: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let b64 = data_url.split(',').nth(1).unwrap_or(&data_url);
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| e.to_string())?;
        let img = image::load_from_memory(&bytes)
            .map_err(|e| e.to_string())?
            .to_rgba8();
        let (w, h) = (img.width() as usize, img.height() as usize);
        let data = arboard::ImageData {
            width: w,
            height: h,
            bytes: std::borrow::Cow::Owned(img.into_raw()),
        };
        let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        cb.set_image(data).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

// ---------------------------------------------------------------------------
// Plugin loading — see plugins.rs
// ---------------------------------------------------------------------------

#[tauri::command]
fn hide_window(window: tauri::WebviewWindow) {
    let _ = window.hide();
}

/// When false, the window stays visible on blur. Disabled during plugin
/// management / file dialogs, which legitimately move focus elsewhere (e.g.
/// dragging a file from Finder, or the native open-file dialog).
struct AutoHide(std::sync::atomic::AtomicBool);

#[tauri::command]
fn set_auto_hide(enabled: bool, state: tauri::State<AutoHide>) {
    state
        .0
        .store(enabled, std::sync::atomic::Ordering::Relaxed);
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

fn test_mode() -> bool {
    cfg!(debug_assertions) && std::env::var_os("PC_TOOL_TEST").is_some()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let toggle = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Space);

    tauri::Builder::default()
        .manage(AutoHide(std::sync::atomic::AtomicBool::new(true)))
        .plugin(tauri_plugin_dialog::init())
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

            // Warm the on-disk icon cache in the background (off the webview,
            // which throttles JS while the window is hidden). A few threads
            // render every app's icon once so later lookups are instant.
            std::thread::spawn(|| {
                let paths: std::sync::Arc<Vec<String>> =
                    std::sync::Arc::new(list_apps().into_iter().map(|a| a.path).collect());
                let mut handles = Vec::new();
                for t in 0..4usize {
                    let paths = paths.clone();
                    handles.push(std::thread::spawn(move || {
                        let mut i = t;
                        while i < paths.len() {
                            let _ = icon_data_url(&paths[i]);
                            i += 4;
                        }
                    }));
                }
                for h in handles {
                    let _ = h.join();
                }
            });

            let test_mode = test_mode();
            if test_mode {
                app.state::<AutoHide>()
                    .0
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.navigate(tauri::Url::parse("http://localhost:1420/test").unwrap());
                    let _ = w.set_title("pc-tool test");
                    let _ = w.set_decorations(true);
                    let _ = w.set_shadow(true);
                    let _ = w.set_always_on_top(false);
                    let _ = w.set_skip_taskbar(false);
                    let _ = w.set_size(tauri::LogicalSize::new(680.0, 560.0));
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }

            // macOS: behave as a tray/agent app — no Dock icon.
            #[cfg(target_os = "macos")]
            if !test_mode {
                let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            }

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
                if window
                    .state::<AutoHide>()
                    .0
                    .load(std::sync::atomic::Ordering::Relaxed)
                {
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_apps,
            launch_app,
            app_icon,
            clipboard_read,
            clipboard_write,
            clipboard_read_files,
            clipboard_write_files,
            open_url,
            open_path,
            hosts_read,
            hosts_write,
            hide_window,
            set_auto_hide,
            plugins::list_plugins,
            plugins::read_plugin_file,
            plugins::read_plugin_icon,
            plugins::pack_plugin,
            plugins::inspect_package,
            plugins::install_package,
            plugins::uninstall_plugin,
            plugins::list_installed,
            plugins::download_package,
            plugins::list_registries,
            plugins::add_registry,
            plugins::remove_registry,
            plugins::fetch_registry,
            plugins::download_package_checked,
            fs_scopes,
            fs_list,
            fs_stat,
            fs_exists,
            fs_read_text,
            fs_read_bytes,
            fs_write_text,
            fs_write_bytes,
            fs_mkdir,
            fs_remove,
            notify,
            http_request,
            clipboard_read_image,
            clipboard_write_image,
            plugins::plugin_storage_get,
            plugins::plugin_storage_set,
            plugins::plugin_storage_remove,
            plugins::plugin_storage_keys,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_read_works() {
        let h = hosts_read().expect("read /etc/hosts");
        assert!(!h.is_empty(), "/etc/hosts is empty?");
    }

    #[test]
    fn fs_guard_enforces_scope_and_permission() {
        let pid = "com.test.fsguard";
        let dl = dirs::home_dir().unwrap().join("Downloads");
        std::fs::create_dir_all(&dl).ok();
        let f = dl.join("pctool-fsguard-test.txt");
        std::fs::write(&f, "x").ok();
        let fp = f.to_string_lossy().to_string();

        // read in Downloads WITH the read grant -> ok
        let granted = vec!["fs.downloads.read".to_string()];
        assert!(fs_guard(&fp, pid, &granted, false).is_ok());
        // write in Downloads WITHOUT a write grant -> denied
        assert!(fs_guard(&fp, pid, &granted, true).is_err());
        // a path outside every scope (/etc/hosts) -> denied
        assert!(fs_guard("/etc/hosts", pid, &granted, false).is_err());
        // a `..` escape attempt -> denied
        let escape = format!("{}/../.ssh/id_rsa", dl.to_string_lossy());
        assert!(fs_guard(&escape, pid, &granted, false).is_err());
        // the plugin's own dir -> allowed with NO permission
        let pdir = dirs::config_dir()
            .unwrap()
            .join("pc-tool/plugin-data")
            .join(pid)
            .join("files");
        std::fs::create_dir_all(&pdir).ok();
        let pf = pdir.join("data.json").to_string_lossy().to_string();
        assert!(fs_guard(&pf, pid, &[], false).is_ok());
        assert!(fs_guard(&pf, pid, &[], true).is_ok());

        let _ = std::fs::remove_file(&f);
    }

    #[test]
    fn extracts_icon_from_real_apps() {
        // At least one app under the system app dirs should yield a PNG icon.
        let apps = list_apps();
        assert!(!apps.is_empty(), "no apps found");
        let got = apps
            .iter()
            .take(40)
            .filter_map(|a| icon_data_url(&a.path))
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

    #[test]
    fn time_icons() {
        for a in list_apps().iter().take(8) {
            let t = std::time::Instant::now();
            let r = icon_data_url(&a.path);
            let bytes = r.as_ref().map(|u| u.len()).unwrap_or(0);
            eprintln!(
                "[total] {}ms b64len={} {}",
                t.elapsed().as_millis(),
                bytes,
                a.name
            );
        }
    }
}
