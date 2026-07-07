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

用户可用 `↑/↓` 在结果间移动，`Enter` 或点击会把该项的 `title` 复制到剪贴板。

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

---

## 剪贴板（图片 / 文件）

### `host.clipboard.readImage()`

读取剪贴板中的图片，返回 PNG data URL；剪贴板无图片时返回 `null`。需要 `clipboard.readImage` 权限。

**返回**：`Promise<string | null>`

```js
host.clipboard.readImage().then(dataUrl => {
  if (dataUrl) {
    document.querySelector('img').src = dataUrl
  }
})
```

---

### `host.clipboard.writeImage(dataUrl)`

将图片写入剪贴板（接受 data URL）。需要 `clipboard.writeImage` 权限。

**返回**：`Promise<void>`

```js
host.clipboard.writeImage(dataUrl)
```

---

### `host.clipboard.readFiles()`

读取剪贴板中的文件路径列表。需要 `clipboard.readFiles` 权限。

**返回**：`Promise<string[]>`

```js
host.clipboard.readFiles().then(paths => {
  console.log('剪贴板文件：', paths)
})
```

---

### `host.clipboard.writeFiles(paths)`

将文件路径列表复制到剪贴板。需要 `clipboard.writeFiles` 权限。

**返回**：`Promise<void>`

```js
host.clipboard.writeFiles(['/Users/you/report.pdf'])
```

---

## 文件系统

插件只能访问它声明并由用户授权的目录（scope）。路径必须落在已授权 scope 的根目录之内；宿主会 canonicalize 路径并拒绝 `..` 与符号链接逃逸，越界路径报错。各 scope 对应的权限 id 见[能力清单](./capabilities)。

### `host.fs.scopes()`

返回当前插件**可用的目录根**，包含免授权的 `plugin` scope 以及所有已授权的 scope。**插件应先调此方法取得根目录，再在其下拼绝对路径。**

**返回**：`Promise<{ [scope: string]: string }>`

```js
const roots = await host.fs.scopes()
// 例：{ plugin: "/Users/you/.config/shu/plugin-data/com.you.x/files", downloads: "/Users/you/Downloads" }
await host.fs.writeText(roots.plugin + '/notes.txt', 'hello')
```

---

### `host.fs.readText(path)`

读取文件内容为字符串。

**返回**：`Promise<string>`

```js
const text = await host.fs.readText('/Users/you/note.txt')
```

---

### `host.fs.readBytes(path)`

读取文件内容为 base64 字符串。

**返回**：`Promise<string>`

```js
const b64 = await host.fs.readBytes('/Users/you/image.png')
```

---

### `host.fs.list(path)`

列出目录内容。

**返回**：`Promise<{ name: string, path: string, is_dir: boolean }[]>`

```js
const entries = await host.fs.list('/Users/you/Documents')
entries.forEach(e => console.log(e.name, e.is_dir ? '(目录)' : ''))
```

---

### `host.fs.exists(path)`

检查路径是否存在。

**返回**：`Promise<boolean>`

```js
if (await host.fs.exists('/Users/you/config.json')) {
  // 文件存在
}
```

---

### `host.fs.stat(path)`

获取文件/目录元信息。

**返回**：`Promise<{ is_dir: boolean, is_file: boolean, size: number }>`

```js
const info = await host.fs.stat('/Users/you/report.pdf')
console.log('大小：', info.size)
```

---

### `host.fs.writeText(path, content)`

将字符串写入文件（文件不存在则创建）。需要对应 scope 的写权限（见能力清单）。

**返回**：`Promise<void>`

```js
await host.fs.writeText('/Users/you/output.txt', 'hello')
```

---

### `host.fs.writeBytes(path, base64)`

将 base64 内容写入文件。需要对应 scope 的写权限（见能力清单）。

**返回**：`Promise<void>`

```js
await host.fs.writeBytes('/Users/you/image.png', b64String)
```

---

### `host.fs.mkdir(path)`

创建目录（含中间路径）。需要对应 scope 的写权限（见能力清单）。

**返回**：`Promise<void>`

```js
await host.fs.mkdir('/Users/you/new-folder/sub')
```

---

### `host.fs.remove(path)`

删除文件或目录。需要对应 scope 的写权限（见能力清单）。

**返回**：`Promise<void>`

```js
await host.fs.remove('/Users/you/temp.txt')
```

---

## 通知

### `host.notify(title, body)`

发送系统通知。需要 `notification` 权限。

**返回**：`Promise<void>`

```js
host.notify('任务完成', '文件已处理完毕')
```

---

## 网络

### `host.http(url, opts?)`

由宿主代发 HTTP 请求，绕过浏览器 CORS 限制——适合调用没有 CORS 头的 API。需要 `network` 权限。

**参数**：`opts?: { method?: string, headers?: Record<string, string>, body?: string }`

**返回**：`Promise<{ status: number, body: string }>`

```js
const res = await host.http('https://api.example.com/data', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ query: 'hello' })
})
console.log(res.status, res.body)
```

> **WebSocket**：插件 iframe 内可直接使用原生 `new WebSocket(url)`，WebSocket 不受 CORS 限制，无需经过宿主。

---

## hosts 文件

内置的「Hosts 编辑器」插件（关键词 `hosts`）即使用这两个能力构建。

### `host.hosts.read()`

读取 `/etc/hosts` 文件内容。需要 `hosts.read` 权限。

**返回**：`Promise<string>`

```js
const content = await host.hosts.read()
console.log(content)
```

---

### `host.hosts.write(content)`

写入 `/etc/hosts` 文件。因为该文件归 root 所有，**保存时会弹出 macOS 系统管理员密码框**要求授权。需要 `hosts.write` 权限。

**返回**：`Promise<void>`

```js
await host.hosts.write(newContent)
```

---

## 插件存储

插件私有的键值存储，按插件 id 命名空间隔离，**无法访问其他插件的数据**。**无需任何权限**。

### `host.storage.get(key)`

读取存储值，键不存在时返回 `null`。

**返回**：`Promise<any>`

```js
const count = await host.storage.get('run_count') ?? 0
```

---

### `host.storage.set(key, value)`

写入存储值，`value` 可为任意 JSON 值。

**返回**：`Promise<void>`

```js
await host.storage.set('run_count', count + 1)
```

---

### `host.storage.remove(key)`

删除存储项。

**返回**：`Promise<void>`

```js
await host.storage.remove('run_count')
```

---

### `host.storage.keys()`

列出所有存储键。

**返回**：`Promise<string[]>`

```js
const keys = await host.storage.keys()
```
