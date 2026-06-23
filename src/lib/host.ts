import { invoke } from "@tauri-apps/api/core";
import type { AppEntry, InstalledPlugin, PackageInspect, Plugin } from "./types";

// --- Core (host-only) APIs ---
export const listApps = () => invoke<AppEntry[]>("list_apps");
export const launchApp = (path: string) => invoke<void>("launch_app", { path });
export const appIcon = (path: string) => invoke<string | null>("app_icon", { path });
export const listPlugins = () => invoke<Plugin[]>("list_plugins");
export const readPluginFile = (dir: string, rel: string) =>
  invoke<string>("read_plugin_file", { dir, rel });
export const hideWindow = () => invoke<void>("hide_window");
export const setAutoHide = (enabled: boolean) =>
  invoke<void>("set_auto_hide", { enabled });
export const readClipboard = () =>
  invoke<{ kind: string; text: string }>("clipboard_read");

export const inspectPackage = (path: string) =>
  invoke<PackageInspect>("inspect_package", { path });
export const downloadPackage = (url: string) =>
  invoke<string>("download_package", { url });
export const installPackage = (path: string, granted: string[], origin: string) =>
  invoke<void>("install_package", { path, granted, origin });
export const uninstallPlugin = (id: string) =>
  invoke<void>("uninstall_plugin", { id });
export const listInstalled = () => invoke<InstalledPlugin[]>("list_installed");

// --- System capabilities exposed to plugins (mediated by the host shell) ---
// Each entry maps a permission name to its Rust implementation. The plugin
// runtime only dispatches here AFTER checking the plugin's permission whitelist.
export const capabilities: Record<string, (args: any) => Promise<unknown>> = {
  "clipboard.read": () => invoke("clipboard_read"),
  "clipboard.write": (a) => invoke("clipboard_write", { text: a.text }),
  "shell.openUrl": (a) => invoke("open_url", { url: a.url }),
  "shell.openPath": (a) => invoke("open_path", { path: a.path }),
  "hosts.read": () => invoke("hosts_read"),
  "hosts.write": (a) => invoke("hosts_write", { content: a.content }),
};
