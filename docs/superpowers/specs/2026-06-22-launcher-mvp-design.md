# pc-tool —— 插件化效率启动器 MVP 设计

- 日期：2026-06-22
- 状态：待 review
- 范围：MVP（对应讨论中的"方案 B + UI 插件扩展"）

## 1. 背景与目标

做一个类 uTools / Raycast 的桌面效率启动器，但走 **Tauri v2（Rust 核心 + 系统 WebView）** 路线，主打**极致轻量**与**沙箱化的 JS 插件体系**。

竞品现状：Raycast 2.0、Asyar（Tauri+Svelte）、Tuff、Gauntlet 等已在做"系统 webview + Rust 核心"架构；uTools 仍是 Electron。本产品的长期差异点在**去中心化生态**（自建同步服务器 + 多机互通 + 插件分享平台）与**本地 AI**，但这些是后续子系统，不在本 MVP。

**本 MVP 的唯一目标**：验证核心技术赌注 —— *一个 JS 插件能在沙箱里运行，拿到输入上下文，并通过受控的能力桥调用系统能力*。Launcher 本身只是承载这个验证的最小外壳。

## 2. 范围

### MVP 内
- 全局热键唤起的无边框搜索窗口
- 内置核心功能：搜索并启动本地应用（仅 macOS）
- 两种形态的插件运行时（逻辑插件 + UI 插件），均跑在 webview 沙箱内
- 宿主中介的能力桥 + manifest 能力白名单
- 第一个 bundled 插件：剪贴板检查器（识别 文本 / JSON / 图片）
- 插件以本地文件夹形式安装

### MVP 外（后续独立子系统，各自单独 spec）
- 本地 AI（语义搜索 / 问答）
- 自建同步服务器、多机数据互通
- 插件分享 / 分发市场
- 第二运行时（`rquickjs` Rust 层强隔离，用于高危不可信纯逻辑插件）
- 跨平台（Windows / Linux 的应用枚举与热键）
- 超级面板、文件类型匹配、自定义快捷键、子进程执行、任意文件系统读写、HTTP 代理、插件持久化存储

## 3. 架构

Tauri v2 应用，单个**无边框窗口**（方案 A：单窗口切换视图，非多窗口）。

```
┌─ 无边框窗口 (Tauri WebviewWindow) ─────────────┐
│  宿主壳 Host Shell (Svelte)                      │
│  ┌───────────────────────────────────────────┐ │
│  │ [← 关闭]  搜索框________________________   │ │ ← 宿主拥有，常驻
│  ├───────────────────────────────────────────┤ │
│  │   结果列表  /  或  UI 插件 <iframe>        │ │ ← 内容区，切换
│  └───────────────────────────────────────────┘ │
└─────────────────────────────────────────────────┘
         │ Tauri IPC invoke（唯一系统能力入口）
         ▼
   Rust 核心：全局热键 / 窗口管理 / 插件加载 /
            能力 API 实现 / ACL 白名单强制
```

### 3.1 宿主中介模式（安全咽喉点）

这是整个沙箱设计的核心：

- **插件 JS 永远拿不到 Tauri 的 `invoke`**。所有插件（逻辑 + UI）都运行在 `sandbox` 属性的 iframe / Web Worker 中，**只能 `postMessage` 给宿主壳**。
- **宿主壳是唯一能调 Tauri `invoke` 的角色**。收到插件的能力请求后：
  1. 校验该能力是否在插件 `plugin.json` 声明的 `permissions` 白名单内；
  2. 在白名单内 → 转发给 Rust 核心执行；不在 → 拒绝（并可触发授权提示，MVP 阶段直接拒绝即可）。
- 结果：所有"插件 → 系统"调用必经宿主壳这一处，能力边界单点强制、可审计。比 uTools/Electron 把系统能力直接暴露给插件更强。

### 3.2 组件边界

| 组件 | 职责 | 依赖 |
|---|---|---|
| Rust 核心 | 全局热键、窗口显隐、插件目录扫描与 manifest 解析、能力 API 实现、应用枚举与启动 | Tauri v2、OS API |
| 宿主壳 (Svelte) | 搜索框、结果列表、插件 iframe 容器、关闭/返回栏、能力请求中介与白名单校验、输入路由 | Tauri IPC |
| 插件运行时 | 在 sandboxed iframe/Worker 内加载并运行插件代码，暴露 `host.*` 协议 API，桥接 postMessage | 仅 postMessage |
| 插件 | 第三方/内置代码，声明 features、triggers、permissions | `host.*` API |

## 4. 插件模型

### 4.1 形态
- **逻辑插件**（`type: "logic"`）：无界面，跑在**隐藏的 sandboxed iframe**（与 UI 插件同一运行时，避免两套 JS 环境），接收实时输入，回填结果到列表。
- **UI 插件**（`type: "ui"`）：内容区挂一个 `sandbox` 属性的 `<iframe>` 加载插件页面；关闭按钮在宿主栏上，宿主卸载 iframe 即关闭。

### 4.2 清单格式 `plugin.json`

```jsonc
{
  "id": "com.you.clipboard",
  "name": "剪贴板检查器",
  "version": "0.1.0",
  "icon": "icon.png",
  "features": [
    {
      "code": "cb",
      "type": "logic",            // "ui" | "logic"
      "entry": "main.js",         // logic→js；ui→html
      "triggers": [
        { "kind": "keyword", "value": "cb" }
      ]
    },
    {
      "code": "cb-image",
      "type": "ui",
      "entry": "image.html",
      "triggers": []              // 无触发词，仅由 host.redirect 进入
    }
  ],
  "permissions": ["clipboard.read", "clipboard.write"]
}
```

