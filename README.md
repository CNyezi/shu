# pc-tool

插件化效率启动器（MVP）。Tauri v2（Rust 核心 + 系统 WebView）+ 沙箱化 JS 插件。

设计文档见 `docs/superpowers/specs/2026-06-22-launcher-mvp-design.md`。

## 运行

```bash
pnpm install
pnpm tauri dev      # 开发模式，启动应用窗口
```

- **唤起 / 隐藏**：全局热键 `Cmd+Shift+Space`（失焦或 `Esc` 自动隐藏）。
- **启动应用**：输入应用名（如 `saf`）→ `↑/↓` 选择 → `Enter` 启动。
- **JSON 预览插件**：输入关键词 `json` 进入。读取剪贴板，若为合法 JSON 则格式化预览，可一键复制格式化结果。`Esc` / 左上角 `←` 返回。

## 结构

- `src-tauri/` — Rust 核心：全局热键、窗口管理、应用枚举与启动、系统能力命令、插件加载。
- `src/` — 宿主壳（Svelte）：搜索框、结果列表、插件 iframe 容器、**能力桥中介 + 白名单校验**。
- `src/lib/pluginRuntime.ts` — 沙箱运行时：插件跑在 `sandbox="allow-scripts"` 的 iframe 里，只能通过 `postMessage` 向宿主请求能力，宿主按 `plugin.json` 的 `permissions` 白名单放行。
- `plugins/<id>/` — 插件（`plugin.json` + 资源）。开发模式从仓库 `plugins/` 加载；发布版从 `~/.config/pc-tool/plugins/` 加载。

## 插件能力（MVP）

声明在 `plugin.json` 的 `permissions` 中，未声明的调用会被宿主拒绝：

`clipboard.read` · `clipboard.write` · `shell.openUrl` · `shell.openPath`
