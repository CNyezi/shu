mod plugins;
mod win;

use std::path::{Path, PathBuf};
use std::process::Command;

use base64::Engine;
use serde::Serialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

// ---------------------------------------------------------------------------
// App launcher (built-in core capability — NOT exposed to plugins)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct AppEntry {
    name: String,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pinyin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    initials: Option<String>,
}

/// 用系统 CFStringTransform 把中文名转为拼音（全拼 + 首字母），无需词表依赖。
/// 纯 ASCII 名称返回 None。
#[cfg(target_os = "macos")]
fn pinyin_pair(name: &str) -> Option<(String, String)> {
    use std::ffi::c_void;
    type CFRef = *const c_void;
    #[repr(C)]
    struct CFRange {
        location: isize,
        length: isize,
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFStringCreateWithBytes(
            alloc: CFRef,
            bytes: *const u8,
            len: isize,
            encoding: u32,
            external: u8,
        ) -> CFRef;
        fn CFStringCreateMutableCopy(alloc: CFRef, max_len: isize, s: CFRef) -> CFRef;
        fn CFStringTransform(s: CFRef, range: *mut CFRange, transform: CFRef, reverse: u8) -> u8;
        fn CFStringGetLength(s: CFRef) -> isize;
        fn CFStringGetBytes(
            s: CFRef,
            range: CFRange,
            encoding: u32,
            loss_byte: u8,
            external: u8,
            buffer: *mut u8,
            max: isize,
            used: *mut isize,
        ) -> isize;
        fn CFRelease(r: CFRef);
        static kCFStringTransformMandarinLatin: CFRef;
        static kCFStringTransformStripDiacritics: CFRef;
    }
    const UTF8: u32 = 0x0800_0100; // kCFStringEncodingUTF8
    if name.is_ascii() {
        return None;
    }
    unsafe {
        let src = CFStringCreateWithBytes(
            std::ptr::null(),
            name.as_ptr(),
            name.len() as isize,
            UTF8,
            0,
        );
        if src.is_null() {
            return None;
        }
        let m = CFStringCreateMutableCopy(std::ptr::null(), 0, src);
        CFRelease(src);
        if m.is_null() {
            return None;
        }
        let ok = CFStringTransform(m, std::ptr::null_mut(), kCFStringTransformMandarinLatin, 0) != 0
            && CFStringTransform(m, std::ptr::null_mut(), kCFStringTransformStripDiacritics, 0) != 0;
        if !ok {
            CFRelease(m);
            return None;
        }
        let len = CFStringGetLength(m);
        let mut buf = vec![0u8; (len as usize) * 4];
        let mut used: isize = 0;
        CFStringGetBytes(
            m,
            CFRange { location: 0, length: len },
            UTF8,
            0,
            0,
            buf.as_mut_ptr(),
            buf.len() as isize,
            &mut used,
        );
        CFRelease(m);
        buf.truncate(used as usize);
        let s = String::from_utf8(buf).ok()?.to_lowercase();
        let syllables: Vec<&str> = s.split_whitespace().collect();
        let full: String = syllables.concat();
        let initials: String = syllables.iter().filter_map(|w| w.chars().next()).collect();
        Some((full, initials))
    }
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn pinyin_pair(_name: &str) -> Option<(String, String)> {
    None
}

#[cfg(not(target_os = "windows"))]
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

#[cfg(not(target_os = "windows"))]
fn collect_apps(dir: &Path, depth: usize, out: &mut Vec<AppEntry>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".app") {
            let clean = name.trim_end_matches(".app").to_string();
            let py = pinyin_pair(&clean);
            out.push(AppEntry {
                name: clean,
                path: path.to_string_lossy().to_string(),
                pinyin: py.as_ref().map(|(f, _)| f.clone()),
                initials: py.map(|(_, i)| i),
            });
        } else if depth > 0 && path.is_dir() {
            collect_apps(&path, depth - 1, out);
        }
    }
}

