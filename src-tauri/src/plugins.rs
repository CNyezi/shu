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
