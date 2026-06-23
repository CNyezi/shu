# pc-tool 插件分享 ① —— 包格式 + 安装/卸载 + 权限授权门

- 日期：2026-06-23
- 状态：待 review
- 范围：插件分享的**第一个子系统（地基）**。完整市场拆为 ①包格式+安装+授权门 → ②自托管注册表 → ③市场 UI → ④发布流程；本 spec 只覆盖 ①。

## 1. 背景与目标

让用户能安装、运行**别人分享的、不可信的**插件，安全可控。这是"插件分享/市场"的地基：在能浏览/评分/自动更新之前，必须先有「打包 → 分发 → 安装 → 授权 → 在沙箱里运行 → 卸载」的闭环。

与 uTools 的根本差异：uTools 插件以**完整 Node.js 权限**运行，安全靠"preload.js 必须可读 + 市场审核"。pc-tool 插件跑在 **sandbox iframe**，只能通过宿主中介的能力桥按白名单调用系统能力。因此 pc-tool 的信任模型是**技术隔离 + 安装时显式授权**，而非人肉审代码。

**①的成功标准**：把一个插件打成 `.pcp`，发给别人（文件或链接），对方安装时看到它申请的能力并授权，插件随后在沙箱内按授权运行；可在内置管理界面查看与卸载。

## 2. 范围

### 内
- `.pcp` 包格式（zip）+ 完整性 sha256
- 安装入口：本地文件、URL、拖拽
- 安装时一次性权限授权门；更新提权需重新同意
- 卸载
- 内置「插件管理」界面（列已装插件、授权、来源、卸载、安装入口）
- 运行时白名单改为按**授权集**放行
- 作者打包命令 `pack_plugin`

### 外（后续子系统）
- ②自托管注册表 / 去中心化发现
- ③市场 UI（浏览/搜索/评分/自动更新）
- ④发布流程、发布者身份、**密码学签名**（依赖②的身份体系）
- 运行时逐能力弹框（C 模型）；细粒度撤销单个权限的 UI（注册表已为其留好结构）
- 能力广度扩展（fs/网络/截图等）——独立的"能力 API"线

## 3. 架构

### 3.1 包格式 `.pcp`
插件文件夹的 **zip**，根目录为 `plugin.json`。安装 = 解压到已装目录。完整性用 **sha256**（展示给用户），v1 **不做签名**（签名/发布者验证依赖②的身份体系）。

### 3.2 两类插件目录
| 类型 | 位置 | 授权 |
|---|---|---|
| 内置插件 | dev：仓库 `plugins/`；release：app 资源目录 | = manifest 全集（第一方可信，不弹框） |
| 已装插件 | `~/.config/pc-tool/plugins/<id>/`（dev & release 均用） | = 注册表 `granted` |

`list_plugins` 合并扫描两类，每项标注 `source`（`bundled`/`installed`）与 `granted`。

### 3.3 注册表（授权与来源）
`~/.config/pc-tool/registry.json`：
```jsonc
{
  "plugins": {
    "com.x.foo": {
      "version": "1.2.0",
      "granted": ["clipboard.read", "shell.openUrl"],
      "source": "url",
      "origin": "https://example.com/foo.pcp",
      "sha256": "ab12…",
      "installedAt": "2026-06-23T10:00:00Z"
    }
  }
}
```
注册表是**授权的唯一真相来源**；运行时白名单 = `granted ∩ manifest.permissions`（不能授权超出声明的能力）。

## 4. 安装/卸载流程

统一安装管线（拖拽 / 文件选择 / URL 三入口汇入）：
```
入口
 → [URL] download_package(url) → 临时文件
 → inspect_package(path)：开 zip、读+校验 plugin.json、算 sha256
 → { manifest, sha256, isUpgrade, newPermissions[] }
 → 权限授权框（内容区视图）：图标/名称/版本 + 申请能力列表 + sha256
     · 全新：列全部申请能力
     · 升级提权：高亮 newPermissions，必须重新同意
     · 升级无提权：直接确认
 → 同意 → install_package(path, granted[])：解压到 plugins/<id>/、写注册表
 → 重新加载插件 → toast「已安装」
```

