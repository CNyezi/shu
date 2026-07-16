# shu Windows 版・阶段 1「基础可用」实施计划

> **修订版（grilling 后）**：噪音过滤进阶段 1；App Paths 已砍（AppsFolder 单源）；新增手写单实例；Windows 默认热键 Alt+Space；不做列表缓存。Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Windows exe 装上即是可用启动器：应用发现（shell:AppsFolder 单源 + 噪音过滤）、启动/打开、图标、toast 通知、剪贴板文件/图片探测、预览降级、单实例、Alt+Space 热键。

**Architecture:** 新增 `src-tauri/src/win/` 模块：纯逻辑（`logic.rs`，全平台编译+单测）与系统调用（`discovery/launch/icons/clipboard/single_instance`，`cfg(windows)`）分离；`lib.rs` 现有命令内部按 `cfg` 分派，macOS 代码零改动，前端零改动（`AppEntry` 形状不变，UWP 的 `path` 存 `shell:AppsFolder\<AUMID>`）。

**Tech Stack:** `windows` 0.61（Tauri 传递依赖已有同版本）、`tauri-plugin-notification`（仅 Windows 接线）、arboard（已有）。

**验证闭环:** 每 task 结束跑 `cargo check`（保 macOS）；Windows 编译由 CI windows job 兜底，若本地 `cargo check --target x86_64-pc-windows-msvc` 可用（Task 1 验证）则作快环。阶段完成 = CI 三平台绿 + tag `v0.2.0` 出 exe → 用户真机验收清单全过。

**注意:** `windows` crate 调用签名按 0.61 文档书写，Option/类型包装与编译器不符时以编译器为准修正——不算偏离计划。

---

### Task 1: 依赖、模块骨架、验证环境

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/win/mod.rs`、`src-tauri/src/win/logic.rs`（空壳）
- Modify: `src-tauri/src/lib.rs:1`（`mod plugins;` 旁加 `mod win;`）

- [ ] **Step 1: Cargo.toml 加 Windows 依赖**（macOS target 段后追加；无 winreg——App Paths 已砍）

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
  "Win32_System_Threading",
  "Win32_Graphics_Gdi",
] }
tauri-plugin-notification = "2"
```

- [ ] **Step 2: 骨架** `src-tauri/src/win/mod.rs`：

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
#[cfg(target_os = "windows")]
pub mod single_instance;
```

`logic.rs` 先放注释占位；`lib.rs` 第 1 行 `mod plugins;` 后加 `mod win;`。（cfg 掉的子模块文件随各 task 创建。）

- [ ] **Step 3:** `cargo check --manifest-path src-tauri/Cargo.toml` → 通过。
- [ ] **Step 4: 尝试本地交叉环（可选，失败不阻塞）** `rustup target add x86_64-pc-windows-msvc && cargo check --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc`。若 tauri-build 资源编译（rc）报错则放弃，Windows 编译全靠 CI；结论记进 commit message。
- [ ] **Step 5: Commit** `feat(win): add windows deps and win module skeleton`

---

### Task 2: 纯逻辑 win/logic.rs——噪音过滤（TDD）

**Files:**
- Modify: `src-tauri/src/win/logic.rs`

AppsFolder 里混着「卸载 XXX」「XXX 帮助」「XXX 官网」类快捷方式，按名字剔除。（App Paths 的 `display_name_from_exe_key`/`merge_supplement` 已随源砍掉，阶段 2 Everything 补充源需要时再写——不为未来船运死代码。）

- [ ] **Step 1: 写失败测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_entries_filtered() {
        for name in [
            "卸载 微信", "Uninstall Foo", "foo uninstaller",
            "Node.js documentation", "VLC 帮助", "Epic 官网",
            "README", "website of thing",
        ] {
            assert!(is_noise_entry(name), "should filter: {name}");
        }
    }

    #[test]
    fn real_apps_kept() {
        for name in ["微信", "Google Chrome", "Visual Studio Code", "设置", "Everything"] {
            assert!(!is_noise_entry(name), "should keep: {name}");
        }
    }
}
```

- [ ] **Step 2:** `cargo test --manifest-path src-tauri/Cargo.toml win::logic` → 编译失败（函数未定义）。
- [ ] **Step 3: 最小实现**

