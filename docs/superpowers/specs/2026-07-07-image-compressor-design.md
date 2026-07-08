# 图片压缩插件（com.shu.image-compressor）

## 背景与选型

用户要 TinyPNG 式压缩（有损色彩量化）、识别剪贴板图片、覆盖源文件或另存为、质量可拖动。确认方案 B：写盘走系统保存对话框（插件不申请文件系统权限）。

引擎选型：`imagequant`（pngquant/TinyPNG 同款量化库）+ `lodepng`（写索引 PNG），放宿主 Rust 侧——沙箱 iframe 里的 canvas 做不出真正的调色板量化。v1 只做 PNG（TinyPNG 的核心就是 pngquant）；JPEG（mozjpeg）留 v2。

## 宿主新增能力（Rust）

1. `image_compress(source, quality) -> { data: base64_png, before: u64, after: u64 }`
   - `source` 二选一：`{ base64 }`（截图/复制的图片内容）或 `{ path }`（Finder 复制的图片文件，由宿主读取字节——macOS 复制文件时剪贴板只有路径无像素）。
   - imagequant `set_quality(max(0,q-30), q)`，q=1..100；lodepng 编码索引 PNG。
   - 输入非 PNG（如 JPEG 字节）时 err，前端提示"当前仅支持 PNG"。
   - 能力名 `image.compress`，权限 `image.compress`（normal 档：纯本地计算；path 变体让宿主读一个用户自己复制的文件，压缩结果只回到无网络权限的本插件，无外发路径）。
2. `save_file_dialog(default_path, base64_data) -> Result<String, String>`
   - 用 tauri-plugin-dialog 的 save 面板（预填 default_path），用户确认后宿主写入字节，返回落盘路径；取消返回特定 err（前端静默）。
   - 能力名 `dialog.saveFile`，权限 `dialog.save`（normal 档，每次写盘由用户在原生面板亲自确认，等价授权）。
   - capabilities/default.json 增加 `dialog:allow-save`。

两个能力都通用，日后别的插件可复用。off-main-thread（spawn_blocking），量化是 CPU 密集。

## 插件（iframe UI）

**不自动压缩**：打开只收集待压图片并列出，用户点「压缩」才动手（可能是多图，且压缩是有损操作，需明确确认）。

- 进入：关键词 `yasuo` / `compress` / `压缩`。不加 content 触发器（避免劫持）。
- 收集输入（打开时按序尝试，不压缩）：
  1. `clipboard.readFiles` 找出**全部** `.png` 文件（多图）→ 列表，每项记源路径（`image_compress({path})`）。
  2. 否则 `clipboard.readImage`（截图/复制的图片内容）→ 单张，无源路径（`image_compress({base64})`）。
  3. 都没有 → 提示"剪贴板没有图片，复制 PNG 或截图后重开"。
- 界面：待压列表（缩略图/文件名 + 单张前后大小）+ 顶部总大小与总压缩率 + 质量滑杆（1–100，默认 65）+ 「压缩 N 张」按钮 + 保存按钮。
- 流程：点「压缩 N 张」→ 逐张调 `image_compress`，显示每张与合计的前后大小、省了多少。改质量后按钮变回「重新压缩」。
- 保存：
  - **多图** → 「另存到文件夹…」：`save_files_dialog(默认目录, [{name, base64}])` 弹一次文件夹选择框，批量写入 `原名-min.png`。
  - **单图（有源路径）** → 「覆盖源文件」`save_file_dialog(源路径, 字节)` + 「另存为」`save_file_dialog(下载目录/原名-min.png, 字节)`。
  - **单图（截图，无源路径）** → 仅「另存为」。
  - 成功 toast 落盘位置与数量；取消静默；失败显示原因。
- **批量原地覆盖不做**（需"往任意路径批量写"的写原语，与方案 B 的规避初衷冲突）；多图统一走"另存到文件夹"。

## 宿主再增能力（多图批量保存）

`save_files_dialog(default_dir: Option<String>, files: [{name, base64}]) -> Result<{dir, count}, String>`
- `blocking_pick_folder()`（预填 default_dir）选目录；用户确认后把每个 `{name, base64}` 解码写入该目录，返回目录与写入数。取消 → `Err("__cancelled__")`。
- 能力名 `dialog.saveFiles`，权限 `dialog.saveFiles`（normal 档，落盘目录由用户在原生面板选定）。文件夹选择复用 `dialog:allow-open`，无需新 capability 声明。

## 权限声明

`plugin.json` permissions：`["clipboard.readImage","clipboard.readFiles","image.compress","dialog.saveFile"]`——全 normal 档，安装确认无红色高危项。

## 文件

- 宿主：`src-tauri/src/lib.rs`（两个命令 + invoke_handler）、`src/lib/host.ts`（capabilities 映射 + 标签）、`src/lib/capabilities.ts`（两个权限 label/tier）、`src/lib/pluginRuntime.ts` BOOTSTRAP（`host.image.compress` / `host.saveFile` 包装）、`capabilities/default.json`（dialog:allow-save）、`Cargo.toml`（imagequant + lodepng）。
- 插件：`plugins/image-compressor/{plugin.json,index.html,icon.svg}`。

## 验收

cargo test（新增压缩往返测试：造一张 PNG → 压缩 → 断言 after<before 且输出仍是合法 PNG）；pnpm 三关；dev 重启，复制截图 → `yasuo` → 拖质量看压缩率 → 另存为落盘。发布注册中心待用户确认。

## 不做（YAGNI）

JPEG/WebP、批量、拖拽进窗口、EXIF 保留选项、压缩历史。
