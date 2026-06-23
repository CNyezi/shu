# Plugin Sharing v1 (Package + Install + Permission Gate) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users package a plugin into a `.pcp` file, share it (file or URL), install it through a permission-consent gate, run it sandboxed under the granted permissions, and manage/uninstall it.

**Architecture:** A new Rust module `plugins.rs` owns the package format (zip), the on-disk install dir, and a `registry.json` that records each installed plugin's *granted* permissions. The sandbox runtime's capability whitelist switches from `manifest.permissions` to the registry's `granted` set. The Svelte host shell gains a plugin-manager view and an install-consent view, plus drag-drop / file / URL install entries that all funnel through one pipeline (inspect → consent → install).

**Tech Stack:** Rust (Tauri v2), crates `zip` `sha2` `ureq` + `tauri-plugin-dialog`; Svelte 5; existing sandbox runtime.

**Spec:** `docs/superpowers/specs/2026-06-23-plugin-sharing-v1-design.md`

---

## File Structure

**Rust (`src-tauri/src/`)**
- Create `plugins.rs` — package/install/registry logic + the plugin-loading commands moved here:
  - dirs: `installed_dir()`, `bundled_dir(app)`, `registry_path()`
  - registry: `Registry`/`RegEntry` structs, `read_registry()`, `write_registry()`
  - `pack_plugin`, `inspect_package`, `download_package`, `install_package`, `uninstall_plugin`, `list_installed`
  - moved from lib.rs: `list_plugins` (now merged + granted), `read_plugin_file`
- Modify `lib.rs` — `mod plugins;`, register the new commands, init `tauri-plugin-dialog`. Remove the old `plugins_dir`/`list_plugins`/`read_plugin_file` (moved).
- Modify `Cargo.toml` — add `zip`, `sha2`, `ureq`, `serde` already present; `tauri-plugin-dialog`.
- Modify `capabilities/default.json` — add dialog permissions.

**Frontend (`src/`)**
- Modify `lib/types.ts` — add `granted`/`source` to `Plugin`; add `InstalledPlugin`, `PackageInspect`.
- Modify `lib/host.ts` — wrappers for the new commands.
- Create `lib/permissions.ts` — capability id → human label map.
- Modify `lib/pluginRuntime.ts` — whitelist from `granted` (∩ manifest).
- Create `lib/InstallConsent.svelte` — consent dialog view.
- Create `lib/PluginManager.svelte` — installed-list + install entries view.
- Modify `App.svelte` — `manager`/`consent` modes, keyword `插件`/`plugins` routing, install pipeline, drag-drop, pass `granted` to runtime.

---

## Task 1: Rust — plugins module, dirs, registry read/write

**Files:**
- Create: `src-tauri/src/plugins.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod plugins;` near top)
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add deps**

In `src-tauri/Cargo.toml` under `[dependencies]` add:

```toml
zip = "2"
sha2 = "0.10"
ureq = "2"
```

- [ ] **Step 2: Create `plugins.rs` with dirs + registry + a round-trip test**

Create `src-tauri/src/plugins.rs`:

```rust
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
}
```

- [ ] **Step 3: Wire the module into lib.rs**

In `src-tauri/src/lib.rs`, add after the `use` block at the top:

```rust
mod plugins;
```

- [ ] **Step 4: Run the test**

Run: `cd src-tauri && cargo test plugins::tests::registry_roundtrip_in_memory -- --nocapture`
Expected: PASS (compiles with the new module + deps).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/plugins.rs src-tauri/src/lib.rs
git commit -m "feat(plugins): add plugins module with registry read/write"
```

---

## Task 2: Rust — `pack_plugin` (zip a plugin folder)

**Files:**
- Modify: `src-tauri/src/plugins.rs`

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `plugins.rs`:

```rust
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
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `cd src-tauri && cargo test plugins::tests::pack_then_unzip -- --nocapture`
Expected: FAIL — `cannot find function pack_plugin`.

- [ ] **Step 3: Implement `pack_plugin`**

Add to `plugins.rs` (above the tests module):

```rust
use std::io::Write;
use std::path::Path;

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
```

- [ ] **Step 4: Run the test**