```rust
//! Windows 纯逻辑（全平台编译，可单测）。

/// 开始菜单快捷方式里的非应用条目：卸载器、帮助、官网链接等。
/// ponytail: 关键词表打底，真机验收发现漏网再补词。
pub fn is_noise_entry(name: &str) -> bool {
    const NOISE: &[&str] = &[
        "卸载", "uninstall", "帮助", "help", "文档", "documentation", "docs",
        "官网", "website", "readme", "release notes", "更新日志", "license",
    ];
    let lower = name.to_lowercase();
    NOISE.iter().any(|kw| lower.contains(kw))
}
```

- [ ] **Step 4:** 同 Step 2 命令 → 2 passed。
- [ ] **Step 5: Commit** `feat(win): start-menu noise entry filter`

---

### Task 3: 应用发现 win/discovery.rs + list_apps 分派

**Files:**
- Create: `src-tauri/src/win/discovery.rs`
- Modify: `src-tauri/src/lib.rs`（`list_apps`，约 151 行）

- [ ] **Step 1: discovery.rs**（AppsFolder 单源 + 噪音过滤）

```rust
//! 应用发现：shell:AppsFolder 枚举（Win32 + UWP 统一、显示名本地化）。
//! COM 每次防御性初始化（S_FALSE 幂等）。
use windows::core::w;
use windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::{
    BHID_EnumItems, IEnumShellItems, IShellItem, SHCreateItemFromParsingName,
    SIGDN, SIGDN_NORMALDISPLAY, SIGDN_PARENTRELATIVEPARSING,
};

use crate::AppEntry;

fn sigdn_string(item: &IShellItem, kind: SIGDN) -> Option<String> {
    unsafe {
        let p = item.GetDisplayName(kind).ok()?;
        let s = p.to_string().ok();
        CoTaskMemFree(Some(p.0 as _));
        s
    }
}

pub fn list_apps() -> Vec<AppEntry> {
    let mut out = Vec::new();
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let Ok(folder) = SHCreateItemFromParsingName::<_, IShellItem>(w!("shell:AppsFolder"), None) else {
            return out;
        };
        let Ok(enumerator) = folder.BindToHandler::<IEnumShellItems>(None, &BHID_EnumItems) else {
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
            if name.is_empty() || parsing.is_empty() || super::logic::is_noise_entry(&name) {
                continue;
            }
            out.push(AppEntry {
                name,
                path: format!("shell:AppsFolder\\{parsing}"),
                pinyin: None,
                initials: None,
            });
        }
    }
    out
}
```

（泛型 turbofish 形参个数、`BindToHandler` 泛型写法以编译器为准。）

- [ ] **Step 2: lib.rs 分派**（排序去重共用）：

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

- [ ] **Step 3:** `cargo check`（+ 交叉环若可用）→ 通过。
- [ ] **Step 4: Commit** `feat(win): app discovery via shell:AppsFolder with noise filter`

---

### Task 4: 启动/打开 win/launch.rs + 四命令分派

**Files:**
- Create: `src-tauri/src/win/launch.rs`
- Modify: `src-tauri/src/lib.rs`（`launch_app_blocking` 约 163、`open_url`/`open_path` 约 415-431、`image_preview` 约 965）

- [ ] **Step 1: launch.rs**

```rust
//! ShellExecuteW 包装：应用（含 shell:AppsFolder\AUMID）、URL、路径统一入口。
use windows::core::PCWSTR;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// ShellExecuteW 默认动词。返回值 ≤32 为错误码。
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

- [ ] **Step 2: lib.rs 四处分派**——`launch_app_blocking` 首行 `#[cfg(target_os = "windows")] return win::launch::shell_open(path);`，原体包 `#[cfg(not(target_os = "windows"))] { ... }`；`open_url`/`open_path` 同法；`image_preview` Windows 分支写完临时 PNG 后 `shell_open`（默认看图器，降级方案），mac 的 qlmanage 分支原样包 cfg。

- [ ] **Step 3:** `cargo check` → 通过。
- [ ] **Step 4: Commit** `feat(win): launch/open/preview via ShellExecuteW`

---

### Task 5: 图标 win/icons.rs + icon_data_url 分派

