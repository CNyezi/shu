//! Windows 平台实现。logic 为纯逻辑（全平台编译、可单测），其余为系统调用。
pub mod logic;

#[cfg(target_os = "windows")]
pub mod clipboard;
#[cfg(target_os = "windows")]
pub mod discovery;
#[cfg(target_os = "windows")]
pub mod icons;
#[cfg(target_os = "windows")]
pub mod launch;
#[cfg(target_os = "windows")]
pub mod single_instance;
