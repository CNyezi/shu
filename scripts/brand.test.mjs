import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const readJson = async (path) => JSON.parse(await readFile(path, "utf8"));
const read = (path) => readFile(path, "utf8");

test("uses shu package and app branding", async () => {
  const pkg = await readJson("package.json");
  const tauri = await readJson("src-tauri/tauri.conf.json");
  const cargo = await read("src-tauri/Cargo.toml");
  const html = await read("index.html");

  assert.equal(pkg.name, "shu");
  assert.equal(tauri.productName, "shu");
  assert.equal(tauri.identifier, "com.yezi.shu");
  assert.equal(tauri.app.windows[0].title, "枢");
  assert.match(cargo, /^name = "shu"$/m);
  assert.match(cargo, /^name = "shu_lib"$/m);
  assert.match(html, /<title>枢<\/title>/);
});

test("uses shu runtime paths and plugin ids", async () => {
  const lib = await read("src-tauri/src/lib.rs");
  const plugins = await read("src-tauri/src/plugins.rs");
  const main = await read("src-tauri/src/main.rs");
  const testMode = await read("src/lib/testMode.ts");
  const harness = await read("src/lib/TestHarness.svelte");

  assert.match(lib, /join\("shu\/plugin-data"\)/);
  assert.match(plugins, /join\("shu"\)/);
  assert.match(main, /shu_lib::run\(\)/);
  assert.match(testMode, /\/tmp\/shu-json-preview\.pcp/);
  const oldNames = new RegExp(["pc" + "-tool", "pc" + "_tool", "pc" + "tool"].join("|"));
  assert.doesNotMatch(`${lib}\n${plugins}\n${main}\n${testMode}\n${harness}`, oldNames);
});

test("bundled plugin ids use com.shu namespace", async () => {
  const pluginPaths = [
    "plugins/json-preview/plugin.json",
    "plugins/hosts-editor/plugin.json",
    "plugins/storage-fs-demo/plugin.json",
  ];
  for (const path of pluginPaths) {
    const plugin = await readJson(path);
    assert.match(plugin.id, /^com\.shu\./);
  }
});
