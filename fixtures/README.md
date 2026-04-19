# `fixtures/`

Small, reviewable sample assets referenced by docs, examples, and
smoke tests. Everything here is plain text (or small enough to audit
at a glance). Keep binaries out of this tree — if a test needs a real
image or blend file, generate it at test time.

## Layout

```
fixtures/
  README.md                 # this file
  manifests/
    gimp.toml               # copy of the curated gimp manifest
    blender.toml            # copy of the curated blender manifest
    drawio.toml             # copy of the curated drawio manifest
```

The per-package fixtures are colocated with their package crate so
they can be found relative to `CARGO_MANIFEST_DIR`:

```
packages/gimp/fixtures/poster.script-fu       # script-fu batch snippet
packages/blender/fixtures/render_cube.py      # blender --python sample
packages/drawio/fixtures/sample.drawio.xml    # minimal drawio document
```

## Guidelines for adding fixtures

* Prefer text fixtures (TOML / JSON / XML / Python / script-fu). They
  diff cleanly and stay under 4 KiB.
* Never commit user-identifying metadata (author, email, machine-
  specific paths). Fixtures must be reproducible.
* If you need a real binary asset (PNG, .blend, `.xcf`) for a smoke
  test, generate it with `tempfile` inside the test rather than
  committing a binary to the repo.
* Every fixture must be referenced either from a test, a doc example,
  or an integration renderer. Dead fixtures get deleted.
