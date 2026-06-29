# 打包与分享

## 插件目录结构

一个插件就是一个文件夹，包含：

- `plugin.json`（必须在根目录）
- feature 的入口文件（HTML 或 JS，路径在 plugin.json 中声明）
- 图标文件（可选）

```
my-plugin/
├── plugin.json
├── index.html      # UI 插件入口
└── icon.svg        # 图标（可选）
```

## 打包为 .pcp 文件

将插件目录打包成单个 `.pcp` 文件（实质是 zip 压缩包，`plugin.json` 必须位于 zip 根）：

```bash
cd my-plugin && zip -r ../my-plugin.pcp .
```

## 插件仓库模板

新插件建议从仓库模板开始：

```txt
templates/plugin-template/
```

模板包含：

- `plugin.json`
- `index.html`
- `.github/workflows/release.yml`

作者只需要改 `plugin.json` 和插件代码，然后推送 tag：

```bash
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions 会自动生成 `.pcp` 并发布到 Release。

## 安装方式

在枢启动器中输入 `插件` 或 `plugins` 进入插件管理，支持以下安装方式：

- 将 `.pcp` 文件拖入窗口
- 点击「从文件安装」选择 `.pcp` 文件
- 粘贴 URL（支持通过网络地址安装）

## 授权确认

安装时会弹出授权框，显示以下信息供用户确认：

- 插件名称、id、版本号
- 申请的能力列表（以中文展示）
- 包的 SHA-256 校验值

用户同意后，所申请的 `permissions` 中的能力会被授予。

## 升级与降级

- **升级**：安装更高版本会覆盖已安装的版本。若新版本申请了新权限，需要用户重新同意。
- **降级**：禁止安装低于当前已安装版本的插件。

## 安装位置

- 已安装插件存放在 `~/.config/shu/plugins/<id>/`
- 授权记录保存在 `~/.config/shu/registry.json`

## 静态注册中心 registry.json

插件可以通过静态 `registry.json` 被枢发现。文件可以放在 GitHub Pages 或任意 HTTP 静态服务器上：

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

用户添加的注册中心 URL 保存在 `~/.config/shu/registries.json`。安装时仍以 `.pcp` 内的 `plugin.json` 为准，并会校验 `sha256`。

应用默认内置官方 registry，也可以通过环境变量覆盖：

```bash
VITE_SHU_OFFICIAL_REGISTRY_URL=https://raw.githubusercontent.com/CNyezi/shu-registry/main/registry.json
```

该 URL 会在插件管理器里自动展示为「官方」，用户手动添加的 registry 仍保存在本机配置里。

## 官方 registry 提交

官方 registry 仓库建议从模板开始：

```txt
templates/registry-template/
```

当前官方仓库：

- https://github.com/CNyezi/shu-registry
- https://github.com/CNyezi/shu-plugin-template

插件作者提交时只需要在 `submissions/` 新增一个 JSON：

```json
{
  "repo": "https://github.com/you/your-shu-plugin"
}
```

registry 的 GitHub Action 会读取插件仓库的 latest release，找到 `.pcp`，生成 `registry.json` 条目。维护者只 review 生成结果。
