# plugin.json 参考

`plugin.json` 是插件的清单文件，必须放在插件根目录。

## 顶层字段

| 字段 | 必填 | 类型 | 说明 |
|---|---|---|---|
| `id` | 是 | string | 反向域名唯一标识，如 `com.you.translate`。**必须是单个安全路径段**（不含 `/`、`\`、`..`），会被用作安装目录名。 |
| `name` | 是 | string | 显示名称 |
| `version` | 是 | string | 版本号，点分数字格式，如 `1.0.0` |
| `icon` | 否 | string | 图标路径（相对插件根目录），推荐使用 SVG |
| `features` | 是 | array | feature 数组，至少包含一个 |
| `permissions` | 否 | array | 申请的能力 id 数组，如 `["clipboard.read"]` |

> **注意**：`_dir`、`granted`、`source` 是运行时由宿主注入的字段，**不要**在 `plugin.json` 中手动填写。

## feature 对象

`features` 数组中每个元素的字段：

| 字段 | 必填 | 类型 | 说明 |
|---|---|---|---|
| `code` | 是 | string | 插件内唯一标识，用于 `host.redirect(code)` |
| `type` | 是 | string | `"ui"` 或 `"logic"` |
| `entry` | 是 | string | 入口文件路径（相对插件根）。`ui` → HTML 文件；`logic` → JS 文件 |
| `triggers` | 否 | array | 触发器数组，见下方说明 |

## 触发器（trigger）对象

| 字段 | 说明 |
|---|---|
| `kind` | 触发类型：`"keyword"`、`"regex"`、`"content"` |
| `value` | 触发值：keyword 为关键词字符串；regex 为正则表达式字符串；content 为内容类型（如 `"json"`） |

## 完整示例

```json
{
  "id": "com.you.translate",
  "name": "划词翻译",
  "version": "1.2.0",
  "icon": "icon.svg",
  "permissions": ["clipboard.read", "clipboard.write"],
  "features": [
    {
      "code": "translate",
      "type": "ui",
      "entry": "index.html",
      "triggers": [
        { "kind": "keyword", "value": "fy" },
        { "kind": "content", "value": "json" }
      ]
    },
    {
      "code": "quick",
      "type": "logic",
      "entry": "quick.js",
      "triggers": [
        { "kind": "regex", "value": "^[a-zA-Z ]+$" }
      ]
    }
  ]
}
```
