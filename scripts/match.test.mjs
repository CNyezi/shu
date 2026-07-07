import assert from "node:assert/strict";
import test from "node:test";
import { matchScore } from "../src/lib/match.ts";

test("name prefix beats substring", () => {
  const prefix = matchScore("saf", { name: "Safari" });
  const sub = matchScore("afa", { name: "Safari" });
  assert.ok(prefix > sub && sub > 0);
});

test("word initials match: vsc -> Visual Studio Code", () => {
  assert.ok(matchScore("vsc", { name: "Visual Studio Code" }) > 0);
});

test("pinyin initials and full pinyin match", () => {
  const t = { name: "微信", pinyin: "weixin", initials: "wx" };
  assert.ok(matchScore("wx", t) > 0);
  assert.ok(matchScore("weixin", t) > 0);
  assert.ok(matchScore("wei", t) > 0);
});

test("subsequence fuzzy match needs >= 2 chars", () => {
  assert.ok(matchScore("chrm", { name: "Google Chrome" }) > 0);
  assert.equal(matchScore("zz", { name: "Google Chrome" }), 0);
});

test("no match returns 0", () => {
  assert.equal(matchScore("xyz", { name: "Safari" }), 0);
  assert.equal(matchScore("", { name: "Safari" }), 0);
});
