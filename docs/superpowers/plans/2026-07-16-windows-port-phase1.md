# shu Windows 版・阶段 1「基础可用」实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Windows exe 装上即是可用启动器：应用发现（shell:AppsFolder + App Paths）、启动/打开、图标、toast 通知、剪贴板文件/图片探测、预览降级。

**Architecture:** 新增 `src-tauri/src/win/` 模块：纯逻辑（`logic.rs`，全平台编译+单测）与系统调用（`discovery/launch/icons/clipboard`，`cfg(windows)`）分离；`lib.rs` 现有命令内部按 `cfg` 分派，macOS 代码零改动，前端零改动（`AppEntry` 形状不变，UWP 的 `path` 存 `shell:AppsFolder\<AUMID>`）。

**Tech Stack:** `windows` 0.61（Tauri 传递依赖已有，锁文件同版本）、`winreg`、`tauri-plugin-notification`（仅 Windows 接线）、arboard（已有）。

**验证闭环:** 每个 task 结束跑 `cargo check`（保 macOS 不破）；Windows 侧编译由 CI windows job 兜底，若本地 `cargo check --target x86_64-pc-windows-msvc` 可用（Task 1 验证）则用它做快环。阶段完成 = CI 三平台绿 + tag `v0.2.0` 出 exe → 用户真机验收清单全过。

**注意:** 计划中 `windows` crate 调用签名按 0.61 文档书写，个别 Option/类型包装如与编译器不符，以编译器为准修正——这不算偏离计划。

---

### Task 1: 依赖、模块骨架、验证环境

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/win/mod.rs`
- Modify: `src-tauri/src/lib.rs:1`（`mod plugins;` 旁加 `mod win;`）

- [ ] **Step 1: Cargo.toml 加 Windows 依赖**（`[target.'cfg(target_os = "macos")'.dependencies]` 段后追加）

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.61", features = [
  "Win32_Foundation",
  "Win32_UI_Shell",
  "Win32_UI_Shell_Common",
  "Win32_UI_WindowsAndMessaging",
  "Win32_System_Com",
  "Win32_System_DataExchange",
  "Win32_System_Ole",
  "Win32_Graphics_Gdi",
] }
winreg = "0.55"
tauri-plugin-notification = "2"
```

- [ ] **Step 2: 建模块骨架** `src-tauri/src/win/mod.rs`

```rust
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
```

同时创建空的 `src-tauri/src/win/logic.rs`（内容 Task 2 填）：先放 `// Windows 纯逻辑（全平台编译）`。`lib.rs` 第 1 行 `mod plugins;` 后加 `mod win;`。

- [ ] **Step 3: 验证 macOS 编译不破**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 通过（win 子模块全部 cfg 掉，只剩空 logic）。

- [ ] **Step 4: 尝试本地交叉检查环（可选加速，失败不阻塞）**

Run: `rustup target add x86_64-pc-windows-msvc && cargo check --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc`
Expected: 若通过，后续每个 task 都用它当 Windows 快环；若 tauri-build 的资源编译（rc）报错，放弃本地交叉检查，Windows 编译全靠 CI windows job（每 task 结束 push 由 CI 验证）。把结论记在 commit message 里。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/win/ src-tauri/src/lib.rs
git commit -m "feat(win): add windows deps and win module skeleton"
```

---

### Task 2: 纯逻辑 win/logic.rs（TDD）

**Files:**
- Modify: `src-tauri/src/win/logic.rs`

App Paths 注册表键名 → 显示名，以及"补充源与主源按名字去重"。这是阶段 2 噪音过滤的落脚模块。

- [ ] **Step 1: 写失败测试**（logic.rs 底部）

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_strips_exe_suffix() {
        assert_eq!(display_name_from_exe_key("Notepad++.exe"), "Notepad++");
        assert_eq!(display_name_from_exe_key("code.EXE"), "code");
        assert_eq!(display_name_from_exe_key("noext"), "noext");
    }

    #[test]
    fn merge_drops_names_already_in_primary() {
        let primary: std::collections::HashSet<String> =
            ["visual studio code".to_string()].into_iter().collect();
        let cands = vec![
            ("Visual Studio Code".to_string(), r"C:\vsc\code.exe".to_string()),
            ("Notepad++".to_string(), r"C:\npp\notepad++.exe".to_string()),
        ];
        let kept = merge_supplement(&primary, cands);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].0, "Notepad++");
    }
}
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml win::logic`
Expected: 编译失败，`display_name_from_exe_key` 未定义。

