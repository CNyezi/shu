import { invoke } from "@tauri-apps/api/core";

export type Settings = {
  hotkey?: string;
  /** pluginId -> 是否允许剪贴板内容匹配时自动打开（默认 true） */
  autoOpen?: Record<string, boolean>;
};

export const readSettings = () => invoke<Settings>("settings_read");
export const writeSettings = (value: Settings) => invoke<void>("settings_write", { value });
export const setHotkey = (hotkey: string) => invoke<void>("set_hotkey", { hotkey });
export const DEFAULT_HOTKEY = "super+shift+space";