#[tauri::command]
fn list_apps() -> Vec<AppEntry> {
    #[cfg(target_os = "windows")]
    let mut out = win::discovery::list_apps();
    #[cfg(not(target_os = "windows"))]
    let mut out = {
        let mut v = Vec::new();
        for dir in app_dirs() {
            collect_apps(&dir, 1, &mut v);
        }
        v
    };
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out.dedup_by(|a, b| a.path == b.path);
    out
}

/// `open` 的错误（应用不存在等）发生在进程退出时，须等 status 才能上报给前端。
fn launch_app_blocking(path: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    return win::launch::shell_open(path);
    #[cfg(not(target_os = "windows"))]
    {
        let out = Command::new("open")
            .arg(path)
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            return Ok(());
        }
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if err.is_empty() {
            format!("open 退出码 {}", out.status.code().unwrap_or(-1))
        } else {
            err
        })
    }
}

#[tauri::command]
async fn launch_app(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || launch_app_blocking(&path))
        .await
        .map_err(|e| e.to_string())?
}

/// Render a file's native macOS Finder icon straight into a 64x64 bitmap, then
/// PNG-encode that small rep — skipping the 1024x1024 TIFF/PNG round-trip that
/// made cold renders cost ~0.5-1.3s each. Works for every app, including
/// asset-catalog apps without .icns.
#[cfg(target_os = "macos")]
fn icon_png(path: &str) -> Option<Vec<u8>> {
    use core::ffi::c_void;
    use core::ptr::NonNull;
    use objc2::AnyThread;
    use objc2_app_kit::{
        NSBitmapImageFileType, NSBitmapImageRep, NSCompositingOperation, NSDeviceRGBColorSpace,
        NSGraphicsContext, NSWorkspace,
    };
    use objc2_foundation::{NSDictionary, NSPoint, NSRange, NSRect, NSSize, NSString};

    objc2::rc::autoreleasepool(|_| unsafe {
        let workspace = NSWorkspace::sharedWorkspace();
        let image = workspace.iconForFile(&NSString::from_str(path));
        // 直接分配一张 64x64 的 RGBA 位图，把图标画进去，避免大图往返。
        let rep = NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            64, 64, 8, 4,
            true, false,
            NSDeviceRGBColorSpace,
            0, 0,
        )?;
        let ctx = NSGraphicsContext::graphicsContextWithBitmapImageRep(&rep)?;
        NSGraphicsContext::saveGraphicsState_class();
        NSGraphicsContext::setCurrentContext(Some(&ctx));
        image.drawInRect_fromRect_operation_fraction(
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(64.0, 64.0)),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0)), // NSZeroRect = 整幅
            // Copy 覆写全部目标像素，不依赖"新分配缓冲已清零"这一未成文保证。
            NSCompositingOperation::Copy,
            1.0,
        );
        NSGraphicsContext::restoreGraphicsState_class();
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
            .join("shu/icons")
            .join(format!("{:x}.png", h.finish())),
    )
}

#[cfg(target_os = "windows")]
fn icon_png(path: &str) -> Option<Vec<u8>> {
    win::icons::icon_png(path)
}

