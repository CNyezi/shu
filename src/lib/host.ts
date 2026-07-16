import { invoke } from "@tauri-apps/api/core";
import type { AppEntry, InstalledPlugin, PackageInspect, Plugin, RegistryFeed } from "./types";
export { canUseCapability, capabilityPermission, effectivePermissions } from "./capabilities";

// --- Core (host-only) APIs ---
export const listApps = () => invoke<AppEntry[]>("list_apps");
export const launchApp = (path: string) => invoke<void>("launch_app", { path });
export const appIcon = (path: string) => invoke<string | null>("app_icon", { path });
export const listPlugins = () => invoke<Plugin[]>("list_plugins");
export const readPluginFile = (dir: string, rel: string) =>
  invoke<string>("read_plugin_file", { dir, rel });
export const readPluginIcon = (dir: string, rel: string) =>
  invoke<string>("read_plugin_icon", { dir, rel });
export const hideWindow = () => invoke<void>("hide_window");
export const setAutoHide = (enabled: boolean) =>
  invoke<void>("set_auto_hide", { enabled });
export const readClipboard = () =>
  invoke<{ kind: string; text: string }>("clipboard_read");
export const writeClipboard = (text: string) => invoke<void>("clipboard_write", { text });
export const clipboardImagePresent = () => invoke<boolean>("clipboard_image_present");

export const inspectPackage = (path: string) =>
  invoke<PackageInspect>("inspect_package", { path });
export const downloadPackage = (url: string) =>
  invoke<string>("download_package", { url });
export const installPackage = (path: string, granted: string[], origin: string) =>
  invoke<void>("install_package", { path, granted, origin });
export const uninstallPlugin = (id: string) =>
  invoke<void>("uninstall_plugin", { id });
export const listInstalled = () => invoke<InstalledPlugin[]>("list_installed");
export const checkForUpdates = () => invoke<string>("check_for_updates");
export const listRegistries = () => invoke<string[]>("list_registries");
export const addRegistry = (url: string) => invoke<void>("add_registry", { url });
export const removeRegistry = (url: string) => invoke<void>("remove_registry", { url });
export const fetchRegistry = (url: string) => invoke<RegistryFeed>("fetch_registry", { url });
export const downloadPackageChecked = (url: string, sha256: string) =>
  invoke<string>("download_package_checked", { url, sha256 });

// --- System capabilities exposed to plugins (mediated by the host shell) ---
// Each entry maps a permission name to its Rust implementation. The plugin
// runtime only dispatches here AFTER checking the plugin's permission whitelist.
export const capabilities: Record<string, (args: any) => Promise<unknown>> = {
  "clipboard.read": () => invoke("clipboard_read"),
  "clipboard.write": (a) => invoke("clipboard_write", { text: a.text }),
  "clipboard.readImage": () => invoke("clipboard_read_image"),
  "clipboard.writeImage": (a) => invoke("clipboard_write_image", { dataUrl: a.dataUrl }),
  "clipboard.readFiles": () => invoke("clipboard_read_files"),
  "clipboard.writeFiles": (a) => invoke("clipboard_write_files", { paths: a.paths }),
  "shell.openUrl": (a) => invoke("open_url", { url: a.url }),
  "shell.openPath": (a) => invoke("open_path", { path: a.path }),
  "hosts.read": () => invoke("hosts_read"),
  "hosts.write": (a) => invoke("hosts_write", { content: a.content }),
  // fs methods are scope-enforced in Rust; the bridge injects `granted` + `pluginId`.
  "fs.scopes": (a) => invoke("fs_scopes", { granted: a.granted, pluginId: a.pluginId }),
  "fs.readText": (a) => invoke("fs_read_text", { path: a.path, granted: a.granted, pluginId: a.pluginId }),
  "fs.readBytes": (a) => invoke("fs_read_bytes", { path: a.path, granted: a.granted, pluginId: a.pluginId }),
  "fs.list": (a) => invoke("fs_list", { path: a.path, granted: a.granted, pluginId: a.pluginId }),
  "fs.exists": (a) => invoke("fs_exists", { path: a.path, granted: a.granted, pluginId: a.pluginId }),
  "fs.stat": (a) => invoke("fs_stat", { path: a.path, granted: a.granted, pluginId: a.pluginId }),
  "fs.writeText": (a) =>
    invoke("fs_write_text", { path: a.path, content: a.content, granted: a.granted, pluginId: a.pluginId }),
  "fs.writeBytes": (a) =>
    invoke("fs_write_bytes", { path: a.path, base64Data: a.base64Data, granted: a.granted, pluginId: a.pluginId }),
  "fs.mkdir": (a) => invoke("fs_mkdir", { path: a.path, granted: a.granted, pluginId: a.pluginId }),
  "fs.remove": (a) => invoke("fs_remove", { path: a.path, granted: a.granted, pluginId: a.pluginId }),
  notification: (a) => invoke("notify", { title: a.title, body: a.body }),
  "network.http": (a) =>
    invoke("http_request", { url: a.url, method: a.method, headers: a.headers, body: a.body }),
  "image.compress": (a) => invoke("image_compress", { source: a.source, quality: a.quality }),
  "dialog.saveFile": (a) =>
    invoke("save_file_dialog", { defaultPath: a.defaultPath, base64Data: a.base64Data }),
  "dialog.saveFiles": (a) =>
    invoke("save_files_dialog", { defaultDir: a.defaultDir, files: a.files }),
  "image.preview": (a) => invoke("image_preview", { base64Data: a.base64Data }),
  "image.read": (a) => invoke("image_read", { path: a.path }),
};

// Per-plugin storage — no permission; the host injects the (trusted) plugin id.
export const storageGet = (pluginId: string, key: string) =>
  invoke("plugin_storage_get", { pluginId, key });
export const storageSet = (pluginId: string, key: string, value: unknown) =>
  invoke<void>("plugin_storage_set", { pluginId, key, value });
export const storageRemove = (pluginId: string, key: string) =>
  invoke<void>("plugin_storage_remove", { pluginId, key });
export const storageKeys = (pluginId: string) =>
  invoke<string[]>("plugin_storage_keys", { pluginId });
