// 把 vendor 里的 Cropper.js CSS/JS 内联进 index.src.html → index.html。
// 插件跑在 sandbox srcdoc iframe（opaque origin，无法加载相邻文件），故须内联为单文件。
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const dir = dirname(fileURLToPath(import.meta.url));
const css = readFileSync(join(dir, "vendor/cropper.min.css"), "utf8");
const js = readFileSync(join(dir, "vendor/cropper.min.js"), "utf8");

const out = readFileSync(join(dir, "index.src.html"), "utf8")
  .replace("/*__CROPPER_CSS__*/", () => css)
  .replace("/*__CROPPER_JS__*/", () => js);

writeFileSync(join(dir, "index.html"), out);
console.log(`built index.html (${out.length} bytes)`);
