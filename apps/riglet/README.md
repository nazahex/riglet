# Riglet

Riglet is a customizable, CUE-powered toolkit for formatting, linting, and synchronizing configuration files across monorepos. It brings order to sprawling repos by enforcing consistent structure, deterministic formatting, and template policies—without friction.

## Why Riglet Exists

Large repositories accumulate configuration entropy. Keys drift out of order, schemas diverge, templates go stale, and CI checks become unreliable. Riglet attacks that chaos head-on: it validates configs with CUE, formats them in a predictable way, and keeps templates in sync—so your repo stays sharp, coherent, and boring-in-the-best-way.

## Key Capabilities

### CUE-Driven Validation

- Enforce strict, type-safe schemas for configs like `package.json`.
- Plug in custom schemas to match your organizational standards.
- Ensure every workspace speaks the same structural language.

### Deterministic, Human-Friendly Formatting

- Canonical key ordering and logical sectioning for JSON files.
- Stable formatting output, even across machines and CI.
- Smart rules: alphabetical sorting for deps, preserved order for scripts, and predictable object-level ordering.

### Scoped Operations

- Run tools only where relevant:
  - **`repo`**: root + `packages/*`
  - **`workspace`**: resolved from `workspaces` or `pnpm-workspace.yaml`
  - **`all`**: full repo traversal with ignore support

### Ignore-Aware Engine

- Honors `.rigletignore` plus sensible defaults such as:
  - `node_modules/**`, `.git/**`, `.turbo/**`, etc.

### Template Synchronization

- Sync repo-level and package-level templates with flexible rules.
- Support for token substitution (`{{repo.name}}`, `{{package.name}}`).
- Optional overwrite control and selective application scopes.

### CI-Ready Checks

- A single command to verify formatting + linting.
- Fast and deterministic—perfect for pipelines.

## Project Status

Riglet’s MVP already includes:

- CLI commands: `lint`, `format`, `sync`, `check`
- Strict root + sub-package schemas for `package.json`
- Workspace discovery via `package.json#workspaces` or `pnpm-workspace.yaml`

## Installation & Build

```sh
cd ./apps/riglet
go build ./cmd/riglet
```

## CLI Commands Overview

### General

- `version`
  Displays Riglet’s current version.

- `validate --schema <cue> --input <json|yaml>`
  Validate any file against any CUE schema.

### Linting & Formatting

- `lint [--repo-root <path>] [--scope repo|workspace|all] [--config <riglet.yaml>]`
  Validate repo configs following conventions.

- `format [--repo-root <path>] [--scope repo|workspace|all] [--write]`
  Format supported configs deterministically (MVP: `package.json`).

### Template Sync & Checks

- `sync [--repo-root <path>] [--scope repo|workspace|all] [--dry-run] [--config <riglet.yaml>]`
  Apply templates based on rules, then auto-format.

- `check [--repo-root <path>] [--scope repo|workspace|all]`
  Run formatting in check-mode + lint; fails when issues are found.

## Scopes Explained

- **`repo`**
  Operates on the root `package.json` plus `packages/*/package.json`.

- **`workspace`**
  Resolves workspaces using `workspaces` in `package.json` or `pnpm-workspace.yaml`.

- **`all`**
  Recursively walks the repo, respecting ignore rules.

## Ignore Rules

### Default skipped paths

- `node_modules/**`
- `.git/**`
- `dist/**`
- `.turbo/**`
- `.artifacts/**`

### `.rigletignore`

- Optional file in repo root.
- Simple patterns: path prefixes or `**` globs.
- `#` indicates comments.

## Built-In Conventions

### `package.json` (root)

Requires:

- `private: true`
- `type: "module"`
- `workspaces` (array or object)

Optional:

- `packageManager`, `engines`, `repository`, `sideEffects`, `exports`

### `package.json` (sub-packages)

