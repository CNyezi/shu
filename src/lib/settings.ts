import { invoke } from "@tauri-apps/api/core";

export type Settings = {
  hotkey?: string;
  /** pluginId -> 是否允许剪贴板内容匹配时自动打开（默认 true） */
  autoOpen?: Record<string, boolean>;
  /** 启动时自动检查应用更新（默认 true） */
  autoUpdateCheck?: boolean;
};

export const readSettings = () => invoke<Settings>("settings_read");
export const writeSettings = (value: Settings) => invoke<void>("settings_write", { value });
export const setHotkey = (hotkey: string) => invoke<void>("set_hotkey", { hotkey });
// 与 Rust 侧 DEFAULT_HOTKEY 的平台分叉保持一致（Windows 用 alt+space）。
export const DEFAULT_HOTKEY = navigator.userAgent.includes("Windows")
  ? "alt+space"
  : "super+shift+space";
