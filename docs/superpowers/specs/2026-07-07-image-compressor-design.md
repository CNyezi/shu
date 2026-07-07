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

- 进入：关键词 `yasuo` / `compress` / `压缩`。不加 content 触发器（避免劫持）。
- 取图（打开时按序尝试）：
  1. `clipboard.readFiles` 找到 `.png` 文件 → 记住源路径，压缩用 `image_compress({path})`，"覆盖源文件"可用。
  2. 否则 `clipboard.readImage`（截图/复制的图片内容）→ 有像素则压缩用 `image_compress({base64})`，"覆盖源文件"禁用（无源路径），只能另存为。
  3. 都没有 → 提示"剪贴板没有图片，复制一张图片或截图后重开"。
- 界面：预览缩略图 + 原始/压缩后大小与压缩率 + 质量滑杆（1–100，默认 65，松手防抖重压）+ 两个动作按钮。
- 动作：
  - 「覆盖源文件」：`save_file_dialog(源路径, 压缩字节)`，对话框预填源路径，用户回车即替换。无源路径时禁用。
  - 「另存为」：`save_file_dialog(下载目录/原名-min.png, 压缩字节)`。
  - 成功 toast 落盘路径与体积；取消静默；失败显示原因。

## 权限声明

`plugin.json` permissions：`["clipboard.readImage","clipboard.readFiles","image.compress","dialog.saveFile"]`——全 normal 档，安装确认无红色高危项。

## 文件

- 宿主：`src-tauri/src/lib.rs`（两个命令 + invoke_handler）、`src/lib/host.ts`（capabilities 映射 + 标签）、`src/lib/capabilities.ts`（两个权限 label/tier）、`src/lib/pluginRuntime.ts` BOOTSTRAP（`host.image.compress` / `host.saveFile` 包装）、`capabilities/default.json`（dialog:allow-save）、`Cargo.toml`（imagequant + lodepng）。
- 插件：`plugins/image-compressor/{plugin.json,index.html,icon.svg}`。

## 验收

cargo test（新增压缩往返测试：造一张 PNG → 压缩 → 断言 after<before 且输出仍是合法 PNG）；pnpm 三关；dev 重启，复制截图 → `yasuo` → 拖质量看压缩率 → 另存为落盘。发布注册中心待用户确认。

## 不做（YAGNI）

JPEG/WebP、批量、拖拽进窗口、EXIF 保留选项、压缩历史。
