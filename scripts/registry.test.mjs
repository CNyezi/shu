import assert from "node:assert/strict";
import test from "node:test";
import { isRegistryFeed } from "../src/lib/registry.ts";

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
