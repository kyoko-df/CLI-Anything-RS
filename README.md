# CLI-Anything-RS

Rust-first harness generator and package workflow for creating stateful CLI
interfaces for GUI applications. The workspace produces one reusable
framework under `crates/` and one generated package per supported software
under `packages/`.

## Layout

```
crates/
  cli-anything-cli           # user-facing `cli-anything` binary
  cli-anything-core          # shared domain model, validation, summaries
  cli-anything-generator     # code generation for packages/<software>/
  cli-anything-integrations  # Claude / OpenCode / Codex / Hub adapters
  cli-anything-manifest      # cli-anything.toml schema and curated specs
  cli-anything-project       # project state (.*-cli.json) + undo/redo
  cli-anything-repl          # terminal skin and REPL primitives
  cli-anything-skillgen      # SKILL.md rendering from a manifest
packages/
  blender/ drawio/ gimp/     # generated CLI packages
```

## Commands

`cli-anything` is the single entry point:

| Command | Purpose |
|---------|---------|
| `cli-anything` | Print the banner and list available subcommands. |
| `cli-anything init [--path <dir>]` | Scaffold a new workspace with `crates/`, `packages/`, and a root `Cargo.toml`. |
| `cli-anything build <source>` | Generate a Rust package under `packages/<software>/` from a local path or GitHub repository. |
| `cli-anything refine <source> [focus]` | Audit a generated package and plan gap-closing work. |
| `cli-anything test <source>` | Emit the `cargo test` / `cargo run -- --help` plan for the package. |
| `cli-anything validate <source>` | Check that the generated package has the expected layout and manifest. |
| `cli-anything list [--path <dir>]` | Discover generated packages by walking for `cli-anything.toml`. |

All commands accept `--json` for machine-readable output.

## Getting started

```bash
# Scaffold a workspace in the current directory
cli-anything init

# Generate a package for GIMP
cli-anything build ./gimp

# Inspect what we generated
cli-anything list
cli-anything validate ./gimp
```

Requirements:

- Rust 1.85+ (the workspace uses `edition = "2024"`).
- Cargo 1.85+.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The same checks run in CI on every pull request (see
`.github/workflows/ci.yml`).

## License

MIT
