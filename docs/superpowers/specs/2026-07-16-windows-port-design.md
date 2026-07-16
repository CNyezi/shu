# shu Windows 版设计

日期：2026-07-16。目标：Windows exe 从"能编译的空壳"变成可用的启动器。

## 背景与验收方式

- CI 已跑通：tag push → Windows NSIS exe + macOS dmg 进 Release 草稿。WiX(msi) 因非 ASCII 产品名「枢」不可用，Windows 只出 NSIS。
- 开发在 macOS 上进行，CI Windows job 做编译门禁；**每阶段产出 exe，用户在自己的 Windows 机器上真机验收**。
- macOS 现有代码原地不动，只在非 macOS 路径上补实现。

## 已定决策

| 决策 | 选择 |
|------|------|
| Everything 集成 | 捆绑到宿主层（非插件层）；`Everything64.dll`（90KB，MIT）FFI，libloading 动态加载；先检测用户已装/运行中实例，有则复用 |
| Win32 绑定 | 复用 Tauri 传递依赖的官方 `windows` crate；注册表用 `winreg` |
| 通知 | `tauri-plugin-notification`，仅 Windows 接线；macOS 保留 osascript |
| 拼音 | `pinyin` crate（纯数据表），非 macOS 编译 |
| 代码组织 | 新增 `src-tauri/src/win/` 模块（discovery / launch / icons / everything / elevate），`lib.rs` 命令按 `cfg` 分派 |
| hosts 提权 | PowerShell `Start-Process -Verb RunAs`（UAC） |

## 阶段 1：基础可用

装上就是能用的启动器。

**应用发现**：`shell:AppsFolder` 枚举单源（一个 API 同时覆盖 Win32 已装应用 + UWP，显示名本地化正确，PowerToys Run 同款方案，无需手工解析 .lnk）+ 名字噪音过滤（「卸载/uninstall/帮助/官网」类快捷方式剔除，纯函数配单测）。`AppEntry.path` 对 UWP 存 `shell:AppsFolder\<AUMID>`，前端零改动。注册表 App Paths 补充源**不做**——键名派生的显示名（"chrome"）与 AppsFolder 显示名（"Google Chrome"）无法按名去重，增量价值又小；长尾交给阶段 2 的 Everything 层。（grilling 决策 2026-07-16）

**启动/打开**：`launch_app` / `open_url` / `open_path` 统一走 `ShellExecuteW`。

**图标**：`IShellItemImageFactory` 出 64px 位图 → PNG 编码（`image` crate 已有 png feature），沿用现有磁盘缓存逻辑。

**通知**：toast，经 tauri-plugin-notification。

**剪贴板图片**：读/写图片切到 `arboard`（已是依赖，Windows 支持完整）。`clipboard_write_files` 暂保持"not supported"（阶段 3 补 CF_HDROP）。

**图片预览**：`image_preview` 降级为写临时 PNG 后 ShellExecute 用默认看图器打开。

**插件**：translator 零改动；image-compressor 除预览降级外零改动；hosts-editor 阶段 3。

**单实例**：手写 Rust（命名互斥量判重 + 命名事件唤醒已有实例窗口），不引插件——这是软件的标准功能，做进宿主。

**默认热键**：Windows 上 `super+shift+space` 与系统输入法反向切换冲突，`DEFAULT_HOTKEY` 平台分叉为 **Alt+Space**（uTools 惯例）；macOS 不变。

**性能**：应用列表不做缓存，真机验收含"唤出速度可接受"检查项，慢再优化。

## 阶段 2：Everything 红利

**捆绑**：`Everything.exe`（2.3MB，不带语言文件）+ `Everything64.dll` 作为 Windows 专属资源（`tauri.windows.conf.json` 平台覆盖配置）。发布前邮件 voidtools 确认 exe 再分发（SDK 本身 MIT）。

**生命周期**：首次需要时 SDK ping 检测运行中实例 → 有则直接用；无则拉起捆绑的 `Everything.exe -startup`（后台无窗口）。MFT 直读需要 Everything 服务：检测到索引不可用时，提示用户一次，UAC 确认后 `-install-service`。用户拒绝 → 功能降级（基础发现仍完整可用），不重复骚扰。

**绿色软件自动发现**：Everything 查 `ext:exe` 补充应用列表——过滤噪音（windir、Program Files 下的 unins*/setup*/helper 等模式），排序权重低于 shell:AppsFolder 结果。过滤规则做成纯函数,配 Rust 单元测试。

**预留**：宿主命令 `everything_query(query, max_results)`，按现有 capability 机制门控，供未来「本地搜索」插件使用。

## 阶段 3：打磨

- 拼音 + 首字母搜索（替换现在返回 `None` 的兜底，行为对齐 macOS 版）。
- hosts-editor：读 `C:\Windows\System32\drivers\etc\hosts`；写走 UAC（临时文件 + 提权 copy），取消 → 返回「已取消」与 macOS 行为一致。
- `clipboard_write_files`：CF_HDROP。
- 开机自启：`HKCU\...\Run` 注册表项 + 设置界面开关（grilling 决策：启动器刚需，但不值得为它打破阶段 1 的"前端零改动"）。
- 图片预览体验评估：默认看图器若够用则不再做浮窗。

## 错误处理

- 所有 win 命令沿用现有 `Result<_, String>` 约定,错误消息中文、面向用户。
- Everything 全链路可降级：DLL 加载失败 / 实例拉不起 / 服务未装,均不影响阶段 1 功能。
- UAC 拒绝一律视为用户取消,不作为错误上报。

## 测试

- 纯逻辑（exe 噪音过滤、排序权重、App Paths 解析）：`cargo test`（src-tauri 首个 `#[cfg(test)]`）,CI Windows job 顺带跑。
- 系统调用边界（COM 枚举、ShellExecute、FFI）：不做 mock 测试,靠每阶段真机验收清单。
- 各阶段验收清单写在实施计划中。