Run: `cd src-tauri && cargo test plugins::tests::pack_then_unzip -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/plugins.rs
git commit -m "feat(plugins): pack a plugin folder into a .pcp zip"
```

---

## Task 3: Rust — `inspect_package` (parse + sha256 + upgrade detection)

**Files:**
- Modify: `src-tauri/src/plugins.rs`

- [ ] **Step 1: Write the failing test**

Add to the `tests` module:

```rust
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
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `cd src-tauri && cargo test plugins::tests::inspect_reads -- --nocapture`
Expected: FAIL — `cannot find function inspect_package` / type `Manifest`.

- [ ] **Step 3: Implement manifest types, sha256, and `inspect_package`**

Add to `plugins.rs`:

```rust
use sha2::{Digest, Sha256};

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
```

- [ ] **Step 4: Run the test**

Run: `cd src-tauri && cargo test plugins::tests::inspect_reads -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/plugins.rs
git commit -m "feat(plugins): inspect_package parses manifest, hashes, detects upgrades"
```

---

## Task 4: Rust — `install_package` + `list_installed` + `uninstall_plugin`

**Files:**
- Modify: `src-tauri/src/plugins.rs`

- [ ] **Step 1: Write the failing test (full round trip)**

Add to the `tests` module:

```rust
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
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `cd src-tauri && cargo test plugins::tests::install_list_uninstall -- --nocapture --test-threads=1`
Expected: FAIL — missing `install_package` / `list_installed` / `uninstall_plugin` / `InstalledPlugin`.

- [ ] **Step 3: Implement install/list/uninstall**

Add to `plugins.rs`:

```rust
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
```

- [ ] **Step 4: Run the test**

Run: `cd src-tauri && cargo test plugins::tests::install_list_uninstall -- --nocapture --test-threads=1`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/plugins.rs
git commit -m "feat(plugins): install/list/uninstall with registry + downgrade guard"
```

---

## Task 5: Rust — merged `list_plugins` (bundled + installed, with granted/source) + move `read_plugin_file` + `download_package`

**Files:**
- Modify: `src-tauri/src/plugins.rs`
- Modify: `src-tauri/src/lib.rs` (remove old `plugins_dir`/`list_plugins`/`read_plugin_file`)

- [ ] **Step 1: Implement bundled_dir, merged list_plugins, read_plugin_file, download_package in plugins.rs**

Add to `plugins.rs`:

```rust
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
    if dir.contains("..") || dir.contains('/') || rel.contains("..") {
        return Err("invalid path".into());
    }
    // Try installed first, then bundled.
    for base in [installed_dir(), bundled_dir(&app)] {
        let path = base.join(&dir).join(&rel);
        if let Ok(text) = std::fs::read_to_string(&path) {
            return Ok(text);
        }
    }
    Err("file not found".into())
}

