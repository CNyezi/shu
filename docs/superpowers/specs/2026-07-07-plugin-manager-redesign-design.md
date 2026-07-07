# 插件管理面板重设计

## 背景与目标

现状是单列长滚动：安装入口、源管理、市场列表、已装列表混排，无搜索。用户确认：装与管两个高频区要分明（C），注册中心规模按几十个设计（A），保持简单。

## 设计

### 布局

```
│ ←  插件管理                                 │  顶栏不变（Esc 返回）
│ [已安装 N ●M]  [插件市场]     🔍 搜索插件…  │  tab + 搜索框（过滤当前 tab）
│  (当前 tab 列表，480px 内自滚动)            │
```

### 已安装 tab

每行：插件图标（复用 App 的 iconMap，缺省显示首字符占位）+ 名称 + 版本 + 权限摘要一行灰字。
有新版本（注册中心同 id 版本号 ≠ 已装版本）时：名称旁 `↑ vX.Y.Z` 徽标 + 「更新」主按钮（走 onInstallRegistryPlugin）。
操作：卸载（保留确认弹窗，危险色弱化样式）、自动打开开关（仅内容触发型插件）。
tab 标签显示可更新数量角标（●M，M=0 时不显示）。

### 插件市场 tab

顶部一行：刷新按钮（含加载中态）。
条目：名称/版本/描述/权限摘要/三态按钮（安装=主色、更新到 vX=主色、已安装=禁用）。
底部 `<details>` 折叠区「安装来源与高级」：从文件安装、粘贴 .pcp 链接、registry 源增删列表（官方 tag 不可删）。

### 搜索

一个输入框过滤当前 tab，用现有 `matchScore`（白赚拼音首字母），匹配 名称/描述/id。空结果显示「无匹配插件」。

### 视觉（frontend-design 标准）

继承现有暗色 token（--bg/--bar/--sel/--muted）。tab 激活态 = 白字 + 主色下划线；行 hover 微亮背景 + 圆角；按钮分层：安装/更新=主色实心、卸载=危险色描边、已安装=禁用态；徽标=主色小 pill；权限摘要=小号灰字。间距保持 8px 节奏。

### 不做（YAGNI）

分类、分页、列表虚拟化、评分下载量、键盘 tab 切换、update-all。

## 实现范围

- `src/lib/PluginManager.svelte`：重写布局与样式；新增 props `iconMap`；内部状态 tab/search；派生 updates 映射。
- `src/App.svelte`：传 `{iconMap}`；无其他逻辑改动。
- 现有回调（onInstallFile/onInstallUrl/onUninstall/onAddRegistry/onRemoveRegistry/onRefreshRegistries/onInstallRegistryPlugin/onToggleAutoOpen）签名全部不变。
- 验证：pnpm check / pnpm test / pnpm build；运行中的 dev 应用 HMR 实测验收。
