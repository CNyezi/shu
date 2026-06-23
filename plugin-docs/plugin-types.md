# 插件类型与触发

## 插件类型

### UI 插件（`type: "ui"`）

`entry` 指向一个 HTML 文件。该文件被加载进沙箱 iframe，渲染在启动器内容区。适合需要交互界面的场景，例如编辑器、查看器等。

```json
{
  "code": "viewer",
  "type": "ui",
  "entry": "index.html",
  "triggers": [{ "kind": "keyword", "value": "view" }]
}
```

### 逻辑插件（`type: "logic"`）

`entry` 指向一个 JS 文件，插件在后台隐藏运行。通过 `host.onInput` 接收用户在启动器实时输入的字符串，再调用 `host.setResults` 将结果回填到启动器列表。适合"输入查询→看结果"类功能。

```json
{
  "code": "search",
  "type": "logic",
  "entry": "main.js",
  "triggers": [{ "kind": "keyword", "value": "search" }]
}
```

## 触发器（triggers）

每个 feature 可以声明一个或多个触发器，决定插件在何种情况下被激活。

### keyword

用户在启动器输入该关键词时，直接进入此 feature。

```json
{ "kind": "keyword", "value": "fy" }
```

### regex

启动器输入匹配该正则时，此 feature 作为一条结果出现在列表中。

```json
{ "kind": "regex", "value": "^[0-9]+$" }
```

### content

唤起时若剪贴板内容被识别为该类型，此 feature 会被推荐。若只有一个适配插件，则直接打开。目前内置 `json` 检测。

```json
{ "kind": "content", "value": "json" }
```
