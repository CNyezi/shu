import { invoke } from "@tauri-apps/api/core";
import type { AppEntry, Plugin } from "./types";

// --- Core (host-only) APIs ---
export const listApps = () => invoke<AppEntry[]>("list_apps");
export const launchApp = (path: string) => invoke<void>("launch_app", { path });
export const appIcon = (path: string) => invoke<string | null>("app_icon", { path });
export const listPlugins = () => invoke<Plugin[]>("list_plugins");
export const readPluginFile = (dir: string, rel: string) =>
  invoke<string>("read_plugin_file", { dir, rel });
export const hideWindow = () => invoke<void>("hide_window");
export const readClipboard = () =>
  invoke<{ kind: string; text: string }>("clipboard_read");

// --- System capabilities exposed to plugins (mediated by the host shell) ---
// Each entry maps a permission name to its Rust implementation. The plugin
// runtime only dispatches here AFTER checking the plugin's permission whitelist.
export const capabilities: Record<string, (args: any) => Promise<unknown>> = {
  "clipboard.read": () => invoke("clipboard_read"),
  "clipboard.write": (a) => invoke("clipboard_write", { text: a.text }),
  "shell.openUrl": (a) => invoke("open_url", { url: a.url }),
  "shell.openPath": (a) => invoke("open_path", { path: a.path }),
};
