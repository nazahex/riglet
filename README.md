<div align="center">

# Riglet

![Go](https://img.shields.io/badge/Go-%2300ADD8.svg?logo=go&logoColor=white)
[![Bun](https://img.shields.io/badge/Bun-%23000000.svg?logo=bun&logoColor=white)](https://bun.sh)<br />
![Conventional Commits](https://img.shields.io/badge/commit-conventional-blue.svg)
[![Commitizen friendly](https://img.shields.io/badge/commitizen-friendly-brightgreen.svg)](http://commitizen.github.io/cz-cli/)<br />
[![Turborepo](https://img.shields.io/badge/-Turborepo-EF4444?logo=turborepo&logoColor=white)](https://turbo.build)
[![Changesets](https://img.shields.io/badge/Changesets-🦋-white)](./CHANGELOG.md)
[![Biome Linter & Formatted](https://img.shields.io/badge/Biome-60a5fa?style=flat&logo=biome&logoColor=white)](https://biomejs.dev/)

</div>

Riglet is a developer-experience toolkit designed to unify setup, linting, formatting, syncing, and automation for monorepos. It ensures deterministic, fast, and straightforward configuration enforcement.

## Key Features

- **CUE-based schemas**: Provides strict validation for configurations like `package.json` with support for custom schemas.
- **Deterministic formatting**: Ensures canonical key ordering, section-aware blank lines, and stable JSON emission.
- **Scoped execution**: Allows operations to be limited to the repo root, workspaces, or all files.
- **Ignore support**: Respects `.rigletignore` and sensible defaults like `node_modules/**`.
- **Template sync**: Applies repo- and package-level templates with token substitution.
- **CI-friendly checks**: Combines format checks and linting for streamlined CI workflows.

## Usage

Riglet provides several commands to manage and enforce configurations:

- `riglet lint`: Validate repository configurations.
- `riglet format`: Apply deterministic formatting to supported files.
- `riglet sync`: Synchronize templates and apply formatting.
- `riglet check`: Run format checks and linting together.

For detailed usage instructions, refer to the [apps/riglet/README.md](apps/riglet/README.md).

## License

MIT © KazViz