- [ ] **Step 3: 最小实现**（logic.rs 顶部）

```rust
/// App Paths 子键名（如 "Notepad++.exe"）→ 显示名。
pub fn display_name_from_exe_key(key: &str) -> String {
    let lower = key.to_lowercase();
    if lower.ends_with(".exe") {
        key[..key.len() - 4].to_string()
    } else {
        key.to_string()
    }
}

/// 补充源候选 (名, 路径) 里剔除主源（小写名集合）已有的项。
pub fn merge_supplement(
    primary_lower: &std::collections::HashSet<String>,
    candidates: Vec<(String, String)>,
) -> Vec<(String, String)> {
    candidates
        .into_iter()
        .filter(|(name, _)| !primary_lower.contains(&name.to_lowercase()))
        .collect()
}
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml win::logic`
Expected: 2 passed。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/win/logic.rs
git commit -m "feat(win): app-paths name derivation and supplement merge logic"
```

---

### Task 3: 应用发现 win/discovery.rs + list_apps 分派

**Files:**
- Create: `src-tauri/src/win/discovery.rs`
- Modify: `src-tauri/src/lib.rs`（`list_apps`，约 151 行）

- [ ] **Step 1: 实现 discovery.rs**

```rust
//! 应用发现：主源 shell:AppsFolder（Win32 + UWP 统一枚举，本地化显示名），
//! 补充源注册表 App Paths。COM 每次防御性初始化（S_FALSE 幂等）。
use std::collections::HashSet;

use windows::core::{w, Interface, PCWSTR};
use windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::{
    BHID_EnumItems, IEnumShellItems, IShellItem, SHCreateItemFromParsingName,
    SIGDN_NORMALDISPLAY, SIGDN_PARENTRELATIVEPARSING,
};

use crate::AppEntry;

fn sigdn_string(item: &IShellItem, kind: windows::Win32::UI::Shell::SIGDN) -> Option<String> {
    unsafe {
        let p = item.GetDisplayName(kind).ok()?;
        let s = p.to_string().ok();
        CoTaskMemFree(Some(p.0 as _));
        s
    }
}

/// 枚举 shell:AppsFolder：返回 (显示名, "shell:AppsFolder\<解析名>")。
fn apps_folder_entries() -> Vec<(String, String)> {
    let mut out = Vec::new();
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let Ok(folder): windows::core::Result<IShellItem> =
            SHCreateItemFromParsingName(w!("shell:AppsFolder"), None)
        else {
            return out;
        };
        let Ok(enumerator): windows::core::Result<IEnumShellItems> =
            folder.BindToHandler(None, &BHID_EnumItems)
        else {
            return out;
        };
        loop {
            let mut items: [Option<IShellItem>; 1] = [None];
            let mut fetched = 0u32;
            if enumerator.Next(&mut items, Some(&mut fetched)).is_err() || fetched == 0 {
                break;
            }
            let Some(item) = items[0].take() else { break };
            let (Some(name), Some(parsing)) = (
                sigdn_string(&item, SIGDN_NORMALDISPLAY),
                sigdn_string(&item, SIGDN_PARENTRELATIVEPARSING),
            ) else {
                continue;
            };
            if name.is_empty() || parsing.is_empty() {
                continue;
            }
            out.push((name, format!("shell:AppsFolder\\{parsing}")));
        }
    }
    out
}

/// 注册表 App Paths（HKLM + HKCU）：返回 (显示名, exe 全路径)，路径不存在的剔除。
fn app_paths_entries() -> Vec<(String, String)> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;
    const SUBKEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths";
    let mut out = Vec::new();
    for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        let Ok(root) = RegKey::predef(hive).open_subkey(SUBKEY) else {
            continue;
        };
        for key in root.enum_keys().flatten() {
            let Ok(sub) = root.open_subkey(&key) else { continue };
            let Ok(path): std::io::Result<String> = sub.get_value("") else {
                continue;
            };
            let path = path.trim_matches('"').to_string();
            if path.is_empty() || !std::path::Path::new(&path).exists() {
                continue;
            }
            out.push((super::logic::display_name_from_exe_key(&key), path));
        }
    }
    out
}

