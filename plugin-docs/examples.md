# 完整示例

## 示例一：Hello（UI 插件）

一个读取剪贴板内容并显示的最小 UI 插件。

### 目录结构

```
hello/
├── plugin.json
└── index.html
```

### plugin.json

```json
{
  "id": "com.you.hello",
  "name": "Hello",
  "version": "1.0.0",
  "features": [
    {
      "code": "hello",
      "type": "ui",
      "entry": "index.html",
      "triggers": [{ "kind": "keyword", "value": "hello" }]
    }
  ],
  "permissions": ["clipboard.read"]
}
```

### index.html

```html
<!doctype html>
<html><head><meta charset="utf-8"></head>
<body><pre id="out">读取剪贴板…</pre>
<script>
  host.clipboard.read().then(c => {
    document.getElementById("out").textContent = c.text || "（剪贴板为空）";
  });
</script>
</body></html>
```

### 打包与安装

```bash
cd hello && zip -r ../hello.pcp .
```

在启动器输入 `插件` → 拖入 `hello.pcp` → 授权 → 输入 `hello` 运行。

---

## 示例二：upper（逻辑插件）

一个把用户输入转为大写并作为结果列表显示的逻辑插件。

### 目录结构

```
upper/
├── plugin.json
└── main.js
```

### plugin.json

```json
{
  "id": "com.you.upper",
  "name": "Upper",
  "version": "1.0.0",
  "features": [
    {
      "code": "upper",
      "type": "logic",
      "entry": "main.js",
      "triggers": [{ "kind": "keyword", "value": "upper" }]
    }
  ]
}
```

注意：此插件不需要任何 `permissions`，因此省略该字段。

### main.js

```js
host.onInput(function (q) {
  if (!q) { host.setResults([]); return; }
  host.setResults([{ title: q.toUpperCase(), subtitle: "转大写结果" }]);
});
```

### 打包与安装

```bash
cd upper && zip -r ../upper.pcp .
```

在启动器输入 `upper`，然后继续输入任意字母，结果列表会实时显示大写转换结果。

---

## 参考：内置示例插件

仓库内置插件可以作为真实参考：

- `plugins/json-preview/`：剪贴板 JSON 编辑器，演示 `clipboard.read` / `clipboard.write`。
- `plugins/hosts-editor/`：hosts 编辑器，演示高危权限 `hosts.read` / `hosts.write`。
- `plugins/storage-fs-demo/`：无权限插件，演示 `host.storage.*` 和插件私有目录 `host.fs.scopes().plugin`。