- Prohibits `workspaces`
- Requires `repository.directory`
- Allows `publishConfig` (`provenance: true` default)
- Supports `engines`, `sideEffects`, `exports`

## Formatting Rules for `package.json`

### Top-Level Structure

- Deterministic key ordering
- Clear blank lines between logical sections

### Nested Object Rules

- `scripts`: preserve order
- Dependency sections: alphabetical
- `engines`: alphabetical

### Custom Ordering for Known Subfields

- `author`: `name`, `email`, `url`
- `repository`: `type`, `url`, `directory`
- `publishConfig`: `access`, `provenance`

Unknown keys remain intact and are appended lexicographically.

## Workspace Discovery

Riglet searches in this order:

1. `package.json#workspaces` (array or `{ packages: [] }`)
2. `pnpm-workspace.yaml`
3. Fallback to `packages/*`

## Customizing Riglet

### 1. Custom Schemas (`riglet.yaml`)

```yaml
rules:
  - id: pkgjson
    patterns: ["package.json"]
    schema: apps/riglet/policies/pkgjson.mono.sub.cue
```

Fields:

- `id`: unique convention identifier
- `patterns`: basenames or globs
- `schema`: absolute or relative path to CUE schema

### **2. Ordering via CUE**

Define ordering once in policy, not code:

```cue
Order: {
  Top: [
    ["name","version","description","license","private","homepage","repository","bugs","author","keywords"],
    ["scripts","workspaces","dependencies","devDependencies","peerDependencies","optionalDependencies"],
    ["packageManager","engines","os","cpu"],
    ["bin","main","module","types","exports","files","sideEffects"],
    ["publishConfig"],
  ]
  Sub: {
    author: ["name","email","url"]
    repository: ["type","url","directory"]
    publishConfig: ["access","provenance"]
  }
}
```

Riglet applies `Order.Top` and `Order.Sub` when present; defaults are used otherwise.

### 3. Template Sync (`riglet.yaml#sync`)

```yaml
sync:
  - id: editorconfig
    source: conventions/hyperedge/templates/.editorconfig
    target: .editorconfig
    when: all
    overwrite: true
```

Parameters:

- `source`: path or glob under repo root
- `target`: destination relative path
- `when`: `root`, `packages`, or `all`
- `overwrite`: replace existing files
- Token support: `{{repo.name}}`, `{{package.name}}`

## Practical Examples

```sh
# Lint root + packages
./riglet lint --repo-root /home/kaizansultan/Project/nazahex/riglet-js --scope repo

# Format check (no write)
./riglet format --repo-root /home/kaizansultan/Project/nazahex/riglet-js --scope repo

# Actually write formatting changes
./riglet format --repo-root /home/kaizansultan/Project/nazahex/riglet-js --scope repo --write

# Template sync (dry-run)
./riglet sync --repo-root /home/kaizansultan/Project/nazahex/riglet-js --scope all --dry-run

# Template sync (apply)
./riglet sync --repo-root /home/kaizansultan/Project/nazahex/riglet-js --scope all

# CI check: format (check mode) then lint
./riglet check --repo-root /home/kaizansultan/Project/nazahex/riglet-js --scope repo
```

## Project Structure

- `cmd/riglet/` — CLI entrypoints
- `engine/` — CUE loader, walker, ignore engine
- `configs/` — conventions registry, adapters, `riglet.yaml` loader
- `configs/pkgjson/` — formatter for `package.json`
- `policies/` — built-in CUE schemas

## Extending Riglet

You get:

- Add new CUE schemas anywhere in the repo.
- Register them through `riglet.yaml#conventions`.
- Add templates under any directory and wire them via `sync` rules.

Riglet adapts to your architecture, not the other way around.

## Notes

- Riglet is schema-agnostic—bring any CUE schema you needs.
- MVP focuses on `package.json`, but adding new config types is straightforward via adapters.
- User-facing configuration lives in `riglet.yaml`.

## License

MIT © KazViz
