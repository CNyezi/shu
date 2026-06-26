import assert from "node:assert/strict";
import test from "node:test";
import { canUseCapability, effectivePermissions } from "../src/lib/capabilities.ts";

test("effective permissions are granted intersect declared", () => {
  assert.deepEqual(
    effectivePermissions(["clipboard.read"], ["clipboard.read", "shell.openUrl"]),
    ["clipboard.read"],
  );
});

test("network.http requires the network grant", () => {
  assert.equal(canUseCapability(["network"], ["network"], "network.http"), true);
  assert.equal(canUseCapability(["network"], [], "network.http"), false);
});