### 4.3 触发模型
- **keyword**：输入匹配关键词 → 进入该 feature（UI 挂 iframe；逻辑开始接收输入）。
- **regex**：输入命中正则 → 该 feature 作为一条结果出现在列表中。

两种均在 MVP 内。更复杂的匹配（文件类型、选中文本超级面板）不做。

### 4.4 加载
- 插件 = 一个文件夹，含 `plugin.json` + 资源。
- 安装方式：放入 `~/.config/pc-tool/plugins/<id>/`。
- 启动时 Rust 核心扫描该目录、解析所有 `plugin.json`、构建触发索引。MVP 不做热重载、不做远程安装。

## 5. API

### 5.1 系统能力 API（需 `permissions` 声明，经咽喉点放行）

| 能力 | 权限名 | 说明 |
|---|---|---|
| 读剪贴板 | `clipboard.read` | 返回 `{ type: 'text' \| 'image', text?, image? }`；图片以 data URL / 临时文件路径形式返回 |
| 写剪贴板 | `clipboard.write` | 写入文本 |
| 打开 URL | `shell.openUrl` | 默认浏览器打开 |
| 打开文件/路径 | `shell.openPath` | 默认程序打开 |
| 发通知 | `notification` | 系统通知 |

### 5.2 宿主协议 API（无需权限，插件运行基础）

| API | 说明 |
|---|---|
| `host.onInput(cb)` | 逻辑插件接收搜索框实时输入 |
| `host.setResults([...])` | 逻辑插件回填结果列表 |
| `host.getContext()` | 获取当前上下文（当前输入、选中文本等） |
| `host.close()` | 关闭当前插件，返回搜索 |
| `host.redirect(code)` | 跳转到另一个 feature |

## 6. 内置核心功能：应用启动（macOS）

不走插件（应用枚举是重度、平台相关的能力，应锁在 Rust 核心，不暴露给沙箱）。

- 启动时 Rust 扫描 `/Applications`、`/System/Applications`、`~/Applications`，解析 `.app`（名称、图标、路径）。
- 搜索框输入 → 模糊匹配应用名 → 结果列表；回车用 `open` 启动选中应用。
- 全局热键唤起窗口、`Esc` 隐藏、失焦隐藏。

## 7. 第一个 bundled 插件：剪贴板检查器

验证整条插件链路。

插件声明**两个 feature**：`cb`（logic，入口）+ `cb-image`（ui，图片预览）。

- 触发：keyword `cb` → 进入 logic feature。
- logic 行为：调 `clipboard.read` 拿内容，在沙箱 JS 内判类型：
  - **JSON**：`JSON.parse` 成功 → 结果项"格式化的 JSON"，动作：复制格式化结果（`clipboard.write`）。
  - **图片**：`host.redirect('cb-image')` 跳到 ui feature，挂 iframe 预览图片。
  - **纯文本**：展示文本摘要。
- 声明 `clipboard.read` + `clipboard.write` 权限，验证白名单机制。

## 8. 平台

MVP 仅 **macOS**（开发机为 darwin）。应用枚举、全局热键、剪贴板图片读取均先实现 macOS 路径。跨平台留作后续。

## 9. 安全模型小结

1. 插件运行在 `sandbox` iframe / Worker，无 `invoke`、无 Node、无 `require`。
2. 唯一出口是 postMessage → 宿主壳。
3. 宿主壳按 `plugin.json.permissions` 白名单逐次校验后才转发给 Rust。
4. Tauri v2 的 capabilities/ACL 进一步限制宿主壳自身可调的命令集合。
5. CSP 限制插件 webview 的外联。

## 10. 验收清单（MVP Done 的定义）

- [ ] 全局热键唤起无边框窗口，Esc / 失焦隐藏。
- [ ] 搜索框输入应用名，列表显示匹配应用，回车成功启动。
- [ ] 宿主扫描 `~/.config/pc-tool/plugins/` 并加载剪贴板插件。
- [ ] 输入 `cb` 进入剪贴板插件，能读到剪贴板并正确识别 文本 / JSON / 图片三种情况。
- [ ] 图片走 UI 插件 iframe 预览；JSON 可一键复制格式化结果。
- [ ] 插件声明 `clipboard.read` 才能读；删掉权限声明后调用被宿主拒绝（白名单生效验证）。
- [ ] 插件代码内 `window.__TAURI__` / `invoke` 不可达（沙箱隔离验证）。

## 11. 技术选型

| 层 | 选型 | 理由 |
|---|---|---|
| 框架 | Tauri v2 | Rust 核心 + 系统 webview，轻量 |
| 宿主壳 UI | Svelte | 编译期产物小、无虚拟 DOM，契合轻量目标 |
| 插件运行时 | sandboxed iframe + Web Worker + postMessage | 全 webview 单运行时，桥只写一遍，作者门槛低 |
| 能力 ACL | Tauri v2 capabilities + 宿主白名单 | 双层限制 |

## 12. 后续子系统（非本 spec，各自独立迭代）

1. 本地 AI 插件（语义搜索 / 问答）。
2. 自建同步服务器 + 多机数据互通。
3. 插件分享 / 分发平台。
4. 第二运行时 `rquickjs`（高危不可信纯逻辑插件的 Rust 层强隔离）。
5. 跨平台（Windows / Linux）。
6. 超级面板、文件类型匹配、自定义快捷键等增强触发。
