import assert from "node:assert/strict";
import test from "node:test";
import { bump, score } from "../src/lib/frecency.ts";

const DAY = 24 * 3600 * 1000;

test("bump increments count and sets last-used", () => {
  const r1 = bump(undefined, 1000);
  assert.deepEqual(r1, { n: 1, t: 1000 });
  const r2 = bump(r1, 2000);
  assert.deepEqual(r2, { n: 2, t: 2000 });
});

test("score decays with a 7-day half-life", () => {
  const rec = { n: 4, t: 0 };
  assert.equal(score(rec, 0), 4);
  assert.ok(Math.abs(score(rec, 7 * DAY) - 2) < 1e-9);
  assert.equal(score(undefined, 0), 0);
});

test("more recent use outranks stale heavy use", () => {
  const stale = { n: 10, t: 0 };
  const fresh = { n: 2, t: 60 * DAY };
  assert.ok(score(fresh, 60 * DAY) > score(stale, 60 * DAY));
});
