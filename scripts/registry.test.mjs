import assert from "node:assert/strict";
import test from "node:test";
import { OFFICIAL_REGISTRY_URL, isOfficialRegistry, registriesWithOfficial, isRegistryFeed } from "../src/lib/registry.ts";

test("validates registry feed shape", () => {
  assert.equal(isRegistryFeed({
    version: 1,
    plugins: [{
      id: "com.you.hello",
      name: "Hello",
      version: "1.0.0",
      description: "demo",
      permissions: ["clipboard.read"],
      packageUrl: "https://example.com/hello.pcp",
      sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    }],
  }), true);

  assert.equal(isRegistryFeed({ version: 2, plugins: [] }), false);
});

test("merges optional official registry before user registries", () => {
  assert.deepEqual(
    registriesWithOfficial(["https://example.com/other.json"], "https://example.com/registry.json"),
    ["https://example.com/registry.json", "https://example.com/other.json"],
  );

  assert.deepEqual(
    registriesWithOfficial(["https://example.com/registry.json"], "https://example.com/registry.json"),
    ["https://example.com/registry.json"],
  );

  assert.deepEqual(registriesWithOfficial(["https://example.com/other.json"], ""), ["https://example.com/other.json"]);
});

test("recognizes the official registry URL", () => {
  assert.equal(isOfficialRegistry("https://example.com/registry.json", "https://example.com/registry.json"), true);
  assert.equal(isOfficialRegistry("https://example.com/registry.json", ""), false);
  assert.equal(isOfficialRegistry("https://example.com/other.json", "https://example.com/registry.json"), false);
});

test("defaults to the official CNyezi registry", () => {
  assert.equal(OFFICIAL_REGISTRY_URL, "https://raw.githubusercontent.com/CNyezi/shu-registry/main/registry.json");
});
