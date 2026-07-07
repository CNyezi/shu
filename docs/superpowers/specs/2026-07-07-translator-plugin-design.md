# 翻译插件（com.shu.translator）

## 背景与选型

用户确认方案 A：启动器内用**微软 Edge 翻译端点**直接出结果（免费、无 key、大陆直连、自动检测语言、~200ms），结果区提供**「在网页中打开」**跳转（Bing / DeepL / Google，免登录页面，走 `shell.openUrl`）作为深度路径。调研对比与排除理由见对话记录（DeepLX 公共实例停服、Google gtx 需代理、LibreTranslate 质量不足等）。

## 交互

- 进入：关键词 `fy` / `translate` / `翻译`。**不加 content 触发器**——任意文本都会命中会造成剪贴板劫持。
- 打开时读一次剪贴板：非空文本且 <5000 字符 → 自动带入并翻译。
- 输入：面板内 textarea（可编辑长文本），同时接收主搜索框转发的 `host.onInput`（350ms 防抖翻译）。
- 翻译方向：客户端启发式——含 CJK 字符 → 目标 `en`，否则 → 目标 `zh-Hans`；源语言由 API 自动检测（`from` 省略），结果区显示 `检测语言 → 目标`。
- 动作：复制结果（`clipboard.write`，内联"已复制"反馈）；网页打开 Bing / DeepL / Google（预填文本与方向）。
- 失败显示错误状态，不静默。

## Edge 端点协议

1. `GET https://edge.microsoft.com/translate/auth` → 纯文本 JWT，内存缓存 8 分钟，401 时强刷一次重试。
2. `POST https://api.cognitive.microsofttranslator.com/translate?api-version=3.0&to=<目标>`，headers `Authorization: Bearer <jwt>` + `Content-Type: application/json`，body `[{"Text":"…"}]`。响应 `[0].detectedLanguage.language` + `[0].translations[0].text`。
3. 并发保护：递增序号丢弃过期响应。

## 文件与权限

- `plugins/translator/plugin.json`：permissions `["clipboard.read","clipboard.write","shell.openUrl","network"]`（network 为高危档，安装确认符合预期）。
- `plugins/translator/index.html`：UI + 逻辑，暗色风格与 json-preview 约定一致。
- `plugins/translator/icon.svg`：圆角方块 + "译"。

## 不做（YAGNI）

多引擎切换 / API key 配置（v2 视需求）、目标语言下拉（中英互判之外的语向）、历史记录、流式输出。

## 验收

dev 模式重启应用 → `fy` 进入 → 剪贴板预填、输入即译、方向自判、复制、三个网页跳转可用。发布到注册中心待用户验收后进行。
