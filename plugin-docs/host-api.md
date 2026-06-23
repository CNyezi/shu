# host.* API

插件内通过全局 `window.host` 对象与宿主交互。所有能力方法（需要 `permissions` 的方法）在未授权时返回的 Promise 会 reject。

## 基础方法

### `host.onInput(callback)`

注册回调函数，用户在启动器输入时，回调会收到当前输入字符串。**仅逻辑插件使用。**

```js
host.onInput(function (q) {
  // q 是用户当前输入的字符串
  console.log('用户输入：', q)
})
```

---

### `host.getContext()`

返回当前上下文对象。目前结构为 `{ input: string }`。

```js
const ctx = host.getContext()
console.log(ctx.input) // 当前输入
```

---

### `host.setResults(results)`

将结果数组推送到启动器列表。**仅逻辑插件使用。**

每个结果项形如 `{ title, subtitle }`：

```js
host.setResults([
  { title: '结果一', subtitle: '副标题' },
  { title: '结果二', subtitle: '描述信息' }
])
```

---

### `host.redirect(code)`

跳转到本插件的另一个 feature，参数为目标 feature 的 `code`。

```js
host.redirect('detail')
```

---

### `host.close()`

关闭插件，返回启动器搜索状态。

```js
host.close()
```

---

## 能力方法

以下方法需要在 `plugin.json` 的 `permissions` 中声明对应权限，且用户安装时授权，才能正常使用。

### `host.clipboard.read()`

读取剪贴板文本内容。需要 `clipboard.read` 权限。

**返回**：`Promise<{ kind: "text" | "empty", text: string }>`

```js
host.clipboard.read().then(c => {
  if (c.kind === 'text') {
    console.log('剪贴板内容：', c.text)
  }
})
```

---

### `host.clipboard.write(text)`

写入文本到剪贴板。需要 `clipboard.write` 权限。

**返回**：`Promise<void>`

```js
host.clipboard.write('要写入的文本').then(() => {
  console.log('写入成功')
})
```

---

### `host.openUrl(url)`

用系统默认浏览器打开网址。需要 `shell.openUrl` 权限。

**返回**：`Promise<void>`

```js
host.openUrl('https://example.com')
```

---

### `host.openPath(path)`

用系统默认程序打开文件或文件夹。需要 `shell.openPath` 权限。

**返回**：`Promise<void>`

```js
host.openPath('/Users/you/Downloads')
```