**Files:**
- Create: `src-tauri/src/win/icons.rs`
- Modify: `src-tauri/src/lib.rs`（`icon_data_url` 约 251-280）

- [ ] **Step 1: icons.rs**

```rust
//! IShellItemImageFactory：普通路径与 shell:AppsFolder\AUMID 统一出 64px 图标。
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
            hdc, hbmp, 0, N as u32,
            Some(buf.as_mut_ptr() as *mut _),
            &mut info, DIB_RGB_COLORS,
        );
        ReleaseDC(None, hdc);
        let _ = DeleteObject(hbmp.into());
        if lines == 0 {
            return None;
        }
        for px in buf.chunks_exact_mut(4) {
            px.swap(0, 2); // BGRA -> RGBA
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

- [ ] **Step 2: lib.rs**——`icon_data_url` 外层 `#[cfg(target_os = "macos")]` 改 `#[cfg(any(target_os = "macos", target_os = "windows"))]`（缓存逻辑共用），兜底块改 `#[cfg(not(any(...)))]`，并加：

```rust
#[cfg(target_os = "windows")]
fn icon_png(path: &str) -> Option<Vec<u8>> {
    win::icons::icon_png(path)
}
```

- [ ] **Step 3:** `cargo check` → 通过。
- [ ] **Step 4: Commit** `feat(win): app icons via IShellItemImageFactory, shared disk cache`

---

### Task 6: 通知（仅 Windows 接线）+ 默认热键分叉

**Files:**
- Modify: `src-tauri/src/lib.rs`（`notify` 约 691；Builder 起链 1083；`DEFAULT_HOTKEY` 1022）

- [ ] **Step 1: 注册插件（仅 Windows）**。`run()`（lib.rs:1082）起链处改为：

```rust
let builder = tauri::Builder::default();
#[cfg(target_os = "windows")]
let builder = builder.plugin(tauri_plugin_notification::init());
builder
    .manage(AutoHide(std::sync::atomic::AtomicBool::new(true)))
    .plugin(tauri_plugin_dialog::init())
    // ……链其余部分逐字不动
```

- [ ] **Step 2: notify 分派**（签名加 `app`，Tauri 自动注入，前端零改动）：

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

（Rust 侧调用插件不走 IPC 权限，capabilities 不动。）

- [ ] **Step 3: 热键分叉**（lib.rs:1022，Win+Shift+Space 在 Windows 撞输入法反向切换）：

```rust
#[cfg(target_os = "windows")]
const DEFAULT_HOTKEY: &str = "alt+space"; // uTools 惯例；Win+Shift+Space 与系统输入法切换冲突
#[cfg(not(target_os = "windows"))]
const DEFAULT_HOTKEY: &str = "super+shift+space";
```

- [ ] **Step 4:** `cargo check` → 通过。
- [ ] **Step 5: Commit** `feat(win): toast notifications + alt+space default hotkey`

---

### Task 7: 剪贴板 win/clipboard.rs

**Files:**
- Create: `src-tauri/src/win/clipboard.rs`
- Modify: `src-tauri/src/lib.rs`（非 mac 版 `clipboard_read_files` 约 338、`clipboard_image_present` 约 376）

- [ ] **Step 1: clipboard.rs**

```rust
//! CF_HDROP 文件列表读取 + 位图格式探测（只探格式不解码，对齐 macOS 版语义）。
use windows::Win32::System::DataExchange::{
    CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
};
use windows::Win32::System::Ole::{CF_BITMAP, CF_DIB, CF_HDROP};
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};

pub fn read_files() -> Vec<String> {
    let mut out = Vec::new();
    unsafe {
        if OpenClipboard(None).is_err() {
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

- [ ] **Step 3:** `cargo check` → 通过。
- [ ] **Step 4: Commit** `feat(win): clipboard file list (CF_HDROP) and image presence probe`

---

### Task 8: 单实例（手写，不引插件）

**Files:**
- Create: `src-tauri/src/win/single_instance.rs`
- Modify: `src-tauri/src/lib.rs`（`run()` 入口 + setup 闭包）

- [ ] **Step 1: single_instance.rs**

```rust
//! 单实例：命名互斥量判重；第二实例经命名事件唤醒主实例窗口后退出。
use windows::core::w;
use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE};
use windows::Win32::System::Threading::{
    CreateEventW, CreateMutexW, OpenEventW, SetEvent, WaitForSingleObject,
    EVENT_MODIFY_STATE, INFINITE,
};

