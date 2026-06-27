# Release URL Registry Intake Design

## Goal

Let plugin authors publish with the least ceremony possible: build a `.pcp`, attach it to a GitHub Release, and give us the asset URL. pc-tool and our registry tooling derive everything else from the package.

## Non-Goals

- No plugin marketplace backend.
- No author accounts.
- No author-written registry JSON.
- No manual sha256 calculation by authors.
- No signing in this step.

## Author Flow

1. Author builds `plugin.pcp`.
2. Author uploads it to a GitHub Release.
3. Author submits only the `.pcp` Release asset URL.

Example:

```txt
https://github.com/author/repo/releases/download/v1.0.0/plugin.pcp
```

## Maintainer Flow

1. Paste the submitted URL into a small registry intake command.
2. The command downloads the `.pcp`.
3. The command reads `plugin.json` from the package.
4. The command computes `sha256`.
5. The command appends or updates the official `registry.json` entry.
6. Maintainer reviews the generated diff and commits it.

Generated entry:

```json
{
  "id": "com.author.plugin",
  "name": "Plugin Name",
  "version": "1.0.0",
  "description": "Short description from plugin.json",
  "permissions": ["clipboard.read"],
  "packageUrl": "https://github.com/author/repo/releases/download/v1.0.0/plugin.pcp",
  "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
}
```

## Validation

The intake command rejects:

- non-HTTP URLs
- URLs that do not download
- files that are not valid `.pcp` packages
- packages without valid `plugin.json`
- duplicate `id + version` entries unless explicitly replacing the same URL

pc-tool still verifies `sha256` during install. GitHub Release assets can change, so the hash remains the real integrity check.

## Storage

Use the existing static registry format. The only change is how entries are produced: generated from a submitted package URL instead of hand-written by the author.

## Testing

Add one small fixture `.pcp` or generated test package. Verify the intake command:

- extracts metadata from `plugin.json`
- computes a 64-character sha256
- rejects an invalid URL or invalid package
- writes a valid registry entry

## Next Step

Build the intake command first. After that, add one real sample plugin release and make pc-tool include the official registry URL by default.
