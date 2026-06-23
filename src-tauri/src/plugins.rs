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

fn config_root() -> PathBuf {
    dirs::config_dir().unwrap_or_default().join("pc-tool")
}

pub fn installed_dir() -> PathBuf {
    config_root().join("plugins")
}

fn registry_path() -> PathBuf {
    config_root().join("registry.json")
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

use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::Path;

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

#[derive(Serialize)]
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
    if manifest.id.trim().is_empty() {
        return Err("plugin.json missing id".into());
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
            source: if origin.starts_with("http") { "url".into() } else { "file".into() },
            origin,
            sha256: info.sha256,
            installed_at: now_iso(),
        },
    );
    write_registry(&reg)
}

#[tauri::command]
pub fn uninstall_plugin(id: String) -> Result<(), String> {
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

#[cfg(test)]
mod tests {
    use super::*;

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

        let id = "com.pc-tool.json-preview";
        // clean any prior state
        let _ = uninstall_plugin(id.into());

        install_package(
            out.to_string_lossy().into(),
            vec!["clipboard.read".into(), "clipboard.write".into()],
            out.to_string_lossy().into(),
        )
        .unwrap();

        let installed = list_installed();
        let found = installed.iter().find(|p| p.id == id).expect("not installed");
        assert_eq!(found.granted, vec!["clipboard.read", "clipboard.write"]);
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
