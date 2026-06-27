use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RegEntry {
    pub version: String,
    pub granted: Vec<String>,
    pub source: String, // "file" | "url"
    pub origin: String,
    pub sha256: String,
    pub installed_at: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Registry {
    pub plugins: BTreeMap<String, RegEntry>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Registries {
    pub urls: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegistryPlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub permissions: Vec<String>,
    #[serde(rename = "packageUrl")]
    pub package_url: String,
    pub sha256: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegistryFeed {
    pub version: u32,
    pub plugins: Vec<RegistryPlugin>,
}

fn config_root() -> PathBuf {
    dirs::config_dir().unwrap_or_default().join("pc-tool")
}

pub fn installed_dir() -> PathBuf {
    config_root().join("plugins")
}

fn registry_path() -> PathBuf {
    config_root().join("registry.json")
}

fn registries_path() -> PathBuf {
    config_root().join("registries.json")
}

fn valid_http_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

pub fn read_registry() -> Registry {
    match std::fs::read_to_string(registry_path()) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Registry::default(),
    }
}

pub fn write_registry(reg: &Registry) -> Result<(), String> {
    let path = registry_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let s = serde_json::to_string_pretty(reg).map_err(|e| e.to_string())?;
    std::fs::write(&path, s).map_err(|e| e.to_string())
}

fn read_registries() -> Registries {
    match std::fs::read_to_string(registries_path()) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Registries::default(),
    }
}

fn write_registries(reg: &Registries) -> Result<(), String> {
    let path = registries_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let s = serde_json::to_string_pretty(reg).map_err(|e| e.to_string())?;
    std::fs::write(path, s).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_registries() -> Vec<String> {
    read_registries().urls
}

#[tauri::command]
pub fn add_registry(url: String) -> Result<(), String> {
    if !valid_http_url(&url) {
        return Err("only http/https registry URLs are allowed".into());
    }
    let mut reg = read_registries();
    if !reg.urls.contains(&url) {
        reg.urls.push(url);
    }
    write_registries(&reg)
}

#[tauri::command]
pub fn remove_registry(url: String) -> Result<(), String> {
    let mut reg = read_registries();
    reg.urls.retain(|u| u != &url);
    write_registries(&reg)
}

use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::Path;

fn is_safe_id(id: &str) -> bool {
    use std::path::Component;
    let id = id.trim();
    if id.is_empty() || id.contains('/') || id.contains('\\') || id == "." || id == ".." {
        return false;
    }
    let mut comps = std::path::Path::new(id).components();
    matches!(comps.next(), Some(Component::Normal(_))) && comps.next().is_none()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    // features kept opaque here; the frontend already parses them.
    #[serde(default)]
    pub features: serde_json::Value,
}

#[derive(Serialize, Debug)]
pub struct PackageInspect {
    pub manifest: Manifest,
    pub sha256: String,
    pub is_upgrade: bool,
    pub new_permissions: Vec<String>,
}

fn read_manifest_from_zip(path: &str) -> Result<(Manifest, Vec<u8>), String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let reader = std::io::Cursor::new(&bytes);
    let mut zip = zip::ZipArchive::new(reader).map_err(|e| e.to_string())?;
    let mut text = String::new();
    {
        use std::io::Read;
        zip.by_name("plugin.json")
            .map_err(|_| "package has no plugin.json at root".to_string())?
            .read_to_string(&mut text)
            .map_err(|e| e.to_string())?;
    }
    let manifest: Manifest =
        serde_json::from_str(&text).map_err(|e| format!("invalid plugin.json: {e}"))?;
    if !is_safe_id(&manifest.id) {
        return Err(format!("unsafe plugin id: {}", manifest.id));
    }
    Ok((manifest, bytes))
}

#[tauri::command]
pub fn inspect_package(path: String) -> Result<PackageInspect, String> {
    let (manifest, bytes) = read_manifest_from_zip(&path)?;
    let sha256 = format!("{:x}", Sha256::digest(&bytes));

    let reg = read_registry();
    let (is_upgrade, new_permissions) = match reg.plugins.get(&manifest.id) {
        Some(entry) => {
            let new_perms: Vec<String> = manifest
                .permissions
                .iter()
                .filter(|p| !entry.granted.contains(p))
                .cloned()
                .collect();
            (true, new_perms)
        }
        None => (false, manifest.permissions.clone()),
    };

    Ok(PackageInspect {
        manifest,
        sha256,
        is_upgrade,
        new_permissions,
    })
}

fn add_dir_to_zip<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    base: &Path,
    dir: &Path,
) -> Result<(), String> {
    let opts = zip::write::SimpleFileOptions::default();
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let rel = path
            .strip_prefix(base)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        if path.is_dir() {
            zip.add_directory(format!("{rel}/"), opts)
                .map_err(|e| e.to_string())?;
            add_dir_to_zip(zip, base, &path)?;
        } else {
            zip.start_file(rel, opts).map_err(|e| e.to_string())?;
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            zip.write_all(&bytes).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn pack_plugin(src_dir: String, out_path: String) -> Result<(), String> {
    let src = PathBuf::from(&src_dir);
    if !src.join("plugin.json").exists() {
        return Err("source folder has no plugin.json".into());
    }
    let file = std::fs::File::create(&out_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    add_dir_to_zip(&mut zip, &src, &src)?;
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Serialize)]
pub struct InstalledPlugin {
    pub id: String,
    pub version: String,
    pub granted: Vec<String>,
    pub source: String,
    pub origin: String,
}

/// `path` is the local `.pcp` to install; `origin` is the user-facing source
/// (the original URL for URL installs, or the file path) recorded in the registry.
#[tauri::command]
pub fn install_package(path: String, granted: Vec<String>, origin: String) -> Result<(), String> {
    let (manifest, _bytes) = read_manifest_from_zip(&path)?;
    let info = inspect_package(path.clone())?;

    // Block downgrades.
    let reg = read_registry();
    if let Some(existing) = reg.plugins.get(&manifest.id) {
        if version_lt(&manifest.version, &existing.version) {
            return Err(format!(
                "refusing to downgrade {} ({} < {})",
                manifest.id, manifest.version, existing.version
            ));
        }
    }

    // Extract into installed_dir/<id>, replacing any prior copy.
    let dest = installed_dir().join(&manifest.id);
    let _ = std::fs::remove_dir_all(&dest);
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    let file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    zip.extract(&dest).map_err(|e| e.to_string())?;

    // granted can never exceed what the manifest declares.
    let granted: Vec<String> = granted
        .into_iter()
        .filter(|p| manifest.permissions.contains(p))
        .collect();

    let mut reg = read_registry();
    reg.plugins.insert(
        manifest.id.clone(),
        RegEntry {
            version: manifest.version,
            granted,
            source: if origin.to_lowercase().starts_with("http://") || origin.to_lowercase().starts_with("https://") { "url".into() } else { "file".into() },
            origin,
            sha256: info.sha256,
            installed_at: now_iso(),
        },
    );
    write_registry(&reg)
}

#[tauri::command]
pub fn uninstall_plugin(id: String) -> Result<(), String> {
    if !is_safe_id(&id) {
        return Err("unsafe plugin id".into());
    }
    let _ = std::fs::remove_dir_all(installed_dir().join(&id));
    let mut reg = read_registry();
    reg.plugins.remove(&id);
    write_registry(&reg)
}

#[tauri::command]
pub fn list_installed() -> Vec<InstalledPlugin> {
    read_registry()
        .plugins
        .into_iter()
        .map(|(id, e)| InstalledPlugin {
            id,
            version: e.version,
            granted: e.granted,
            source: e.source,
            origin: e.origin,
        })
        .collect()
}

/// Naive semver-ish "a < b" by dotted numeric comparison.
fn version_lt(a: &str, b: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.split('.').map(|p| p.parse().unwrap_or(0)).collect()
    };
    parse(a) < parse(b)
}

fn now_iso() -> String {
    // Avoid extra deps; seconds since epoch is enough for an install timestamp.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

fn bundled_dir(app: &tauri::AppHandle) -> PathBuf {
    #[cfg(debug_assertions)]
    {
        let _ = app;
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map(|p| p.join("plugins"))
            .unwrap_or_default()
    }
    #[cfg(not(debug_assertions))]
    {
        use tauri::Manager;
        app.path()
            .resource_dir()
            .map(|p| p.join("plugins"))
            .unwrap_or_default()
    }
}

fn scan_dir(dir: &PathBuf, source: &str, reg: &Registry, out: &mut Vec<serde_json::Value>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let manifest = entry.path().join("plugin.json");
        let Ok(text) = std::fs::read_to_string(&manifest) else {
            continue;
        };
        let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let dir_name = entry.file_name().to_string_lossy().to_string();
        v["_dir"] = serde_json::Value::String(dir_name);
        v["source"] = serde_json::Value::String(source.to_string());
        // granted: bundled -> manifest perms; installed -> registry grant.
        let manifest_perms = v["permissions"].clone();
        let granted = if source == "installed" {
            let id = v["id"].as_str().unwrap_or("");
            reg.plugins
                .get(id)
                .map(|e| serde_json::to_value(&e.granted).unwrap())
                .unwrap_or(manifest_perms)
        } else {
            manifest_perms
        };
        v["granted"] = granted;
        out.push(v);
    }
}

fn safe_plugin_dir(dir: &str) -> Option<PathBuf> {
    is_safe_id(dir).then(|| PathBuf::from(dir))
}

fn safe_plugin_rel_path(rel: &str) -> Option<PathBuf> {
    let path = Path::new(rel);
    if path.is_absolute() {
        return None;
    }
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            std::path::Component::Normal(p) => out.push(p),
            _ => return None,
        }
    }
    (!out.as_os_str().is_empty()).then_some(out)
}