pub fn list_apps() -> Vec<AppEntry> {
    let primary = apps_folder_entries();
    let primary_names: HashSet<String> =
        primary.iter().map(|(n, _)| n.to_lowercase()).collect();
    let supplement = super::logic::merge_supplement(&primary_names, app_paths_entries());
    primary
        .into_iter()
        .chain(supplement)
        .map(|(name, path)| AppEntry { name, path, pinyin: None, initials: None })
        .collect()
}
```

（`PCWSTR`、`Interface` 若未用上会有 unused 警告——按编译器提示删；`SHCreateItemFromParsingName` 在 0.61 是泛型返回，`let Ok(x): windows::core::Result<T>` 写法如不被接受改为 turbofish `SHCreateItemFromParsingName::<_, IShellItem>` 或普通 match，以编译器为准。）

- [ ] **Step 2: lib.rs 的 list_apps 分派**（排序去重逻辑两平台共用，保持现状）

```rust
#[tauri::command]
fn list_apps() -> Vec<AppEntry> {
    #[cfg(target_os = "windows")]
    let mut out = win::discovery::list_apps();
    #[cfg(not(target_os = "windows"))]
    let mut out = {
        let mut v = Vec::new();
        for dir in app_dirs() {
            collect_apps(&dir, 1, &mut v);
        }
        v
    };
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out.dedup_by(|a, b| a.path == b.path);
    out
}
```

- [ ] **Step 3: 验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`（macOS 不破）；若 Task 1 交叉环可用再跑 `--target x86_64-pc-windows-msvc`。
Expected: 均通过。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/win/discovery.rs src-tauri/src/lib.rs
git commit -m "feat(win): app discovery via shell:AppsFolder + App Paths registry"
```

---

### Task 4: 启动/打开 win/launch.rs + 四个命令分派

**Files:**
- Create: `src-tauri/src/win/launch.rs`
- Modify: `src-tauri/src/lib.rs`（`launch_app_blocking` 约 163 行、`open_url`/`open_path` 约 415-431 行、`image_preview` 约 965 行）

- [ ] **Step 1: 实现 launch.rs**

```rust
//! ShellExecuteW 包装：应用（含 shell:AppsFolder\AUMID）、URL、路径统一入口。
use windows::core::PCWSTR;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// ShellExecuteW "open"。返回值 ≤32 为错误码。
pub fn shell_open(target: &str) -> Result<(), String> {
    let w_target = wide(target);
    let ret = unsafe {
        ShellExecuteW(
            None,
            PCWSTR::null(),
            PCWSTR(w_target.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if ret.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!("ShellExecute 失败（{}）：{}", ret.0 as isize, target))
    }
}
```

- [ ] **Step 2: lib.rs 分派**。`launch_app_blocking` 整体改为：

```rust
/// `open` 的错误（应用不存在等）发生在进程退出时，须等 status 才能上报给前端。
fn launch_app_blocking(path: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    return win::launch::shell_open(path);
    #[cfg(not(target_os = "windows"))]
    {
        let out = Command::new("open")
            .arg(path)
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            return Ok(());
        }
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if err.is_empty() {
            format!("open 退出码 {}", out.status.code().unwrap_or(-1))
        } else {
            err
        })
    }
}
```

`open_url` 与 `open_path` 同法：函数体首行加 `#[cfg(target_os = "windows")] return win::launch::shell_open(&url);`（`open_path` 为 `&path`），原 `Command::new("open")` 体包进 `#[cfg(not(target_os = "windows"))] { ... }`。

`image_preview` 内 `qlmanage` 调用处同法分派：Windows 分支写完临时 PNG 后 `win::launch::shell_open(&path.to_string_lossy())`（默认看图器打开，降级方案），mac 分支原样。

- [ ] **Step 3: 验证** — 同 Task 3 Step 3。
- [ ] **Step 4: Commit** — `git commit -m "feat(win): launch/open/preview via ShellExecuteW"`

---

### Task 5: 图标 win/icons.rs + icon_data_url 分派

**Files:**
- Create: `src-tauri/src/win/icons.rs`
- Modify: `src-tauri/src/lib.rs`（`icon_data_url` 约 251-280 行）

- [ ] **Step 1: 实现 icons.rs**

