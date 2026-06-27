# pc-tool 插件分享 ② —— Registry v1 静态注册中心

- 日期：2026-06-27
- 状态：待 review
- 范围：插件分享第二个子系统。只做静态注册中心协议和客户端安装入口，不做市场 UI。

## 1. 背景与目标

插件安装、授权、卸载已经跑通，但用户还缺一个“从哪里发现插件”的入口。Registry v1 的目标是用最小协议补上发现链路：作者把 `.pcp` 放到任意可访问 URL，再把元数据写进一个静态 `registry.json`。客户端拉取这个 JSON，展示插件列表，并复用现有 URL 安装、inspect、授权、install 流程。

成功标准：用户添加一个 registry URL 后，可以看到插件列表，选择一个插件安装；安装前仍显示权限授权和包 SHA-256，安装后插件进入现有插件系统。

## 2. 范围

### 内

- 静态 `registry.json` 协议。
- 客户端保存 registry URL 列表。
- 拉取 registry 并展示插件条目。
- 从 registry 条目安装插件，复用现有 `download_package -> inspect_package -> install_package`。
- 最小错误提示：URL 无效、JSON 无效、下载失败、sha256 不匹配。

### 外

- 插件市场 UI（评分、分类、评论、截图、排行榜）。
- 账号、发布后台、审核流。
- 自动更新。
- 签名和发布者身份。
- 多 registry 合并排序策略。

## 3. Registry 协议

Registry 是一个静态 JSON 文件：

```json
{
  "version": 1,
  "plugins": [
    {
      "id": "com.you.hello",
      "name": "Hello",
      "version": "1.0.0",
      "description": "读取剪贴板并显示内容",
      "permissions": ["clipboard.read"],
      "packageUrl": "https://example.com/hello.pcp",
      "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    }
  ]
}
```

字段规则：

- `version`：协议版本，v1 固定为 `1`。
- `plugins`：插件条目数组。
- `id` / `name` / `version` / `permissions`：必须与 `.pcp` 内 `plugin.json` 一致；安装时以包内 manifest 为准，不信任 registry。
- `description`：列表展示用，纯文本。
- `packageUrl`：仅允许 `http` / `https`。
- `sha256`：下载后校验；不匹配则不进入授权框。

## 4. 客户端设计

### Rust

新增轻量命令：

- `list_registries() -> string[]`
- `add_registry(url: string) -> ()`
- `remove_registry(url: string) -> ()`
- `fetch_registry(url: string) -> RegistryFeed`
- `download_package_checked(url: string, sha256: string) -> path`

Registry URL 存在 `~/.config/pc-tool/registries.json`：

```json
{ "urls": ["https://example.com/pc-tool/registry.json"] }
```

### 前端

在现有 `PluginManager.svelte` 里加一个很小的 “注册中心” 区块：

- 输入 registry URL，点击添加。
- 列出已添加 URL，可删除。
- 点击刷新，展示该 registry 的插件条目。
- 插件条目只有名称、版本、描述、权限和安装按钮。
- 安装按钮走现有 consent 流程：下载校验包 -> inspect -> 授权 -> install。

## 5. 错误处理

- registry URL 非 http/https：拒绝。
- 拉取失败：toast 显示错误。
- JSON 结构不合法：toast 显示错误。
- `sha256` 不匹配：拒绝安装。
- registry 元数据与包内 manifest 不一致：授权框以包内 manifest 为准；列表仍展示 registry 元数据。

## 6. 测试

- Rust：registry URL 读写往返；非法 URL 拒绝；sha256 不匹配拒绝。
- Node/前端纯函数：registry JSON 校验和列表过滤。
- 手动：用本地静态 registry 指向 `/tmp/pc-tool-json-preview.pcp` 或本地 HTTP 服务，完成刷新和安装。

## 7. 验收清单

- [ ] 可添加一个 registry URL。
- [ ] 可刷新并看到 registry 里的插件。
- [ ] 可从 registry 插件条目进入现有授权安装流程。
- [ ] sha256 不匹配时安装被拒绝。
- [ ] 删除 registry 后列表不再显示该源插件。

## 8. 后续

Registry v1 跑通后，再做市场 UI。市场第一版只是 registry 的浏览器，不改变安装信任链路。