fn icon_data_url(path: &str) -> Option<String> {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        let cache = icon_cache_path(path);
        // Fast path: a previously rendered 64px icon on disk.
        if let Some(c) = &cache {
            if let Ok(bytes) = std::fs::read(c) {
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                return Some(format!("data:image/png;base64,{b64}"));
            }
        }
        let small = icon_png(path)?;
        if let Some(c) = &cache {
            if let Some(parent) = c.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(c, &small);
        }
        let b64 = base64::engine::general_purpose::STANDARD.encode(&small);
        Some(format!("data:image/png;base64,{b64}"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
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
    #[cfg(target_os = "windows")]
    return win::clipboard::read_files();
    #[cfg(not(target_os = "windows"))]
    Vec::new()
}

fn is_image_path(p: &str) -> bool {
    let l = p.to_lowercase();
    [".png", ".jpg", ".jpeg", ".gif", ".webp", ".tiff", ".bmp"]
        .iter()
        .any(|ext| l.ends_with(ext))
}

/// 剪贴板是否含图片——只探类型/文件扩展名，不解码像素（供 app 壳做内容推荐）。
/// 命中条件：复制的是图片文件，或剪贴板有位图类型（截图 / 复制的图片内容）。
#[cfg(target_os = "macos")]
#[tauri::command]
fn clipboard_image_present() -> bool {
    if clipboard_read_files().iter().any(|p| is_image_path(p)) {
        return true;
    }
    use objc2_app_kit::NSPasteboard;
    objc2::rc::autoreleasepool(|_| {
        let pb = NSPasteboard::generalPasteboard();
        let Some(types) = pb.types() else {
            return false;
        };
        for i in 0..types.count() {
            let t = types.objectAtIndex(i).to_string().to_lowercase();
            if t.contains("png") || t.contains("tiff") || t.contains("jpeg") || t.contains("image") {
                return true;
            }
        }
        false
    })
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn clipboard_image_present() -> bool {
    #[cfg(target_os = "windows")]
    return win::clipboard::image_present()
        || clipboard_read_files().iter().any(|p| is_image_path(p));
    #[cfg(not(target_os = "windows"))]
    false
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
    #[cfg(target_os = "windows")]
    return win::launch::shell_open(&url);
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    return win::launch::shell_open(&path);
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn hosts_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into()))
            .join(r"System32\drivers\etc\hosts")
    }
    #[cfg(not(target_os = "windows"))]
    PathBuf::from("/etc/hosts")
}

#[tauri::command]
fn hosts_read() -> Result<String, String> {
    std::fs::read_to_string(hosts_path()).map_err(|e| e.to_string())
}

