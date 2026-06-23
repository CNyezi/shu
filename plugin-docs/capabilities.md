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
| `fs.read` | 读取你电脑上的文件 | `host.fs.read*()` / `host.fs.list()` / `host.fs.stat()` / `host.fs.exists()` | ⚠️ 高危 |
| `fs.write` | 写入/删除你电脑上的文件 | `host.fs.write*()` / `host.fs.mkdir()` / `host.fs.remove()` | ⚠️ 高危 |
| `notification` | 发送系统通知 | `host.notify(title, body)` | |
| `network` | 访问网络（可连任意服务器） | `host.http(url, opts?)` | ⚠️ 高危 |

> **插件存储**（`host.storage.*`）是插件的私有数据，按插件 id 命名空间隔离，无法访问其他插件的数据，**不属于系统能力，无需在 permissions 中声明**。

## 高危权限说明

高危权限（`fs.read`、`fs.write`、`network`）在安装授权框里会**标红**并要求用户单独勾选确认。

若插件同时申请 `fs.read` 和 `network`，授权框会额外提示"**可能上传你的数据**"，提醒用户注意数据安全风险。

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
