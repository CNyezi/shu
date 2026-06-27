# Release URL Registry Intake Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a tiny registry intake command that turns a `.pcp` Release asset URL into a generated static registry entry.

**Architecture:** Keep this outside the app runtime as a Node script under `scripts/`. The script downloads or reads a `.pcp`, extracts `plugin.json`, computes sha256, validates the URL/package, and writes or updates `registry.json`.

**Tech Stack:** Node ESM, Node stdlib, existing `node:test`, system `zip`/`unzip` for fixture packages.

---

### Task 1: Intake Script

**Files:**
- Create: `scripts/registry-intake.mjs`
- Create: `scripts/registry-intake.test.mjs`
- Modify: `package.json`

- [x] **Step 1: Write the failing test**

Create `scripts/registry-intake.test.mjs` with tests that build a tiny `.pcp`, run the intake helper, and verify one generated registry entry plus rejection of invalid URL/package inputs.

- [x] **Step 2: Run test to verify it fails**

Run: `node --test scripts/registry-intake.test.mjs`

Expected: fail because `scripts/registry-intake.mjs` does not exist.

- [x] **Step 3: Write minimal implementation**

Create `scripts/registry-intake.mjs` exporting `createRegistryEntry`, `readPackageManifest`, and `updateRegistry`. Provide a CLI:

```bash
node scripts/registry-intake.mjs <package-url-or-file> <registry-json>
```

The CLI should write the generated entry into the registry JSON file.

- [x] **Step 4: Wire npm scripts**

Add:

```json
"registry:intake": "node scripts/registry-intake.mjs",
"test:registry-intake": "node --test scripts/registry-intake.test.mjs"
```

and include `pnpm test:registry-intake` in `pnpm test`.

- [x] **Step 5: Verify**

Run:

```bash
pnpm test
pnpm check
```

Expected: all pass.

- [x] **Step 6: Commit**

```bash
git add package.json scripts/registry-intake.mjs scripts/registry-intake.test.mjs docs/superpowers/plans/2026-06-27-release-url-registry-intake.md
git commit -m "feat: add registry intake command"
```

## Self-Review

- Spec coverage: author submits only a Release asset URL; tooling derives manifest metadata and sha256; registry format stays unchanged.
- Scope: no backend, no accounts, no signing, no marketplace.
- Test coverage: generated package path, invalid URL, invalid package, registry write/update.