/// Write the hosts file via an admin prompt (macOS auth dialog / Windows UAC).
/// Runs off the main thread because the prompt is modal and blocking.
#[tauri::command]
async fn hosts_write(content: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let tmp = std::env::temp_dir().join("shu-hosts.tmp");
        std::fs::write(&tmp, content.as_bytes()).map_err(|e| e.to_string())?;
        let status = {
            #[cfg(target_os = "windows")]
            {
                // UAC 提权复制；拒绝 UAC → Start-Process 抛错 → 退出码 1；
                // copy 本身的失败经 -PassThru 的 ExitCode 传出。
                let script = format!(
                    "$p = Start-Process -FilePath cmd.exe -ArgumentList '/c copy /y \"{}\" \"{}\"' -Verb RunAs -Wait -WindowStyle Hidden -PassThru; exit $p.ExitCode",
                    tmp.to_string_lossy(),
                    hosts_path().to_string_lossy()
                );
                Command::new("powershell")
                    .args(["-NoProfile", "-Command", &script])
                    .status()
                    .map_err(|e| e.to_string())?
            }
            #[cfg(not(target_os = "windows"))]
            {
                let script = format!(
                    "do shell script \"cat '{}' > /etc/hosts\" with administrator privileges",
                    tmp.to_string_lossy()
                );
                Command::new("osascript")
                    .arg("-e")
                    .arg(&script)
                    .status()
                    .map_err(|e| e.to_string())?
            }
        };
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
        .join("shu/plugin-data")
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
fn notify(app: AppHandle, title: String, body: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use tauri_plugin_notification::NotificationExt;
        return app
            .notification()
            .builder()
            .title(&title)
            .body(&body)
            .show()
            .map_err(|e| e.to_string());
    }
    #[cfg(target_os = "macos")]
    {
        let _ = &app;
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
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let _ = (&app, &title, &body);
        Err("not supported".into())
    }
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
// Image compression (gated by image.compress) — TinyPNG 式有损量化压缩 PNG。
// 用 imagequant（pngquant 库）做调色板量化，再用 lodepng 编码为索引色 PNG。
// ---------------------------------------------------------------------------

/// 把 PNG 字节做有损量化压缩，返回压缩后的 PNG 字节。纯同步、无异步依赖，
/// 便于直接单测（参照 launch_app_blocking 的可测性抽取方式）。
fn compress_png_bytes(png: &[u8], quality: u8) -> Result<Vec<u8>, String> {
    // 只启用了 image 的 png feature，非 PNG 会解码失败。
    let img = image::load_from_memory(png)
        .map_err(|_| "当前仅支持 PNG 图片".to_string())?
        .to_rgba8();
    let (w, h) = (img.width() as usize, img.height() as usize);
    // imagequant::RGBA 即 rgb::Rgba<u8>，从 RGBA8 缓冲逐像素构造。
    let pixels: Vec<imagequant::RGBA> = img
        .chunks_exact(4)
        .map(|p| imagequant::RGBA { r: p[0], g: p[1], b: p[2], a: p[3] })
        .collect();

    let mut liq = imagequant::new();
    liq.set_quality(quality.saturating_sub(30), quality)
        .map_err(|e| e.to_string())?;
    liq.set_speed(4).ok();
    let mut qimg = liq
        .new_image(pixels, w, h, 0.0)
        .map_err(|e| e.to_string())?;
    let mut res = liq.quantize(&mut qimg).map_err(|e| e.to_string())?;
    res.set_dithering_level(1.0).ok();
    let (palette, indexed) = res.remapped(&mut qimg).map_err(|e| e.to_string())?;

    // lodepng::RGBA 与 imagequant::RGBA 同为 rgb::Rgba<u8>，调色板可直接传入。
    // set_palette 已把色彩类型设为 8-bit 调色板，indexed 即每像素一个调色板下标。
    let mut enc = lodepng::Encoder::new();
    enc.set_palette(&palette).map_err(|e| e.to_string())?;
    enc.encode(&indexed, w, h).map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
struct CompressSource {
    base64: Option<String>,
    path: Option<String>,
}

#[tauri::command]
async fn image_compress(source: CompressSource, quality: u8) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let png: Vec<u8> = if let Some(b64) = source.base64 {
            // 可能是裸 base64，也可能是 data:image/png;base64,... ——有逗号则取其后。
            let payload = b64.split(',').nth(1).unwrap_or(&b64);
            base64::engine::general_purpose::STANDARD
                .decode(payload)
                .map_err(|e| e.to_string())?
        } else if let Some(path) = source.path {
            // ponytail: {path} 是任意文件读原语，靠 network 为高危档（安装红字警告）兜底 exfil；
            // 若将来非压缩插件也需按路径读取，拆成 sensitive 档的独立能力，别复用本 normal 档能力。
            std::fs::read(&path).map_err(|e| e.to_string())?
        } else {
            return Err("缺少图片来源".to_string());
        };
        let before = png.len();
        let out = compress_png_bytes(&png, quality)?;
        let after = out.len();
        Ok(serde_json::json!({
            "data": base64::engine::general_purpose::STANDARD.encode(&out),
            "before": before,
            "after": after,
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 读取一个图片文件的原始字节，返回 base64，供插件把像素载入 canvas 裁剪。
#[tauri::command]
async fn image_read(path: String) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        // ponytail: 与 image_compress({path}) 同类的任意文件读原语，靠 network 为高危档
        // （安装红字警告）兜底 exfil；非压缩插件若也要按路径读取，另拆 sensitive 档能力。
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "base64": base64::engine::general_purpose::STANDARD.encode(&bytes),
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

// ---------------------------------------------------------------------------
// Save dialog (gated by dialog.saveFile) — 弹原生保存面板并写入字节。
// ---------------------------------------------------------------------------

#[tauri::command]
async fn save_file_dialog(
    app: tauri::AppHandle,
    default_path: Option<String>,
    base64_data: String,
) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;
    let mut builder = app.dialog().file().add_filter("PNG", &["png"]);
    if let Some(dp) = &default_path {
        let p = Path::new(dp);
        if let Some(dir) = p.parent() {
            if !dir.as_os_str().is_empty() {
                builder = builder.set_directory(dir);
            }
        }
        if let Some(name) = p.file_name() {
            builder = builder.set_file_name(name.to_string_lossy());
        }
    }
    // 异步命令跑在 Tauri 异步运行时（非主线程）；blocking_save_file 内部会把面板
    // 派发到主线程再阻塞本线程等结果，正是它要求的调用环境。
    let picked = builder.blocking_save_file();
    let path = match picked {
        Some(fp) => fp.into_path().map_err(|e| e.to_string())?,
        None => return Err("__cancelled__".to_string()),
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&base64_data)
        .map_err(|e| e.to_string())?;
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

// ---------------------------------------------------------------------------
// Batch save (gated by dialog.saveFiles) — 弹一次文件夹选择框，批量写入多个文件。
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct SaveFileItem {
    name: String,
    base64: String,
}

#[tauri::command]
async fn save_files_dialog(
    app: tauri::AppHandle,
    default_dir: Option<String>,
    files: Vec<SaveFileItem>,
) -> Result<serde_json::Value, String> {
    use tauri_plugin_dialog::DialogExt;
    let mut builder = app.dialog().file();
    if let Some(dir) = &default_dir {
        if !dir.is_empty() {
            builder = builder.set_directory(dir);
        }
    }
    let dir = match builder.blocking_pick_folder() {
        Some(fp) => fp.into_path().map_err(|e| e.to_string())?,
        None => return Err("__cancelled__".to_string()),
    };
    let mut count = 0u64;
    for f in &files {
        // 只取文件名，忽略路径分量，避免写到所选目录之外。无正常文件名（.. / / 空）则跳过。
        let Some(name) = Path::new(&f.name).file_name() else {
            continue;
        };
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&f.base64)
            .map_err(|e| e.to_string())?;
        std::fs::write(dir.join(name), &bytes).map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(serde_json::json!({ "dir": dir.to_string_lossy(), "count": count }))
}

// ---------------------------------------------------------------------------
// Image preview (gated by image.preview) — 写临时文件后用 Quick Look 浮窗预览。
// ---------------------------------------------------------------------------

#[tauri::command]
async fn image_preview(base64_data: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&base64_data)
            .map_err(|e| e.to_string())?;
        let path = std::env::temp_dir().join("shu-preview.png");
        std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
        // Windows：无 Quick Look，降级为默认看图器打开（阶段 3 评估是否够用）。
        #[cfg(target_os = "windows")]
        return win::launch::shell_open(&path.to_string_lossy());
        // qlmanage -p 打开 Quick Look 浮层（点别处自动消失）；detached，不等它退出。
        #[cfg(not(target_os = "windows"))]
        {
            Command::new("qlmanage")
                .arg("-p")
                .arg(&path)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok(())
        }
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

// ---------------------------------------------------------------------------
// App settings — ~/.config/shu/settings.json（热键、插件自动打开开关等）
// ---------------------------------------------------------------------------

fn settings_path() -> PathBuf {
    dirs::config_dir().unwrap_or_default().join("shu/settings.json")
}

#[tauri::command]
fn settings_read() -> serde_json::Value {
    std::fs::read_to_string(settings_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

#[tauri::command]
fn settings_write(value: serde_json::Value) -> Result<(), String> {
    let path = settings_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| e.to_string())
}

// Windows 上 Win+Shift+Space 与系统输入法反向切换冲突，改用 uTools 惯例的 Alt+Space。
#[cfg(target_os = "windows")]
const DEFAULT_HOTKEY: &str = "alt+space";
#[cfg(not(target_os = "windows"))]
const DEFAULT_HOTKEY: &str = "super+shift+space";

fn register_toggle_hotkey(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    let sc: Shortcut = hotkey.parse().map_err(|e| format!("无法识别的快捷键 {hotkey}: {e:?}"))?;
    app.global_shortcut()
        .on_shortcut(sc, |app, _s, event| {
            if event.state() == ShortcutState::Pressed {
                toggle_window(app);
            }
        })
        .map_err(|e| e.to_string())
}

/// 更换全局热键：先注销旧的，注册失败（冲突等）时回滚到旧热键。
#[tauri::command]
fn set_hotkey(app: AppHandle, hotkey: String) -> Result<(), String> {
    let old = settings_read()["hotkey"].as_str().unwrap_or(DEFAULT_HOTKEY).to_string();
    app.global_shortcut().unregister_all().map_err(|e| e.to_string())?;
    if let Err(e) = register_toggle_hotkey(&app, &hotkey) {
        if register_toggle_hotkey(&app, &old).is_err() {
            let _ = register_toggle_hotkey(&app, DEFAULT_HOTKEY);
        }
        return Err(e);
    }
    let mut s = settings_read();
    s["hotkey"] = serde_json::Value::String(hotkey);
    settings_write(s)
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
    cfg!(debug_assertions) && std::env::var_os("SHU_TEST").is_some()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Windows 单实例：已有实例在跑则唤醒它并退出。
    #[cfg(target_os = "windows")]
    if !win::single_instance::acquire_or_wake() {
        return;
    }

    let builder = tauri::Builder::default();
    #[cfg(target_os = "windows")]
    let builder = builder.plugin(tauri_plugin_notification::init());
    builder
        .manage(AutoHide(std::sync::atomic::AtomicBool::new(true)))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(move |app| {
            let hotkey = settings_read()["hotkey"]
                .as_str()
                .unwrap_or(DEFAULT_HOTKEY)
                .to_string();
            let handle = app.handle().clone();
            if register_toggle_hotkey(&handle, &hotkey).is_err() {
                // 设置里的热键非法/被占用时退回默认，保证应用总能被唤起。
                let _ = register_toggle_hotkey(&handle, DEFAULT_HOTKEY);
            }

            let test_mode = test_mode();
            if test_mode {
                app.state::<AutoHide>()
                    .0
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.navigate(tauri::Url::parse("http://localhost:1420/test").unwrap());
                    let _ = w.set_title("枢 test");
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

            // Windows：监听第二实例的唤醒信号，收到即唤出窗口。
            #[cfg(target_os = "windows")]
            {
                let handle = app.handle().clone();
                win::single_instance::spawn_wake_listener(move || {
                    if let Some(w) = handle.get_webview_window("main") {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                });
            }

            // System tray icon + menu.
            let toggle_item = MenuItem::with_id(app, "toggle", "显示 / 隐藏", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出 枢", true, None::<&str>)?;
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
            settings_read,
            settings_write,
            set_hotkey,
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
            image_compress,
            save_file_dialog,
            save_files_dialog,
            image_preview,
            image_read,
            clipboard_image_present,
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
    fn settings_roundtrip() {
        let path = settings_path();
        let backup = std::fs::read_to_string(&path).ok();
        let v = serde_json::json!({ "hotkey": "super+shift+space" });
        settings_write(v).expect("write settings");
        let r = settings_read();
        assert_eq!(r["hotkey"], "super+shift+space");
        match backup {
            Some(text) => std::fs::write(&path, text).expect("restore settings"),
            None => {
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    #[test]
    fn launch_app_rejects_bad_path() {
        assert!(launch_app_blocking("/nonexistent/definitely-missing.app").is_err());
    }

    // 全平台：Windows 走 %SystemRoot%\System32\drivers\etc\hosts（0.2.x 验收 bug 的回归测试）。
    #[test]
    fn hosts_read_works() {
        let h = hosts_read().expect("read hosts file");
        assert!(!h.is_empty(), "hosts file is empty?");
    }

    // CFStringTransform 是 macOS 系统能力；Windows 拼音在阶段 3 用 pinyin crate。
    #[cfg(target_os = "macos")]
    #[test]
    fn pinyin_transform_works() {
        let (full, initials) = pinyin_pair("微信").expect("pinyin for 微信");
        assert_eq!(full, "weixin");
        assert_eq!(initials, "wx");
        assert!(pinyin_pair("Safari").is_none());
        // 系统 CFStringTransform 对多音字 乐(yuè/lè) 取 "le"，以实际输出为准。
        let (full, _) = pinyin_pair("QQ音乐").expect("pinyin for QQ音乐");
        assert_eq!(full, "qqyinle");
    }

    #[test]
    fn fs_guard_enforces_scope_and_permission() {
        let pid = "com.test.fsguard";
        let dl = dirs::home_dir().unwrap().join("Downloads");
        std::fs::create_dir_all(&dl).ok();
        let f = dl.join("shu-fsguard-test.txt");
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
            .join("shu/plugin-data")
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
        // icon_png 直连渲染路径（绕过磁盘缓存），暖缓存机器上也能真正验证绘制。
        let apps = list_apps();
        assert!(!apps.is_empty(), "no apps found");
        let bytes = apps
            .iter()
            .take(40)
            .filter_map(|a| icon_png(&a.path))
            .next()
            .expect("no icon rendered from any app");
        assert_eq!(&bytes[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
        // rendered straight at 64x64, and not silently blank (wrong ctx setup)
        let img = image::load_from_memory(&bytes).expect("decode png").to_rgba8();
        assert_eq!((img.width(), img.height()), (64, 64));
        assert!(
            img.pixels().any(|p| p.0[3] != 0),
            "icon rendered fully transparent"
        );
        // data-url 封装（含缓存路径）仍由 icon_data_url 保证
        let url = apps
            .iter()
            .take(40)
            .filter_map(|a| icon_data_url(&a.path))
            .next()
            .expect("no data url");
        assert!(url.starts_with("data:image/png;base64,"), "bad data url");
    }

    #[test]
    fn image_compress_roundtrip() {
        // 造一张 256x256 平滑真彩渐变图：色数远超 256，量化后必然显著变小。
        let mut src = image::RgbaImage::new(256, 256);
        for y in 0..256u32 {
            for x in 0..256u32 {
                src.put_pixel(
                    x,
                    y,
                    image::Rgba([x as u8, y as u8, ((x + y) / 2) as u8, 255]),
                );
            }
        }
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(src)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .expect("encode source png");

        let out = compress_png_bytes(&png, 80).expect("compress");
        assert!(!out.is_empty(), "compressed png is empty");
        let decoded = image::load_from_memory(&out)
            .expect("decode compressed png")
            .to_rgba8();
        assert_eq!((decoded.width(), decoded.height()), (256, 256));
        eprintln!("compress: before={} after={}", png.len(), out.len());
        assert!(
            out.len() < png.len(),
            "compression should shrink a truecolor gradient: before={} after={}",
            png.len(),
            out.len()
        );

        // 非 PNG 输入应报错。
        assert!(compress_png_bytes(b"not a png", 80).is_err());
    }

    #[test]
    fn image_read_roundtrip() {
        let p = std::env::temp_dir().join("shu-imgread-test.bin");
        let data = [1u8, 2, 3, 4, 5, 250, 128, 0];
        std::fs::write(&p, data).expect("write temp");
        let v = tauri::async_runtime::block_on(image_read(p.to_string_lossy().to_string()))
            .expect("image_read");
        let b64 = v["base64"].as_str().expect("base64 field");
        let back = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .expect("decode");
        assert_eq!(back, data);
        let _ = std::fs::remove_file(&p);
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
