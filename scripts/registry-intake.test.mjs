import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { execFileSync } from "node:child_process";
import test from "node:test";
import { createRegistryEntry, updateRegistry } from "./registry-intake.mjs";

async function makePackage(dir, manifest) {
  const pluginDir = join(dir, "plugin");
  await mkdir(pluginDir);
  await writeFile(join(pluginDir, "plugin.json"), JSON.stringify(manifest));
  const pcpPath = join(dir, "plugin.pcp");
  execFileSync("zip", ["-qr", pcpPath, "."], { cwd: pluginDir });
  return pcpPath;
}

test("creates registry entry from package metadata", async () => {
  const dir = await mkdtemp(join(tmpdir(), "shu-intake-"));
  try {
    const manifest = {
      id: "com.you.hello",
      name: "Hello",
      version: "1.0.0",
      description: "Demo",
      permissions: ["clipboard.read"],
    };
    const packagePath = await makePackage(dir, manifest);

    const entry = await createRegistryEntry(packagePath);

    assert.deepEqual(entry, {
      ...manifest,
      packageUrl: packagePath,
      sha256: entry.sha256,
    });
    assert.match(entry.sha256, /^[0-9a-f]{64}$/);
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});

test("updates registry and replaces same id/version from same URL", async () => {
  const dir = await mkdtemp(join(tmpdir(), "shu-intake-"));
  try {
    const registryPath = join(dir, "registry.json");
    const first = {
      id: "com.you.hello",
      name: "Hello",
      version: "1.0.0",
      description: "Demo",
      permissions: [],
      packageUrl: "https://example.com/old.pcp",
      sha256: "0".repeat(64),
    };
    const second = { ...first, sha256: "1".repeat(64) };

    await updateRegistry(registryPath, first);
    await updateRegistry(registryPath, second);

    const registry = JSON.parse(await readFile(registryPath, "utf8"));
    assert.deepEqual(registry, { version: 1, plugins: [second] });
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});

test("rejects duplicate id/version from different URL", async () => {
  const dir = await mkdtemp(join(tmpdir(), "shu-intake-"));
  try {
    const registryPath = join(dir, "registry.json");
    const first = {
      id: "com.you.hello",
      name: "Hello",
      version: "1.0.0",
      description: "Demo",
      permissions: [],
      packageUrl: "https://example.com/old.pcp",
      sha256: "0".repeat(64),
    };
    const second = { ...first, packageUrl: "https://example.com/new.pcp", sha256: "1".repeat(64) };

    await updateRegistry(registryPath, first);
    await assert.rejects(() => updateRegistry(registryPath, second), /duplicate plugin version/);
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});

test("rejects non-http URL strings", async () => {
  await assert.rejects(() => createRegistryEntry("ftp://example.com/plugin.pcp"), /only http/);
});

test("rejects invalid packages", async () => {
  const dir = await mkdtemp(join(tmpdir(), "shu-intake-"));
  try {
    const badPath = join(dir, "bad.pcp");
    await writeFile(badPath, "not a zip");

    await assert.rejects(() => createRegistryEntry(badPath), /plugin.json/);
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});

test("cli writes registry file", async () => {
  const dir = await mkdtemp(join(tmpdir(), "shu-intake-"));
  try {
    const packagePath = await makePackage(dir, {
      id: "com.you.cli",
      name: "CLI",
      version: "1.0.0",
      description: "Demo",
      permissions: [],
    });
    const registryPath = join(dir, "registry.json");

    execFileSync("node", ["scripts/registry-intake.mjs", packagePath, registryPath], {
      cwd: process.cwd(),
      stdio: ["ignore", "pipe", "pipe"],
    });

    const registry = JSON.parse(await readFile(registryPath, "utf8"));
    assert.equal(registry.plugins[0].id, "com.you.cli");
    assert.match(registry.plugins[0].sha256, /^[0-9a-f]{64}$/);
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});