#[tauri::command]
pub fn list_plugins(app: tauri::AppHandle) -> Vec<serde_json::Value> {
    let reg = read_registry();
    let mut out = Vec::new();
    scan_dir(&bundled_dir(&app), "bundled", &reg, &mut out);
    scan_dir(&installed_dir(), "installed", &reg, &mut out);
    out
}

#[tauri::command]
pub fn read_plugin_file(app: tauri::AppHandle, dir: String, rel: String) -> Result<String, String> {
    let dir = safe_plugin_dir(&dir).ok_or("invalid path")?;
    let rel = safe_plugin_rel_path(&rel).ok_or("invalid path")?;
    // Try installed first, then bundled.
    for base in [installed_dir(), bundled_dir(&app)] {
        let path = base.join(&dir).join(&rel);
        if let Ok(text) = std::fs::read_to_string(&path) {
            return Ok(text);
        }
    }
    Err("file not found".into())
}

/// Read a plugin's logo (any image format the plugin author chose) as a data
/// URL, so `plugin.json`'s `icon` can be SVG, PNG, JPG, etc.
#[tauri::command]
pub fn read_plugin_icon(app: tauri::AppHandle, dir: String, rel: String) -> Result<String, String> {
    use base64::Engine;
    let dir = safe_plugin_dir(&dir).ok_or("invalid path")?;
    let rel = safe_plugin_rel_path(&rel).ok_or("invalid path")?;
    for base in [installed_dir(), bundled_dir(&app)] {
        let path = base.join(&dir).join(&rel);
        if let Ok(bytes) = std::fs::read(&path) {
            let mime = match path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .as_deref()
            {
                Some("svg") => "image/svg+xml",
                Some("png") => "image/png",
                Some("jpg") | Some("jpeg") => "image/jpeg",
                Some("gif") => "image/gif",
                Some("webp") => "image/webp",
                Some("ico") => "image/x-icon",
                _ => "application/octet-stream",
            };
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            return Ok(format!("data:{mime};base64,{b64}"));
        }
    }
    Err("icon not found".into())
}

