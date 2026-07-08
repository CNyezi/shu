# 图片裁剪 + 缩放（并入 image-compressor 插件）

## 背景与选型

在已有的「图片压缩」插件里加入裁剪与缩放，与压缩串成一条流水线：**裁切 → 缩放 → 压缩 → 保存**。用户确认：用开源库做裁剪、多图允许逐个裁剪。

裁剪库选 **Cropper.js v1.6.2**（MIT、零依赖纯 JS，37KB JS + 4KB CSS）。插件跑在 `sandbox="allow-scripts"` 的 srcdoc iframe（opaque origin，无法加载相邻文件），故库须**内联**进单个 index.html。构建时用脚本把 `cropper.min.css` / `cropper.min.js` 注入 index.html 模板的占位标记，保留其 MIT 版权头。v2 为 web-component 架构、难内联，不用。

## 交互

- **列表**（沿用）：每张图一行，新增「裁剪」按钮 → 打开该图的**全窗口裁剪编辑器**。
- **裁剪编辑器**（Cropper.js 覆盖整个插件视图）：
  - 比例预设：自由 / 原图 / 1:1 / 4:3 / 16:9 / 3:2（切换即 `cropper.setAspectRatio()`；自由=`NaN`=任意裁切）。
  - 「应用」：记下该图裁切框（图像像素坐标 `getData()`），返回列表，行标「已裁剪 W×H」。「取消」：放弃返回。
- **底部全局控制**（沿用+新增）：质量滑杆 + 「最大边 ___ px」（留空=不缩放；对每张图裁切后的输出同比例缩小分辨率）。
- **压缩**：逐张 = 应用自身裁切（canvas `drawImage` 裁）→ 全局缩放（第二个 canvas 按最大边等比缩）→ `toDataURL('image/png')` → 现有 `host.image.compress({base64}, quality)`。未裁的图只做缩放+压缩。
- **保存 / 预览**：沿用（单图覆盖/另存，多图另存到文件夹，每行 Quick Look 预览）。

## 像素来源

- 截图 / 复制的图片内容：`clipboard.readImage` 已给 data URL，直接用。
- 复制的图片文件（`{path}`）：进裁剪或压缩前需原始像素——新增宿主能力 `image.read(path) -> { base64 }` 读文件字节返回 base64。normal 档，与 `image.compress({path})` 同类读原语，加同样的 `ponytail:` 安全注释（靠 network 高危档兜底 exfil）。
- 变换后统一走 `image.compress({base64})`；`{path}` 分支在有裁切/缩放时改用读到的像素走 `{base64}`，无变换时仍可直接 `{path}` 压缩。

## 文件与改动

- 宿主：`src-tauri/src/lib.rs`（`image_read` 命令 + handler）、`src/lib/host.ts`（capabilities `image.read`）、`src/lib/capabilities.ts`（label/tier normal）、`src/lib/pluginRuntime.ts`（`host.image.read`）。
- 插件：`plugins/image-compressor/plugin.json`（加 `image.read` 权限）、`index.html`（内联 Cropper.js + 编辑器 UI + 裁切/缩放/压缩流水线）。
- 构建脚本：`plugins/image-compressor/build.mjs` 把 `vendor/cropper.min.{css,js}` 内联进 `index.src.html` → `index.html`；`vendor/` 存 Cropper.js dist 与 LICENSE。

## 测试与验收

- cargo：`image_read` 往返（写临时 PNG → image_read → 断言 base64 解码回同样字节）。
- pnpm check / build。
- dev 重启：单图裁剪（各比例 + 自由）→ 最大边缩放 → 压缩 → 预览/保存；多图逐个裁剪后批量另存。

## 不做（YAGNI）

旋转/翻转、滤镜、批量统一裁切、裁剪历史、非 PNG 输出。
