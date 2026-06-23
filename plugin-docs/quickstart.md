# 快速上手

本章通过一个最小的 UI 插件 `hello` 带你走完完整开发流程。

## 1. 创建目录，编写 plugin.json

新建目录 `hello/`，在其中创建 `plugin.json`：

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

## 2. 编写入口 HTML

创建 `hello/index.html`，读取剪贴板并显示内容：

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

> `window.host` 是宿主注入的全局对象，提供所有与系统交互的能力。详见 [host.* API](./host-api)。

## 3. 打包

```bash
cd hello && zip -r ../hello.pcp .
```

打包结果是一个 `.pcp` 文件，实质是 `plugin.json` 位于 zip 根的压缩包。

## 4. 安装并运行

1. 打开 pc-tool 启动器，输入 `插件` 或 `plugins` 进入插件管理。
2. 将 `hello.pcp` 拖入窗口（或点击「从文件安装」选择文件）。
3. 弹出授权框，确认插件名称、id、申请的能力（此例为 `clipboard.read`）后同意安装。
4. 回到启动器，输入 `hello`，插件界面即出现在内容区。

## 下一步

- [插件类型与触发](./plugin-types) — 了解 UI 插件与逻辑插件的区别
- [plugin.json 参考](./manifest) — 完整字段说明
- [完整示例](./examples) — 更多可运行的示例
