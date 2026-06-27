import assert from "node:assert/strict";
import { access, readFile } from "node:fs/promises";
import test from "node:test";

const exists = async (path) => access(path).then(() => true, () => false);
const read = (path) => readFile(path, "utf8");

test("plugin template includes manifest, app, docs, and release workflow", async () => {
  const files = [
    "templates/plugin-template/plugin.json",
    "templates/plugin-template/index.html",
    "templates/plugin-template/README.md",
    "templates/plugin-template/.github/workflows/release.yml",
  ];
  for (const file of files) assert.equal(await exists(file), true, file);

  const manifest = JSON.parse(await read("templates/plugin-template/plugin.json"));
  const workflow = await read("templates/plugin-template/.github/workflows/release.yml");

  assert.match(manifest.id, /^com\.example\./);
  assert.equal(manifest.features?.[0]?.entry, "index.html");
  assert.deepEqual(manifest.permissions, ["notification"]);
  assert.match(workflow, /tags:/);
  assert.match(workflow, /zip -qr/);
  assert.match(workflow, /gh release create/);
});

test("registry template accepts repo submissions and builds registry", async () => {
  const files = [
    "templates/registry-template/package.json",
    "templates/registry-template/registry.json",
    "templates/registry-template/scripts/registry-intake.mjs",
    "templates/registry-template/submissions/example.json",
    "templates/registry-template/README.md",
    "templates/registry-template/.github/workflows/validate-submissions.yml",
  ];
  for (const file of files) assert.equal(await exists(file), true, file);

  const registry = JSON.parse(await read("templates/registry-template/registry.json"));
  const submission = JSON.parse(await read("templates/registry-template/submissions/example.json"));
  const workflow = await read("templates/registry-template/.github/workflows/validate-submissions.yml");

  assert.deepEqual(registry, { version: 1, plugins: [] });
  assert.match(submission.repo, /^https:\/\/github\.com\//);
  assert.match(workflow, /submissions\/\*\.json/);
  assert.match(workflow, /registry:intake/);
  assert.match(await read("templates/registry-template/package.json"), /"registry:intake"/);
});
