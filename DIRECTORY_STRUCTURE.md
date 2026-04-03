# CLI-Anything-RS Directory Structure

## Naming Decision

- Use `packages/` as the unified directory for software-specific Rust packages.
- Do not use `harnesses/` as the top-level directory name.
- Keep `crates/` for shared framework capabilities and `packages/` for concrete software packages such as GIMP, Blender, and Draw.io.

## Proposed Repository Layout

```text
CLI-Anything-RS/
├── Cargo.toml
├── .gitignore
├── crates/
│   ├── cli-anything-cli/
│   ├── cli-anything-core/
│   ├── cli-anything-manifest/
│   ├── cli-anything-generator/
│   ├── cli-anything-repl/
│   ├── cli-anything-skillgen/
│   ├── cli-anything-project/
│   └── cli-anything-integrations/
├── packages/
│   ├── gimp/
│   │   ├── Cargo.toml
│   │   ├── cli-anything.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── commands/
│   │   │   └── backend/
│   │   ├── tests/
│   │   ├── skills/
│   │   │   └── SKILL.md
│   │   └── fixtures/
│   ├── blender/
│   └── drawio/
├── templates/
│   ├── rust-cli/
│   ├── skill/
│   └── tests/
├── fixtures/
│   ├── manifests/
│   ├── snapshots/
│   └── sample-inputs/
├── scripts/
└── docs/
```

## Layering Rules

### `crates/`

Shared Rust infrastructure for the whole system:

- `cli-anything-cli`: top-level command entry such as init, build, refine, test, validate, list
- `cli-anything-core`: shared domain models, error types, serialization contracts
- `cli-anything-manifest`: schema and validation for `cli-anything.toml`
- `cli-anything-generator`: package generation logic
- `cli-anything-repl`: shared REPL rendering and interaction model
- `cli-anything-skillgen`: SKILL.md generation
- `cli-anything-project`: project state, layout helpers, artifact management
- `cli-anything-integrations`: Claude/OpenCode/Codex/Hub related integration output

### `packages/`

Software-specific Rust packages generated or maintained by CLI-Anything-RS:

- one directory per software target
- each package owns its commands, backend bridge, tests, fixtures, and generated skill file
- package-local code should only contain software-specific logic
- reusable logic must be moved back into `crates/`

## Mapping from CLI-Anything

Python CLI-Anything commonly uses:

```text
<software>/agent-harness/
```

Rust CLI-Anything-RS maps that idea into:

```text
packages/<software>/
```

This keeps the repository compact while making the Rust workspace easier to scale and maintain.

## Practical Rules

- Add new shared abstractions under `crates/`, not under `packages/`
- Add a new software integration under `packages/<software>/`
- Keep `templates/` language- and output-oriented, not software-oriented
- Keep `fixtures/` for generator testing and snapshot verification
- Treat `packages/` as product packages, not as internal framework crates