/// 尝试成为唯一实例。已有实例 → 发唤醒信号并返回 false（调用方应直接 return）。
pub fn acquire_or_wake() -> bool {
    unsafe {
        // 互斥量句柄故意不关——进程存活期间持有即锁。
        let _mutex = CreateMutexW(None, true, w!("shu-single-instance-mutex"));
        if GetLastError() == ERROR_ALREADY_EXISTS {
            if let Ok(ev) = OpenEventW(EVENT_MODIFY_STATE, false, w!("shu-single-instance-wake")) {
                let _ = SetEvent(ev);
                let _ = CloseHandle(ev);
            }
            return false;
        }
        true
    }
}

/// 主实例：后台线程等唤醒事件，收到即回调（回调内做 show+focus，Tauri 窗口方法线程安全）。
pub fn spawn_wake_listener(on_wake: impl Fn() + Send + 'static) {
    unsafe {
        let Ok(event) = CreateEventW(None, false, false, w!("shu-single-instance-wake")) else {
            return;
        };
        let raw = event.0 as isize;
        std::thread::spawn(move || {
            let h = HANDLE(raw as _);
            loop {
                WaitForSingleObject(h, INFINITE);
                on_wake();
            }
        });
    }
}
```

- [ ] **Step 2: lib.rs 接线**。`run()` 函数体首行：

```rust
#[cfg(target_os = "windows")]
if !win::single_instance::acquire_or_wake() {
    return;
}
```

setup 闭包内（热键注册后）：

```rust
#[cfg(target_os = "windows")]
{
    let handle = app.handle().clone();
    win::single_instance::spawn_wake_listener(move || {
        if let Some(w) = handle.get_webview_window("main") {
            let _ = w.show();
            let _ = w.set_focus();
        }
    });
}
```

- [ ] **Step 3:** `cargo check` → 通过。
- [ ] **Step 4: Commit** `feat(win): hand-rolled single instance (named mutex + wake event)`

---

### Task 9: CI 加 cargo test、版本 bump、出验收 exe

**Files:**
- Modify: `.github/workflows/release.yml`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`、`package.json`（version → 0.2.0）

- [ ] **Step 1:** workflow `pnpm install` 步后加（全平台跑，logic 测试秒级）：

```yaml
      - run: cargo test --manifest-path src-tauri/Cargo.toml
```

- [ ] **Step 2:** 三处版本 bump 0.2.0；`pnpm test && cargo test --manifest-path src-tauri/Cargo.toml` 全绿。
- [ ] **Step 3:** commit + push + `git tag v0.2.0 && git push origin v0.2.0`。
- [ ] **Step 4:** 盯 CI 三平台绿（失败修复重发时：先删草稿 Release 再删/重打 tag，避免资产名冲突）。
- [ ] **Step 5:** 交用户真机验收，清单：

1. exe 安装（SmartScreen「仍要运行」预期内）
2. 托盘图标出现；**Alt+Space** 唤出/隐藏窗口
3. 应用列表非空：含 UWP（「设置」「计算器」）与 Win32（Chrome/微信等）
4. 列表里**没有**「卸载 XXX」类噪音条目
5. 中文应用显示名正确；启动 Win32 与 UWP 应用各一个成功
6. 反复唤出窗口，应用列表出现速度可接受（无缓存设计的验证点）
7. 应用图标正常渲染（首次略慢，二次即缓存）
8. **再次双击 exe：不出现第二个实例/托盘图标，已有窗口被唤出**
9. translator 插件可用；image-compressor 压缩可用、预览用默认看图器打开
10. 复制截图 → 唤出 shu → 图片推荐出现；复制图片文件 → 同上（CF_HDROP）
11. 任一插件触发通知 → Windows toast 弹出

---

## 阶段 2 / 3

见 spec（`docs/superpowers/specs/2026-07-16-windows-port-design.md`）：阶段 2（Everything 捆绑 + 绿色软件发现 + `merge_supplement` 补充源合并）待阶段 1 验收通过后单独出计划；阶段 3（拼音 / hosts / CF_HDROP 写入 / 开机自启 / 预览评估）随后。
