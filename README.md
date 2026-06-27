# 枢

插件化效率启动器（MVP）。Tauri v2（Rust 核心 + 系统 WebView）+ 沙箱化 JS 插件。

设计文档见 `docs/superpowers/specs/2026-06-22-launcher-mvp-design.md`。

## 运行

```bash
pnpm install
pnpm tauri dev      # 开发模式，启动应用窗口
```

- 以**菜单栏托盘应用**运行（macOS 不占 Dock）。点击托盘图标或全局热键唤起；托盘菜单含「显示/隐藏」「退出」。
- **唤起 / 隐藏**：全局热键 `Cmd+Shift+Space`（失焦或 `Esc` 自动隐藏）。
- **启动应用**：输入应用名（如 `saf`）→ `↑/↓` 选择 → `Enter` 启动（结果带应用图标）。
- **内容感知**：唤起时读一次剪贴板并识别类型。若内容是 JSON：只有一个适配插件时**直接打开**，多个时在列表顶部带图标推荐供选择。
- **JSON 编辑器插件**：识别到 JSON 自动进入，或输入关键词 `json` 进入。左侧可编辑文本（实时校验），右侧可折叠树形可视化，可一键复制格式化结果。`Esc` / 左上角 `←` 返回。

## 测试

```bash
pnpm test          # 轻量 Node 自检
pnpm check
pnpm build
cd src-tauri && cargo test
```

UI 插件安装流可用测试窗口跑，避开托盘和全局热键：

```bash
cd plugins/json-preview && zip -qr /tmp/shu-json-preview.pcp .
cd ../..
pnpm tauri:test
```

`pnpm tauri:test` 仅在开发模式打开 `/test`，可直接点 Inspect / Install / Uninstall。

## 结构

- `src-tauri/` — Rust 核心：全局热键、窗口管理、应用枚举与启动、系统能力命令、插件加载。
- `src/` — 宿主壳（Svelte）：搜索框、结果列表、插件 iframe 容器、**能力桥中介 + 白名单校验**。
- `src/lib/pluginRuntime.ts` — 沙箱运行时：插件跑在 `sandbox="allow-scripts"` 的 iframe 里，只能通过 `postMessage` 向宿主请求能力，宿主按 `plugin.json` 的 `permissions` 白名单放行。
- `plugins/<id>/` — 插件（`plugin.json` + 资源）。开发模式从仓库 `plugins/` 加载；发布版从 `~/.config/shu/plugins/` 加载。

## 插件能力（MVP）

声明在 `plugin.json` 的 `permissions` 中，未声明的调用会被宿主拒绝：

`clipboard.read` · `clipboard.write` · `shell.openUrl` · `shell.openPath`
