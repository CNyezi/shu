# 能力清单

插件需要在 `plugin.json` 的 `permissions` 字段声明所需能力，并在用户安装时获得授权，才能调用对应的 `host.*` API。

## 当前支持的能力

| 能力 id | 说明 | 对应 API | 风险 |
|---|---|---|---|
| `clipboard.read` | 读取剪贴板文本 | `host.clipboard.read()` | |
| `clipboard.write` | 写入剪贴板文本 | `host.clipboard.write(text)` | |
| `clipboard.readImage` | 读取剪贴板图片 | `host.clipboard.readImage()` | |
| `clipboard.writeImage` | 写入剪贴板图片 | `host.clipboard.writeImage(dataUrl)` | |
| `clipboard.readFiles` | 读取剪贴板中的文件路径 | `host.clipboard.readFiles()` | |
| `clipboard.writeFiles` | 复制文件路径到剪贴板 | `host.clipboard.writeFiles(paths)` | |
| `shell.openUrl` | 用默认浏览器打开网址 | `host.openUrl(url)` | |
| `shell.openPath` | 用默认程序打开文件/文件夹 | `host.openPath(path)` | |
| `hosts.read` | 读取 hosts 文件 | `host.hosts.read()` | |
| `hosts.write` | 修改 hosts 文件（弹管理员授权） | `host.hosts.write(content)` | ⚠️ 高危 |
| `notification` | 发送系统通知 | `host.notify(title, body)` | |
| `network` | 访问网络（可连任意服务器） | `host.http(url, opts?)` | ⚠️ 高危 |

> **插件存储**（`host.storage.*`）是插件的私有数据，按插件 id 命名空间隔离，无法访问其他插件的数据，**不属于系统能力，无需在 permissions 中声明**。

## 文件系统作用域

fs 不再是全盘权限，而是按 **scope（作用域）** 授权。插件只能访问它声明且用户同意的目录。

| scope | 目录 | 权限 id（读 / 写） |
|---|---|---|
| `plugin` | 插件私有目录（`~/.config/pc-tool/plugin-data/<id>/files`） | **免授权** |
| `downloads` | `~/Downloads` | `fs.downloads.read` / `fs.downloads.write` |
| `desktop` | `~/Desktop` | `fs.desktop.read` / `fs.desktop.write` |
| `documents` | `~/Documents` | `fs.documents.read` / `fs.documents.write` |
| `temp` | 临时目录 | `fs.temp.read` / `fs.temp.write` |
| `home` | 整个 `~` | `fs.home.read` / `fs.home.write` |

在 `plugin.json` 中声明所需 scope 权限，例如：

```json
{
  "permissions": ["fs.downloads.read", "fs.downloads.write"]
}
```

`host.fs.*` 接受绝对路径；宿主会 canonicalize 路径并校验其落在已授权 scope 根目录之内，拒绝 `..` 与符号链接逃逸，越界路径报错。建议先调 `host.fs.scopes()` 获取根目录再拼路径，详见 [host.* API](./host-api)。

## 安装授权框：三档风险等级

安装时的授权框按权限风险分为三档显示：

| 等级 | 颜色 | 哪些权限 |
|---|---|---|
| **高危** | 红色 ⚠️ | `network`、`hosts.write`、`fs.home.read`、`fs.home.write`、以及任何 `fs.*.write` |
| **敏感** | 琥珀色 | 作用域文件只读：`fs.downloads.read`、`fs.desktop.read`、`fs.documents.read`、`fs.temp.read` |
| **普通** | 默认 | 其余（剪贴板、通知、`shell.open*`、`hosts.read` 等） |

**只要插件申请了任何高危或敏感权限，用户就必须勾选"我已了解上述权限的风险"才能完成安装。** 换言之，插件想读取你任何一个目录，安装时都需要你显式点头。

若插件同时申请任意 fs 读权限和 `network`，授权框会额外弹出红色警告："**该插件能读取你的文件并联网，可能上传你的数据**"。

## 在 plugin.json 中声明

```json
{
  "permissions": ["clipboard.read", "clipboard.write"]
}
```

## 权限生效条件

实际能否调用能力，取决于：

```
有效权限 = granted（用户授权）∩ declared（manifest 声明）
```

两个条件都满足时，调用才会真正到达系统。任一条件不满足，`host.*` 方法返回的 Promise 都会 reject，错误信息为 `"permission denied"`。

详见 [安全模型](./security)。