#[tauri::command]
pub async fn download_package(url: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let resp = ureq::get(&url).call().map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        use std::io::Read;
        resp.into_reader()
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        let out = std::env::temp_dir().join("pctool-download.pcp");
        std::fs::write(&out, &bytes).map_err(|e| e.to_string())?;
        Ok(out.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
```

- [ ] **Step 2: Remove the old copies from lib.rs**

In `src-tauri/src/lib.rs`, delete the now-duplicated functions: `fn plugins_dir`, `fn list_plugins`, `fn read_plugin_file` (they live in `plugins.rs` now). Leave everything else.

- [ ] **Step 3: Update the command registration in lib.rs**

In `lib.rs`, change the `tauri::generate_handler![...]` list: replace the bare `list_plugins, read_plugin_file` entries with the module-qualified new commands. The handler block becomes:

```rust
.invoke_handler(tauri::generate_handler![
    list_apps,
    launch_app,
    app_icon,
    clipboard_read,
    clipboard_write,
    open_url,
    open_path,
    hide_window,
    plugins::list_plugins,
    plugins::read_plugin_file,
    plugins::pack_plugin,
    plugins::inspect_package,
    plugins::install_package,
    plugins::uninstall_plugin,
    plugins::list_installed,
    plugins::download_package
])
```

- [ ] **Step 4: Verify it compiles + all tests pass**

Run: `cd src-tauri && cargo test -- --test-threads=1 2>&1 | tail -20`
Expected: compiles; all `plugins::tests::*` and existing tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/plugins.rs src-tauri/src/lib.rs
git commit -m "feat(plugins): merged list_plugins (bundled+installed+granted), download_package"
```

---

## Task 6: Rust — add `tauri-plugin-dialog` for the file picker

**Files:**
- Modify: `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json`, `package.json`

- [ ] **Step 1: Add the Rust + JS deps**

`src-tauri/Cargo.toml` `[dependencies]`: add `tauri-plugin-dialog = "2"`.
Then in the project root: `pnpm add @tauri-apps/plugin-dialog` (run from repo root).

- [ ] **Step 2: Init the plugin in lib.rs**

In `lib.rs` `run()`, in the builder chain (next to the global-shortcut plugin), add:

```rust
.plugin(tauri_plugin_dialog::init())
```

- [ ] **Step 3: Grant the dialog capability**

In `src-tauri/capabilities/default.json`, add to the `permissions` array:

```json
"dialog:allow-open"
```

- [ ] **Step 4: Verify it compiles**

Run: `cd src-tauri && cargo build 2>&1 | tail -5`
Expected: `Finished`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/lib.rs src-tauri/capabilities/default.json package.json pnpm-lock.yaml
git commit -m "feat(plugins): add tauri-plugin-dialog for file picker"
```

---

## Task 7: Frontend — types, host wrappers, permission labels

**Files:**
- Modify: `src/lib/types.ts`, `src/lib/host.ts`
- Create: `src/lib/permissions.ts`

- [ ] **Step 1: Extend types**

In `src/lib/types.ts`, add `granted`/`source` to `Plugin` and new types:

```ts
export type Plugin = {
  id: string;
  name: string;
  version: string;
  icon?: string;
  features: Feature[];
  permissions: string[];
  /** folder name on disk, injected by the Rust loader */
  _dir: string;
  /** permissions actually granted by the user (bundled = all) */
  granted: string[];
  /** "bundled" | "installed" */
  source: string;
};

export type InstalledPlugin = {
  id: string;
  version: string;
  granted: string[];
  source: string;
  origin: string;
};

export type PackageInspect = {
  manifest: {
    id: string;
    name: string;
    version: string;
    icon?: string;
    permissions: string[];
  };
  sha256: string;
  is_upgrade: boolean;
  new_permissions: string[];
};
```

- [ ] **Step 2: Add host wrappers**

In `src/lib/host.ts`, add:

```ts
import type { InstalledPlugin, PackageInspect } from "./types";

export const inspectPackage = (path: string) =>
  invoke<PackageInspect>("inspect_package", { path });
export const downloadPackage = (url: string) =>
  invoke<string>("download_package", { url });
export const installPackage = (path: string, granted: string[], origin: string) =>
  invoke<void>("install_package", { path, granted, origin });
export const uninstallPlugin = (id: string) =>
  invoke<void>("uninstall_plugin", { id });
export const listInstalled = () => invoke<InstalledPlugin[]>("list_installed");
```

- [ ] **Step 3: Create the permission label map**

Create `src/lib/permissions.ts`:

```ts
const LABELS: Record<string, string> = {
  "clipboard.read": "读取剪贴板",
  "clipboard.write": "写入剪贴板",
  "shell.openUrl": "用浏览器打开网址",
  "shell.openPath": "用默认程序打开文件",
};

export function permissionLabel(id: string): string {
  return LABELS[id] ?? id;
}
```

- [ ] **Step 4: Type-check**

Run (from repo root): `pnpm check 2>&1 | tail -10`
Expected: no errors from these files (pre-existing warnings elsewhere are fine).

- [ ] **Step 5: Commit**

```bash
git add src/lib/types.ts src/lib/host.ts src/lib/permissions.ts
git commit -m "feat(plugins): frontend types, host wrappers, permission labels"
```

---

## Task 8: Frontend — runtime whitelist uses `granted`

**Files:**
- Modify: `src/lib/pluginRuntime.ts`, `src/App.svelte`

- [ ] **Step 1: Use granted in the runtime whitelist**

In `src/lib/pluginRuntime.ts`, in `mountPlugin`, change the whitelist line:

```ts
// before: const whitelist = new Set(plugin.permissions || []);
const declared = new Set(plugin.permissions || []);
const whitelist = new Set((plugin.granted || []).filter((p) => declared.has(p)));
```

- [ ] **Step 2: Type-check**

Run: `pnpm check 2>&1 | tail -10`
Expected: no new errors (`Plugin.granted` exists from Task 7).

- [ ] **Step 3: Build to confirm**

Run: `pnpm build 2>&1 | tail -4`
Expected: built.

- [ ] **Step 4: Commit**

```bash
git add src/lib/pluginRuntime.ts
git commit -m "feat(plugins): sandbox whitelist enforces granted ∩ declared"
```

---

## Task 9: Frontend — `InstallConsent.svelte` consent dialog

**Files:**
- Create: `src/lib/InstallConsent.svelte`

- [ ] **Step 1: Create the component**

Create `src/lib/InstallConsent.svelte`:

```svelte
<script lang="ts">
  import { permissionLabel } from "./permissions";
  import type { PackageInspect } from "./types";

  let {
    info,
    onApprove,
    onCancel,
  }: {
    info: PackageInspect;
    onApprove: () => void;
    onCancel: () => void;
  } = $props();

  const perms = $derived(info.manifest.permissions ?? []);
  const isNew = (p: string) => info.is_upgrade && info.new_permissions.includes(p);
</script>

<div class="consent">
  <div class="head">
    <div class="title">{info.manifest.name}</div>
    <div class="ver">v{info.manifest.version}{info.is_upgrade ? "（升级）" : ""}</div>
  </div>

  <div class="section">该插件申请以下能力：</div>
  <ul class="perms">
    {#each perms as p (p)}
      <li class:fresh={isNew(p)}>
        {permissionLabel(p)}{isNew(p) ? "  · 新增" : ""}
      </li>
    {/each}
    {#if perms.length === 0}
      <li class="none">无需任何系统能力</li>
    {/if}
  </ul>

  <div class="hash">SHA-256: {info.sha256}</div>

  <div class="actions">
    <button class="cancel" onclick={onCancel}>取消</button>
    <button class="ok" onclick={onApprove}>
      {info.is_upgrade ? "升级并授权" : "安装并授权"}
    </button>
  </div>
</div>

<style>
  .consent {
    padding: 14px 16px;
    color: #e8e8ea;
  }
  .head {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 12px;
  }
  .title {
    font-size: 16px;
    font-weight: 600;
  }
  .ver {
    color: var(--muted);
    font-size: 12px;
  }
  .section {
    font-size: 13px;
    color: var(--muted);
    margin-bottom: 6px;
  }
  .perms {
    list-style: none;
    margin: 0 0 12px;
    padding: 0;
  }
  .perms li {
    padding: 5px 10px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.05);
    margin-bottom: 4px;
    font-size: 13px;
  }
  .perms li.fresh {
    background: rgba(47, 111, 237, 0.25);
  }
  .perms li.none {
    color: var(--muted);
    background: none;
  }
  .hash {
    font-size: 11px;
    color: var(--muted);
    word-break: break-all;
    margin-bottom: 14px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  button {
    border: 0;
    border-radius: 6px;
    padding: 6px 14px;
    cursor: pointer;
    font-size: 13px;
  }
  .cancel {
    background: #3a3a3e;
    color: #e8e8ea;
  }
  .ok {
    background: var(--sel);
    color: #fff;
  }
</style>
```

- [ ] **Step 2: Type-check**

Run: `pnpm check 2>&1 | tail -10`
Expected: no errors in `InstallConsent.svelte`.

- [ ] **Step 3: Commit**

```bash
git add src/lib/InstallConsent.svelte
git commit -m "feat(plugins): install consent dialog component"
```

---

## Task 10: Frontend — `PluginManager.svelte` installed list + install entries

**Files:**
- Create: `src/lib/PluginManager.svelte`

- [ ] **Step 1: Create the component**

Create `src/lib/PluginManager.svelte`:

```svelte
<script lang="ts">
  import { permissionLabel } from "./permissions";
  import type { InstalledPlugin } from "./types";

  let {
    installed,
    onInstallFile,
    onInstallUrl,
    onUninstall,
  }: {
    installed: InstalledPlugin[];
    onInstallFile: () => void;
    onInstallUrl: (url: string) => void;
    onUninstall: (id: string) => void;
  } = $props();

  let url = $state("");
</script>

<div class="manager">
  <div class="bar">
    <button onclick={onInstallFile}>从文件安装</button>
    <input
      placeholder="粘贴 .pcp 链接后回车"
      bind:value={url}
      onkeydown={(e) => {
        if (e.key === "Enter" && url.trim()) {
          onInstallUrl(url.trim());
          url = "";
        }
      }}
    />
  </div>

  {#if installed.length === 0}
    <div class="empty">还没有安装任何插件。拖入 .pcp 文件，或从上面安装。</div>
  {/if}

  <ul class="list">
    {#each installed as p (p.id)}
      <li>
        <div class="row">
          <span class="name">{p.id}</span>
          <span class="ver">v{p.version} · {p.source}</span>
          <button class="rm" onclick={() => onUninstall(p.id)}>卸载</button>
        </div>
        <div class="perms">
          {p.granted.map(permissionLabel).join(" · ") || "无授权能力"}
        </div>
      </li>
    {/each}
  </ul>
</div>

<style>
  .manager {
    padding: 10px 12px;
    color: #e8e8ea;
  }
  .bar {
    display: flex;
    gap: 8px;
    margin-bottom: 10px;
  }
  .bar button {
    border: 0;
    background: var(--sel);
    color: #fff;
    border-radius: 6px;
    padding: 6px 12px;
    cursor: pointer;
    font-size: 13px;
    white-space: nowrap;
  }
  .bar input {
    flex: 1;
    border: 0;
    outline: 0;
    background: rgba(255, 255, 255, 0.06);
    color: #fff;
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 13px;
  }
  .empty {
    color: var(--muted);
    font-size: 13px;
    padding: 12px 4px;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .list li {
    padding: 8px 4px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .name {
    font-size: 14px;
  }
  .ver {
    color: var(--muted);
    font-size: 12px;
  }
  .rm {
    margin-left: auto;
    border: 0;
    background: #5a2b2b;
    color: #ffd9d9;
    border-radius: 6px;
    padding: 4px 10px;
    cursor: pointer;
    font-size: 12px;
  }
  .perms {
    color: var(--muted);
    font-size: 12px;
    margin-top: 3px;
  }
</style>
```

- [ ] **Step 2: Type-check**

Run: `pnpm check 2>&1 | tail -10`
Expected: no errors in `PluginManager.svelte`.

- [ ] **Step 3: Commit**

```bash
git add src/lib/PluginManager.svelte
git commit -m "feat(plugins): plugin manager view component"
```

---

## Task 11: Frontend — App.svelte wiring (modes, keyword, pipeline, drag-drop)

**Files:**
- Modify: `src/App.svelte`

- [ ] **Step 1: Import the new pieces + state**

In `src/App.svelte` `<script>`, add imports:

```ts
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import PluginManager from "./lib/PluginManager.svelte";
import InstallConsent from "./lib/InstallConsent.svelte";
import {
  inspectPackage,
  downloadPackage,
  installPackage,
  uninstallPlugin,
  listInstalled,
} from "./lib/host";
import type { InstalledPlugin, PackageInspect } from "./lib/types";
```

Extend the mode union and add state (near the other `$state`):

```ts
let mode: "search" | "plugin" | "manager" | "consent" = $state("search");
let installed: InstalledPlugin[] = $state([]);
let consentInfo: PackageInspect | null = $state(null);
let pendingPath: string | null = $state(null);
let pendingOrigin: string | null = $state(null);
let toast = $state("");
```

- [ ] **Step 2: Add the install pipeline + manager/uninstall functions**

Add these functions in the `<script>`:

```ts
async function refreshInstalled() {
  installed = await listInstalled();
}

function showToast(msg: string) {
  toast = msg;
  setTimeout(() => (toast = ""), 2000);
}

async function beginInstallFromPath(path: string, origin: string) {
  try {
    consentInfo = await inspectPackage(path);
    pendingPath = path;
    pendingOrigin = origin;
    mode = "consent";
  } catch (e) {
    showToast("无法读取插件包：" + String(e));
  }
}

async function installFromFile() {
  const picked = await openFileDialog({
    multiple: false,
    filters: [{ name: "pc-tool 插件", extensions: ["pcp"] }],
  });
  if (typeof picked === "string") await beginInstallFromPath(picked, picked);
}

async function installFromUrl(url: string) {
  try {
    const path = await downloadPackage(url);
    await beginInstallFromPath(path, url); // origin = the URL, not the temp file
  } catch (e) {
    showToast("下载失败：" + String(e));
  }
}

async function approveInstall() {
  if (!consentInfo || !pendingPath || !pendingOrigin) return;
  try {
    await installPackage(pendingPath, consentInfo.manifest.permissions, pendingOrigin);
    plugins = await listPlugins();
    await refreshInstalled();
    showToast(`已安装 ${consentInfo.manifest.name}`);
  } catch (e) {
    showToast("安装失败：" + String(e));
  }
  consentInfo = null;
  pendingPath = null;
  pendingOrigin = null;
  mode = "manager";
}

function cancelInstall() {
  consentInfo = null;
  pendingPath = null;
  pendingOrigin = null;
  mode = "manager";
}

async function doUninstall(id: string) {
  await uninstallPlugin(id);
  plugins = await listPlugins();
  await refreshInstalled();
  showToast("已卸载");
}

async function openManager() {
  query = "";
  results = [];
  await refreshInstalled();
  mode = "manager";
}

function exitManager() {
  mode = "search";
  query = "";
  computeResults("");
}
```

- [ ] **Step 3: Make manager/consent modes inert to the search bar, and route the `插件`/`plugins` keyword**

In `handleInput()`, the very first lines (after the existing `if (composing) return;`) become:

```ts
if (composing) return;
if (mode === "manager" || mode === "consent") return; // these views own their own inputs
if (mode === "plugin") {
  controller?.sendInput(query);
  return;
}
const q = query.trim();
const token = q.split(/\s+/)[0] ?? "";
if (token === "插件" || token === "plugins") {
  void openManager();
  return;
}
```

(This replaces the existing top of `handleInput` down to where `token` is computed; the keyword-feature lookup that follows stays unchanged.)

- [ ] **Step 4: Wire drag-drop in onMount**

Inside `onMount`, after the `pc:shown` listener, add:

```ts
await getCurrentWebview().onDragDropEvent((event) => {
  if (event.payload.type === "drop") {
    const file = event.payload.paths.find((p) => p.endsWith(".pcp"));
    if (file) void beginInstallFromPath(file, file);
  }
});
```

- [ ] **Step 5: Render the new modes**

In the markup, the resize `$effect` already tracks `mode`. Update the content area. Replace the top-level content conditional so it handles all four modes. After the `.bar` div, the body becomes:

```svelte
  {#if mode === "consent" && consentInfo}
    <InstallConsent
      info={consentInfo}
      onApprove={approveInstall}
      onCancel={cancelInstall}
    />
  {:else if mode === "manager"}
    <PluginManager
      {installed}
      onInstallFile={installFromFile}
      onInstallUrl={installFromUrl}
      onUninstall={doUninstall}
    />
  {:else if mode === "plugin"}
    <div class="content" class:hidden={activeFeatureType === "logic"}>
      <div class="plugin-host" bind:this={pluginHost}></div>
    </div>
    {#if activeFeatureType === "logic" && pluginResults.length > 0}
      <ul class="results">
        {#each pluginResults as r, i (i)}
          <li><span class="title">{r.title ?? r}</span><span class="sub">{r.subtitle ?? ""}</span></li>
        {/each}
      </ul>
    {/if}
  {:else if results.length > 0}
    <ul class="results">
      <!-- existing results list unchanged -->
    </ul>
  {/if}

  {#if toast}
    <div class="toast">{toast}</div>
  {/if}
```

(Keep the existing results `{#each}` body; only the surrounding `{#if}` chain changes.)

- [ ] **Step 6: Add a single `goBack()` and wire Esc + the back button**

Add a `goBack()` function in the `<script>`:

```ts
function goBack() {
  if (mode === "consent") cancelInstall();
  else if (mode === "manager") exitManager();
  else if (mode === "plugin") exitPlugin();
  else void hideWindow();
}
```

In `onKeydown`, replace the Escape branch body with a call to it:

```ts
if (e.key === "Escape") {
  e.preventDefault();
  goBack();
  return;
}
```

Show the back button for every non-search mode (change the bar condition from `mode === "plugin"` to `mode !== "search"`):

```svelte
{#if mode !== "search"}
  <button class="back" onclick={goBack} title="返回 (Esc)">←</button>
  <span class="label">{mode === "manager" ? "插件管理" : mode === "consent" ? "安装插件" : activeLabel}</span>
{/if}
```

- [ ] **Step 7: Add a toast style**

In the `<style>` block add:

```css
.toast {
  position: absolute;
  bottom: 10px;
  left: 50%;
  transform: translateX(-50%);
  background: #2f2f33;
  color: #fff;
  padding: 6px 14px;
  border-radius: 8px;
  font-size: 12px;
  white-space: nowrap;
}
```

- [ ] **Step 8: Build + type-check**

Run: `pnpm check 2>&1 | tail -10 && pnpm build 2>&1 | tail -4`
Expected: no new errors; built.

- [ ] **Step 9: Commit**

```bash
git add src/App.svelte
git commit -m "feat(plugins): manager + consent modes, install pipeline, drag-drop"
```

---

## Task 12: End-to-end verification (real app + webview stub)

**Files:** none (verification only)

- [ ] **Step 1: Pack the bundled plugin via the test helper**

Run: `cd src-tauri && cargo test plugins::tests::pack_then_unzip -- --nocapture` then copy the package:
`cp /var/folders/**/pctool-test-json-preview.pcp ~/Desktop/json-preview.pcp` (or note the temp path printed). This is the artifact you'll install.

- [ ] **Step 2: Launch the app**

Run (from repo root): `. "$HOME/.cargo/env" && node_modules/.bin/tauri dev` (background). Summon with `Cmd+Shift+Space`.

- [ ] **Step 3: Verify the acceptance checklist (spec §8)**

Walk each item, observing the real window:
- Drag `json-preview.pcp` onto the window → consent dialog lists 读取剪贴板 / 写入剪贴板 + SHA-256.
- Approve → toast 已安装; `ls ~/.config/pc-tool/plugins/` shows `com.pc-tool.json-preview`; `cat ~/.config/pc-tool/registry.json` shows `granted`.
- Type `插件` → manager lists the installed plugin with its permissions + source `installed`; Uninstall removes it (dir + registry entry gone).
- Install again via the URL box (serve the file: `python3 -m http.server` in the folder, paste `http://localhost:8000/json-preview.pcp`).
- Build a fake "escalation" copy: add a permission to a copy's plugin.json, repack, bump version, install over → consent highlights the new permission as 新增.

- [ ] **Step 4: Verify the granted-whitelist enforcement (webview stub)**

Reuse the deny-path verification pattern: stub `list_plugins` to return the json plugin with `granted: ["clipboard.read"]` but `permissions: ["clipboard.read","clipboard.write"]`, then confirm a `clipboard.write` call from the plugin is rejected by the host (never reaches `clipboard_write`). This proves the runtime honors `granted`, not `permissions`.

- [ ] **Step 5: Final commit (if any fixups were needed)**

```bash
git add -A
git commit -m "test(plugins): end-to-end install/uninstall/consent verified"
```

---

## Notes for the implementer
- Run Rust install/uninstall tests with `--test-threads=1` (they share the real `~/.config/pc-tool/registry.json` and the installed dir).
- The install tests mutate the real config dir; they clean up after themselves but if a run is interrupted, `rm -rf ~/.config/pc-tool/plugins/com.pc-tool.json-preview` and remove its entry from `registry.json`.
- `download_package` writes to a single temp path; that's fine for sequential installs. If you later support concurrent installs, make the temp name unique.
- Bundled vs installed: in dev, bundled = repo `plugins/`. Don't install a plugin whose id collides with a bundled one during testing, or both appear in `list_plugins`.