```rust
//! IShellItemImageFactory：对普通路径与 shell:AppsFolder\AUMID 统一出 64px 图标。
use windows::core::PCWSTR;
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDC, GetDIBits, ReleaseDC, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS,
};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::{IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_ICONONLY};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn icon_png(path: &str) -> Option<Vec<u8>> {
    const N: i32 = 64;
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let w_path = wide(path);
        let factory: IShellItemImageFactory =
            SHCreateItemFromParsingName(PCWSTR(w_path.as_ptr()), None).ok()?;
        let hbmp = factory.GetImage(SIZE { cx: N, cy: N }, SIIGBF_ICONONLY).ok()?;

        // 32bpp top-down DIB 拉出 BGRA 像素。
        let hdc = GetDC(None);
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: N,
                biHeight: -N, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut buf = vec![0u8; (N * N * 4) as usize];
        let lines = GetDIBits(
            hdc,
            hbmp,
            0,
            N as u32,
            Some(buf.as_mut_ptr() as *mut _),
            &mut info,
            DIB_RGB_COLORS,
        );
        ReleaseDC(None, hdc);
        let _ = DeleteObject(hbmp.into());
        if lines == 0 {
            return None;
        }
        // BGRA -> RGBA
        for px in buf.chunks_exact_mut(4) {
            px.swap(0, 2);
        }
        let img = image::RgbaImage::from_raw(N as u32, N as u32, buf)?;
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .ok()?;
        Some(png)
    }
}
```

- [ ] **Step 2: lib.rs 复用缓存逻辑**。`icon_data_url` 的外层 `#[cfg(target_os = "macos")]` 块改为 `#[cfg(any(target_os = "macos", target_os = "windows"))]`（缓存读写 + `icon_png` 调用两平台共用），兜底块相应改 `#[cfg(not(any(target_os = "macos", target_os = "windows")))]`。并新增：

```rust
#[cfg(target_os = "windows")]
fn icon_png(path: &str) -> Option<Vec<u8>> {
    win::icons::icon_png(path)
}
```

- [ ] **Step 3: 验证** — 同 Task 3 Step 3。
- [ ] **Step 4: Commit** — `git commit -m "feat(win): app icons via IShellItemImageFactory, shared disk cache"`

---

### Task 6: 通知 tauri-plugin-notification（仅 Windows 接线）

**Files:**
- Modify: `src-tauri/src/lib.rs`（`notify` 约 691 行；`run()` 里 Builder 链起点约 1100 行前）

- [ ] **Step 1: 注册插件（仅 Windows）**。`run()`（lib.rs:1082）现为 `tauri::Builder::default().manage(...).plugin(tauri_plugin_dialog::init())...` 一条长链，起链处改为：

```rust
let builder = tauri::Builder::default();
#[cfg(target_os = "windows")]
let builder = builder.plugin(tauri_plugin_notification::init());
builder
    .manage(AutoHide(std::sync::atomic::AtomicBool::new(true)))
    .plugin(tauri_plugin_dialog::init())
    // ……链其余部分逐字不动
```

- [ ] **Step 2: notify 命令分派**（签名加 `app`，前端零改动；macOS 体原样包 cfg）：

```rust
#[tauri::command]
fn notify(app: tauri::AppHandle, title: String, body: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use tauri_plugin_notification::NotificationExt;
        return app
            .notification()
            .builder()
            .title(&title)
            .body(&body)
            .show()
            .map_err(|e| e.to_string());
    }
    #[cfg(target_os = "macos")]
    {
        let _ = &app;
        let script = format!(
            "display notification {} with title {}",
            osa_quote(&body),
            osa_quote(&title)
        );
        Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .status()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let _ = (&app, &title, &body);
        Err("not supported".into())
    }
}
```

（Rust 侧调用插件不走 IPC 权限，`capabilities/default.json` 无需改动。）

- [ ] **Step 3: 验证** — `cargo check` 两侧；macOS 侧确认 notify 仍编译（`app` 参数未破坏现有调用——前端 invoke 参数不含 app，Tauri 自动注入）。
- [ ] **Step 4: Commit** — `git commit -m "feat(win): toast notifications via tauri-plugin-notification"`

---

### Task 7: 剪贴板文件/图片探测 win/clipboard.rs

**Files:**
- Create: `src-tauri/src/win/clipboard.rs`
- Modify: `src-tauri/src/lib.rs`（`clipboard_read_files` 非 mac 版约 338 行、`clipboard_image_present` 非 mac 版约 376 行）

- [ ] **Step 1: 实现 clipboard.rs**

