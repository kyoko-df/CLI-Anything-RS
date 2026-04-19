# AGENTS.md

Repository-level guide for agents (Claude, OpenCode, Codex, Devin, etc.)
working in **CLI-Anything-RS**. Prefer this file over `README.md` when
planning edits: it is intentionally terse and oriented around the
invariants you must not break.

## What this repo is

CLI-Anything-RS is a Rust-first harness generator. From a
`cli-anything.toml` manifest it produces a stateful, REPL-capable CLI
crate (`packages/<software>/`) that delegates heavy work to an upstream
GUI through a well-defined subprocess seam (`gimp -i -b`,
`blender --background --python`, `draw.io -x --format ...`).

The cargo workspace is organised as:

```
crates/
  cli-anything-cli           # `cli-anything` binary (init/build/test/...)
  cli-anything-core          # CommandResponse, PackageSummary, manifest re-exports
  cli-anything-generator     # package scaffolding from a manifest
  cli-anything-integrations  # CLAUDE.md / opencode.md / codex.yaml / hub.md renderers
  cli-anything-manifest      # TOML schema + curated specs (gimp/blender/drawio)
  cli-anything-project       # ProjectState (undo/redo/save) + Backend trait
  cli-anything-repl          # Skin formatter + read-dispatch-render REPL loop
  cli-anything-skillgen      # SKILL.md renderer driven by the manifest
packages/
  gimp/                      # curated harness for GIMP
  blender/                   # curated harness for Blender
  drawio/                    # curated harness for draw.io / diagrams.net
templates/                   # file templates consumed by the generator
fixtures/                    # small assets used by examples / smoke docs
```

The three curated packages are **both** end-user products and
integration tests for the generator: the generator must be able to
reproduce them, and their smoke tests must stay green.

## Invariants you must not break

1. **Default execution is hermetic.** Package binaries must default to
   the dry-run backend. Real subprocess execution only happens when
   `CLI_ANYTHING_BACKEND=system` is set. Never call `gimp`, `blender`,
   `draw.io`, etc. directly; always go through `cli_anything_project::
   backend::{Backend, DryRunBackend, SystemBackend, backend_from_env}`.

2. **State files are local to the caller's CWD.** Packages write
   `.{software}-cli.json` in the current working directory. Tests
   override the path via `CLI_ANYTHING_STATE_FILE`. Do not introduce a
   global/XDG state location without an explicit opt-in.

3. **Command responses are typed.** Every command returns a
   `cli_anything_core::CommandResponse` constructed with
   `CommandResponse::new(...).with_details(details)`. Details live in
   `ResponseDetails` (a `BTreeMap<String, serde_json::Value>`). Do not
   re-introduce ad-hoc `serde_json::Value` returns.

4. **Markdown tables are atomic blocks.** When rendering SKILL.md or
   similar artefacts, keep each table (header + separator + body) as a
   single string. Splitting it across `join("\n\n")` boundaries has
   bitten us before; there is a regression test
   (`command_group_tables_are_not_broken_by_blank_lines`) that will
   catch it again.

5. **Generated packages never edit themselves.** The generator owns
   every file under `packages/<software>/` that it writes. Agents and
   humans can hand-edit curated packages, but keep the shape compatible
   with what `cli-anything build` regenerates (same module layout, same
   command-group names).

## Day-to-day workflow

### Lint / test / build

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

CI runs exactly those three steps in `.github/workflows/ci.yml`. If any
of them fail locally, they will fail in CI.

### Regenerate the curated SKILL.md files

After touching `cli-anything-skillgen`:

```bash
cargo run --example regen_skills -p cli-anything-skillgen
```

This overwrites `packages/{gimp,blender,drawio}/skills/SKILL.md` from
each package's `cli-anything.toml`. Commit the regenerated files
alongside the code change.

### Emit integration docs for a curated package

```bash
cargo run -p cli-anything -- build ./gimp --emit-integrations
ls packages/gimp/integrations/
# CLAUDE.md  codex.yaml  hub.md  opencode.md
```

### Drive the real GUI

Set `CLI_ANYTHING_BACKEND=system` on a package invocation:

```bash
CLI_ANYTHING_BACKEND=system cli-anything-gimp project new --name demo
```

Without the flag you will get `backend: dry-run` in the response
details and no subprocess will start. With the flag you get
`backend: system` and an actual `gimp -i -b …` child process.

## Where things live (pointers, not full tours)

* Manifest schema and curated specs:
  `crates/cli-anything-manifest/src/lib.rs`.
* Generator entrypoint: `cli_anything_generator::generate_package` in
  `crates/cli-anything-generator/src/lib.rs`. The `cli-anything build`
  subcommand calls this.
* Project state (`ProjectState`, `ActionRecord`, load/save helpers):
  `crates/cli-anything-project/src/lib.rs`.
* Backend adapter (`Backend`, `DryRunBackend`, `SystemBackend`,
  `backend_from_env`, `ensure_success`, `BACKEND_MODE_ENV`):
  `crates/cli-anything-project/src/backend.rs`.
* REPL loop (`Repl`, `DispatchOutcome`, `tokenize`, `Skin`):
  `crates/cli-anything-repl/src/lib.rs`.
* SKILL.md renderer and regen example:
  `crates/cli-anything-skillgen/src/lib.rs` and
  `crates/cli-anything-skillgen/examples/regen_skills.rs`.
* Integration renderers:
  `crates/cli-anything-integrations/src/lib.rs`.

## Conventions for agents

* **Surface the subprocess.** When wiring a new command to the real
  GUI, build a `BackendInvocation` explicitly, pass it through the
  configured `Backend`, and include the resolved
  `backend: "dry-run" | "system"` and the `command` string in the
  response's `ResponseDetails`. This keeps logs and traces grep-able.
* **Do not swallow errors from `save_state`.** Any command that mutates
  `ProjectState` must call `save_state` and propagate failures; the
  smoke tests assert this.
* **Prefer `BTreeMap`-ordered details.** `ResponseDetails` is a
  `BTreeMap`, so JSON output is deterministic. Keep key names in
  `snake_case`.
* **Keep human output readable.** `print_response` in each package
  binary is the canonical place to format the non-JSON output. Agents
  consuming the tool should pass `--json`; humans get the decorated
  banner.

## Testing notes

* Smoke tests for the curated packages live under
  `packages/<software>/tests/smoke.rs`. They set
  `CLI_ANYTHING_STATE_FILE` to a temp path, invoke the package binary,
  and assert on both the JSON response and the on-disk state.
* The REPL has unit tests covering the tokenizer (quoted segments,
  unknown-escape preservation, unterminated-quote rejection) and the
  dispatch loop (Rendered / Failed / Exit outcomes).
* The backend module's tests use `/usr/bin/true` so they run on any
  POSIX CI image without extra fixtures.

## When in doubt

* Read `README.md` for the human-facing tour.
* Read `DIRECTORY_STRUCTURE.md` for the intended final layout.
* When adding a feature, ask: "does this belong in the generator, or
  in a curated package?" Generator-level changes must regenerate all
  three curated packages cleanly; curated-only changes can afford to
  diverge temporarily but should be reconciled with the generator in
  the same PR or tracked in an issue.