**错误/边界**：
- 包损坏 / 无 plugin.json / id 缺失 → inspect 报错，不进授权框。
- id 已存在 → 视为升级替换；新版本 `permissions ⊋ granted` → 授权框要求同意新增项。
- 版本号低于已装 → 阻止降级并提示。
- URL 下载失败 / 非法包 → 报错。
- 授权框取消 / 升级提权未同意 → 不安装，不动现有插件。

## 5. 接口

### 5.1 Rust 命令
| 命令 | 说明 |
|---|---|
| `inspect_package(path) -> {manifest, sha256, is_upgrade, new_permissions}` | 解析+校验+哈希，不落地 |
| `download_package(url) -> path` | 下载 `.pcp` 到临时文件 |
| `install_package(path, granted: string[]) -> ()` | 解压到已装目录 + 写注册表 |
| `uninstall_plugin(id) -> ()` | 删目录 + 删注册表项 |
| `list_installed() -> InstalledPlugin[]` | 已装插件 + granted + source + origin |
| `pack_plugin(src_dir, out_path) -> ()` | 文件夹打成 `.pcp`（作者用） |

`list_plugins`（已有）改为合并扫描内置 + 已装，返回项含 `granted`、`source`、`_dir`。

### 5.2 前端
- `host.ts`：上述命令封装。
- 内置「插件管理」视图（关键词 `插件`/`plugins`）：已装列表（图标/名称/版本/已授权能力/来源/卸载）+ `从文件安装` + `从 URL 安装`。
- 权限授权框视图：能力中文标签 + sha256 + 安装/取消。
- 能力名→中文标签映射（`clipboard.read`→"读取剪贴板" 等）。
- 窗口 `.pcp` 拖拽 → 安装管线。

### 5.3 运行时改动
- `pluginRuntime.mountPlugin` 的白名单从 `plugin.permissions` 改为 `granted ∩ manifest.permissions`（granted 由 `list_plugins` 提供）。内置插件 granted=manifest。

## 6. 依赖
- Rust：`zip`（解压/打包）、`sha2`（哈希）、`ureq`（URL 下载，轻量阻塞式）。
- `tauri-plugin-dialog`（文件选择器）。
- 文件拖拽：Tauri 窗口 `dragDropEnabled` + drop 事件 + 对应 capability。

## 7. 测试
- Rust 往返：`pack_plugin` → `inspect_package` → `install_package` → `list_installed` → `uninstall_plugin` 全链路；提权检测（新版本多声明权限 → `new_permissions` 非空）；降级阻止；损坏包报错。
- 前端：授权框正确列出申请能力；白名单走 granted（扩展已有"拒绝路径"测试：未授权能力被宿主拦在系统调用前）。

## 8. 验收清单
- [ ] `pack_plugin` 能把 `plugins/json-preview/` 打成 `xxx.pcp`。
- [ ] 把该 `.pcp` 拖进窗口 → 弹授权框列出 `clipboard.read`/`clipboard.write`（中文标签）+ sha256。
- [ ] 同意 → 插件装到 `~/.config/pc-tool/plugins/`，注册表写入 granted，插件可用。
- [ ] 关键词 `插件` 进管理界面，能看到已装插件及其授权与来源，可卸载。
- [ ] 卸载后目录与注册表项均删除，插件不再加载。
- [ ] 从 URL 安装走通同样的授权 → 安装闭环。
- [ ] 构造一个"提权"新版本（多一个权限）→ 升级时授权框高亮新增项，未同意则不安装。
- [ ] 把某插件 granted 改小（去掉一个权限）→ 该能力调用被宿主拒绝（验证白名单走 granted）。

## 9. 后续子系统（非本 spec）
②自托管注册表（去中心化发现/托管，对标 uTools 中心化市场，是核心差异点） · ③市场 UI · ④发布流程 + 发布者身份 + 签名 · 能力广度扩展 · 运行时逐能力授权。