```rust
//! CF_HDROP 文件列表读取 + 位图格式探测（只探格式不解码，对齐 macOS 版语义）。
use windows::Win32::Foundation::{HANDLE, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
};
use windows::Win32::System::Ole::{CF_BITMAP, CF_DIB, CF_HDROP};
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};

pub fn read_files() -> Vec<String> {
    let mut out = Vec::new();
    unsafe {
        if OpenClipboard(HWND::default()).is_err() {
            return out;
        }
        if let Ok(handle) = GetClipboardData(CF_HDROP.0 as u32) {
            let hdrop = HDROP(handle.0);
            let count = DragQueryFileW(hdrop, u32::MAX, None);
            for i in 0..count {
                let len = DragQueryFileW(hdrop, i, None);
                if len == 0 {
                    continue;
                }
                let mut buf = vec![0u16; (len + 1) as usize];
                let got = DragQueryFileW(hdrop, i, Some(&mut buf));
                if got > 0 {
                    out.push(String::from_utf16_lossy(&buf[..got as usize]));
                }
            }
        }
        let _ = CloseClipboard();
    }
    out
}

pub fn image_present() -> bool {
    unsafe {
        IsClipboardFormatAvailable(CF_DIB.0 as u32).is_ok()
            || IsClipboardFormatAvailable(CF_BITMAP.0 as u32).is_ok()
    }
}
```

（`HANDLE`/`HWND` 包装、`GetClipboardData` 返回类型按 0.61 实际签名微调；`HWND::default()` 如不成立用 `None`。）

- [ ] **Step 2: lib.rs 两个非 mac 存根分派**

```rust
#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn clipboard_read_files() -> Vec<String> {
    #[cfg(target_os = "windows")]
    return win::clipboard::read_files();
    #[cfg(not(target_os = "windows"))]
    Vec::new()
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn clipboard_image_present() -> bool {
    #[cfg(target_os = "windows")]
    return win::clipboard::image_present()
        || clipboard_read_files().iter().any(|p| is_image_path(p));
    #[cfg(not(target_os = "windows"))]
    false
}
```

- [ ] **Step 3: 验证** — 同 Task 3 Step 3。
- [ ] **Step 4: Commit** — `git commit -m "feat(win): clipboard file list (CF_HDROP) and image presence probe"`

---

### Task 8: CI 加 cargo test、版本 bump、出验收 exe

**Files:**
- Modify: `.github/workflows/release.yml`（windows job 加测试步）
- Modify: `src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`（version 0.1.0 → 0.2.0）、`package.json`（同步 0.2.0）

- [ ] **Step 1: workflow 的 `pnpm install` 步后加**（matrix 全平台跑，logic 测试很快）：

```yaml
      - run: cargo test --manifest-path src-tauri/Cargo.toml
```

- [ ] **Step 2: 三处版本号 bump 0.2.0；`pnpm test` 全套过一遍（前端脚本测试不受影响确认）**

Run: `pnpm test && cargo test --manifest-path src-tauri/Cargo.toml`
Expected: 全绿。

- [ ] **Step 3: Commit + tag + push**

```bash
git add -A && git commit -m "chore: bump 0.2.0, run cargo test in CI"
git push origin main
git tag v0.2.0 && git push origin v0.2.0
```

- [ ] **Step 4: 盯 CI 到三平台绿**（失败读日志修复后：删草稿 Release + 删远端 tag + 重打，避免资产名冲突——教训已验）

Run: `gh run watch <id> --exit-status`
Expected: 三 job 绿，Release 草稿含 exe + 2 dmg。

- [ ] **Step 5: 交用户真机验收**，清单：

1. exe 安装（SmartScreen「仍要运行」预期内）
2. 托盘图标出现；Alt+Space（或设置的热键）唤出/隐藏窗口
3. 应用列表非空：含 UWP（如"设置"「计算器」）与 Win32（如 Chrome/微信）
4. 中文应用显示名正确；启动 Win32 与 UWP 应用各一个成功
5. 应用图标正常渲染（首次略慢，二次即缓存）
6. translator 插件可用；image-compressor 压缩可用、预览用默认看图器打开
7. 复制一张截图 → 唤出 shu → 图片推荐出现（clipboard_image_present）
8. 复制一个图片文件 → 同上（CF_HDROP 路径）
9. 任一插件触发通知 → Windows toast 弹出

---

## 阶段 2 / 3

见 spec（`docs/superpowers/specs/2026-07-16-windows-port-design.md`），各自单独出计划：阶段 2（Everything 捆绑 + 绿色软件发现）待阶段 1 验收通过后启动；阶段 3（拼音 / hosts / CF_HDROP 写入 / 预览评估）随后。