// ---------------------------------------------------------------------------
// Per-plugin private key/value storage (no permission required — a plugin's
// own sandboxed data, namespaced by plugin id which the host injects).
// ---------------------------------------------------------------------------

fn storage_path(plugin_id: &str) -> Option<PathBuf> {
    if plugin_id.contains("..") || plugin_id.contains('/') || plugin_id.contains('\\') {
        return None;
    }
    Some(config_root().join("plugin-data").join(format!("{plugin_id}.json")))
}

fn read_storage(plugin_id: &str) -> serde_json::Map<String, serde_json::Value> {
    let Some(p) = storage_path(plugin_id) else {
        return Default::default();
    };
    std::fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_storage(
    plugin_id: &str,
    map: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), String> {
    let p = storage_path(plugin_id).ok_or("invalid plugin id")?;
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let s = serde_json::to_string(map).map_err(|e| e.to_string())?;
    std::fs::write(&p, s).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn plugin_storage_get(plugin_id: String, key: String) -> serde_json::Value {
    read_storage(&plugin_id)
        .get(&key)
        .cloned()
        .unwrap_or(serde_json::Value::Null)
}

#[tauri::command]
pub fn plugin_storage_set(
    plugin_id: String,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    let mut m = read_storage(&plugin_id);
    m.insert(key, value);
    write_storage(&plugin_id, &m)
}

#[tauri::command]
pub fn plugin_storage_remove(plugin_id: String, key: String) -> Result<(), String> {
    let mut m = read_storage(&plugin_id);
    m.remove(&key);
    write_storage(&plugin_id, &m)
}

#[tauri::command]
pub fn plugin_storage_keys(plugin_id: String) -> Vec<String> {
    read_storage(&plugin_id).keys().cloned().collect()
}

fn validate_feed(feed: &RegistryFeed) -> Result<(), String> {
    if feed.version != 1 {
        return Err("unsupported registry version".into());
    }
    for p in &feed.plugins {
        if !is_safe_id(&p.id) {
            return Err(format!("unsafe plugin id: {}", p.id));
        }
        if !valid_http_url(&p.package_url) {
            return Err(format!("invalid packageUrl for {}", p.id));
        }
        if p.sha256.len() != 64 || !p.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("invalid sha256 for {}", p.id));
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn fetch_registry(url: String) -> Result<RegistryFeed, String> {
    if !valid_http_url(&url) {
        return Err("only http/https registry URLs are allowed".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let text = ureq::get(&url)
            .call()
            .map_err(|e| e.to_string())?
            .into_string()
            .map_err(|e| e.to_string())?;
        let feed: RegistryFeed =
            serde_json::from_str(&text).map_err(|e| format!("invalid registry.json: {e}"))?;
        validate_feed(&feed)?;
        Ok(feed)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn verify_sha256(bytes: &[u8], expected: &str) -> Result<(), String> {
    let got = format!("{:x}", Sha256::digest(bytes));
    if got.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(format!("sha256 mismatch: expected {expected}, got {got}"))
    }
}

#[tauri::command]
pub async fn download_package(url: String) -> Result<String, String> {
    if !valid_http_url(&url) {
        return Err("only http/https URLs are allowed".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let resp = ureq::get(&url).call().map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        use std::io::Read;
        resp.into_reader()
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        // Unique temp name derived from the URL so concurrent downloads don't collide.
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        url.hash(&mut h);
        let out = std::env::temp_dir().join(format!("pctool-dl-{:x}.pcp", h.finish()));
        std::fs::write(&out, &bytes).map_err(|e| e.to_string())?;
        Ok(out.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn download_package_checked(url: String, sha256: String) -> Result<String, String> {
    if !valid_http_url(&url) {
        return Err("only http/https URLs are allowed".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let resp = ureq::get(&url).call().map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        use std::io::Read;
        resp.into_reader()
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        verify_sha256(&bytes, &sha256)?;
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        url.hash(&mut h);
        let out = std::env::temp_dir().join(format!("pctool-registry-{:x}.pcp", h.finish()));
        std::fs::write(&out, &bytes).map_err(|e| e.to_string())?;
        Ok(out.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_namespaced_roundtrip() {
        let _ = std::fs::remove_file(storage_path("com.test.s").unwrap());
        plugin_storage_set("com.test.s".into(), "k".into(), serde_json::json!(42)).unwrap();
        assert_eq!(plugin_storage_get("com.test.s".into(), "k".into()), serde_json::json!(42));
        assert!(plugin_storage_keys("com.test.s".into()).contains(&"k".to_string()));
        plugin_storage_remove("com.test.s".into(), "k".into()).unwrap();
        assert_eq!(plugin_storage_get("com.test.s".into(), "k".into()), serde_json::Value::Null);
        // path traversal id rejected
        assert!(storage_path("../evil").is_none());
    }

    #[test]
    fn inspect_rejects_path_traversal_id() {
        // Build a zip in memory whose plugin.json has id = "../evil".
        use std::io::Write as _;
        let out = std::env::temp_dir().join("pctool-test-evil.pcp");
        let _ = std::fs::remove_file(&out);
        let file = std::fs::File::create(&out).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("plugin.json", opts).unwrap();
        zip.write_all(
            br#"{"id":"../evil","name":"Evil","version":"1.0.0"}"#,
        )
        .unwrap();
        zip.finish().unwrap();

        let err = inspect_package(out.to_string_lossy().into())
            .expect_err("should have rejected traversal id");
        assert!(
            err.contains("unsafe"),
            "error message should mention 'unsafe', got: {err}"
        );
    }

    #[test]
    fn plugin_asset_paths_must_stay_relative() {
        assert!(safe_plugin_dir("json-preview").is_some());
        assert!(safe_plugin_dir("../evil").is_none());

        assert!(safe_plugin_rel_path("index.html").is_some());
        assert!(safe_plugin_rel_path("assets/main.js").is_some());
        assert!(safe_plugin_rel_path("/etc/passwd").is_none());
        assert!(safe_plugin_rel_path("../secret").is_none());
    }

    #[test]
    fn registry_urls_roundtrip_and_validate() {
        let old = read_registries();
        let _ = write_registries(&Registries::default());
        add_registry("https://example.com/registry.json".into()).unwrap();
        assert_eq!(list_registries(), vec!["https://example.com/registry.json"]);
        assert!(add_registry("file:///tmp/registry.json".into()).is_err());
        remove_registry("https://example.com/registry.json".into()).unwrap();
        assert!(list_registries().is_empty());
        write_registries(&old).unwrap();
    }

    #[test]
    fn checked_download_rejects_bad_hash() {
        let err = verify_sha256(b"abc", "bad").expect_err("bad hash should fail");
        assert!(err.contains("sha256"));
    }

    #[test]
    fn registry_roundtrip_in_memory() {
        let mut reg = Registry::default();
        reg.plugins.insert(
            "com.x.foo".into(),
            RegEntry {
                version: "1.0.0".into(),
                granted: vec!["clipboard.read".into()],
                source: "file".into(),
                origin: "/tmp/foo.pcp".into(),
                sha256: "abc".into(),
                installed_at: "2026-06-23T00:00:00Z".into(),
            },
        );
        let json = serde_json::to_string(&reg).unwrap();
        let back: Registry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.plugins["com.x.foo"].granted, vec!["clipboard.read"]);
    }

    #[test]
    fn install_list_uninstall_roundtrip() {
        let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("plugins/json-preview");
        let out = std::env::temp_dir().join("pctool-test-install.pcp");
        let _ = std::fs::remove_file(&out);
        pack_plugin(src.to_string_lossy().into(), out.to_string_lossy().into()).unwrap();
        let info = inspect_package(out.to_string_lossy().into()).unwrap();
        assert_eq!(info.manifest.id, "com.pc-tool.json-preview");
        assert_eq!(info.new_permissions, vec!["clipboard.read", "clipboard.write"]);

        let id = "com.pc-tool.json-preview";
        // clean any prior state
        let _ = uninstall_plugin(id.into());

        install_package(
            out.to_string_lossy().into(),
            vec![
                "clipboard.read".into(),
                "clipboard.write".into(),
                "shell.openUrl".into(),
            ],
            out.to_string_lossy().into(),
        )
        .unwrap();

        let installed = list_installed();
        let found = installed.iter().find(|p| p.id == id).expect("not installed");
        assert_eq!(found.granted, vec!["clipboard.read", "clipboard.write"]);
        assert!(!found.granted.contains(&"shell.openUrl".to_string()));
        assert!(installed_dir().join(id).join("plugin.json").exists());

        uninstall_plugin(id.into()).unwrap();
        assert!(!installed_dir().join(id).exists());
        assert!(list_installed().iter().all(|p| p.id != id));
    }

    #[test]
    fn inspect_reads_manifest_and_hash() {
        let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("plugins/json-preview");
        let out = std::env::temp_dir().join("pctool-test-inspect.pcp");
        let _ = std::fs::remove_file(&out);
        pack_plugin(src.to_string_lossy().into(), out.to_string_lossy().into()).unwrap();

        let info = inspect_package(out.to_string_lossy().into()).unwrap();
        assert_eq!(info.manifest.id, "com.pc-tool.json-preview");
        assert!(!info.sha256.is_empty());
        assert_eq!(info.sha256.len(), 64); // hex of 32 bytes
    }

    #[test]
    fn pack_then_unzip_contains_manifest() {
        // Pack the repo's bundled json-preview plugin.
        let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("plugins/json-preview");
        let out = std::env::temp_dir().join("pctool-test-json-preview.pcp");
        let _ = std::fs::remove_file(&out);

        pack_plugin(src.to_string_lossy().into(), out.to_string_lossy().into()).unwrap();
        assert!(out.exists(), "package not created");

        let f = std::fs::File::open(&out).unwrap();
        let mut zip = zip::ZipArchive::new(f).unwrap();
        let mut manifest = String::new();
        use std::io::Read;
        zip.by_name("plugin.json")
            .expect("plugin.json missing from package")
            .read_to_string(&mut manifest)
            .unwrap();
        assert!(manifest.contains("\"id\""), "manifest has no id");
    }
}
