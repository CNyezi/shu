# 安全模型

## 沙箱隔离

插件运行在 `sandbox="allow-scripts"` 的 iframe 里（opaque origin）。这意味着：

- **无法访问 Node.js**
- **无法访问文件系统**
- **无法访问 Tauri 内部 API**

与系统交互的**唯一途径**是 `window.host.*` API。它通过 postMessage 将请求发给宿主，宿主按权限白名单决定是否放行。

## 权限白名单模型

能力调用的实际生效条件：

```
有效权限 = granted ∩ declared
```

- **granted**：用户在安装时授权的权限集合
- **declared**：插件 `plugin.json` 中 `permissions` 字段声明的权限集合

只有同时满足两个条件，能力调用才会真正到达系统。否则，`host.*` 返回的 Promise 会 reject，错误信息为 `"permission denied"`。

这意味着：

- 手改注册表（`~/.config/shu/registry.json`）无法绕过 manifest 中的声明限制。
- manifest 中声明权限，但用户未授权，调用同样会被拒绝。
- manifest 声明本身在用户同意授权之前不赋予任何权限。

## 与 uTools 的对比

| | 枢 | uTools |
|---|---|---|
| 隔离方式 | 技术隔离（sandbox iframe） | 依赖 preload 可读 + 市场审核 |
| 权限模型 | 安装时显式授权，技术层面强制 | 插件有完整 Node 权限 |

枢的安全模型是**技术隔离 + 安装时显式授权**，不依赖审核流程。

## 插件 ID 安全

插件的 `id` 字段在安装时会被校验，以防止路径穿越攻击（path traversal）。`id` 必须是单个安全路径段，不能包含 `/`、`\`、`..` 等字符，因为它会被直接用作安装目录名。
