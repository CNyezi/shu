import assert from "node:assert/strict";
import test from "node:test";
import { DEFAULT_TEST_PACKAGE, isTestPath, testPackagePath } from "../src/lib/testMode.ts";

test("/test is enabled only in dev", () => {
  assert.equal(isTestPath("/test", true), true);
  assert.equal(isTestPath("/test", false), false);
  assert.equal(isTestPath("/", true), false);
});

test("test package path defaults to /tmp package", () => {
  assert.equal(testPackagePath(""), DEFAULT_TEST_PACKAGE);
  assert.equal(testPackagePath("?package=/tmp/x.pcp"), "/tmp/x.pcp");
});
