# 能力清单

插件需要在 `plugin.json` 的 `permissions` 字段声明所需能力，并在用户安装时获得授权，才能调用对应的 `host.*` API。

## 当前支持的能力

| 能力 id | 说明 | 对应 API |
|---|---|---|
| `clipboard.read` | 读取剪贴板文本 | `host.clipboard.read()` |
| `clipboard.write` | 写入剪贴板文本 | `host.clipboard.write(text)` |
| `shell.openUrl` | 用默认浏览器打开网址 | `host.openUrl(url)` |
| `shell.openPath` | 用默认程序打开文件/文件夹 | `host.openPath(path)` |

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
