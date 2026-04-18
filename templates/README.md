# `templates/`

Templates consumed (or referenced) by the generator crates.

## Layout

```
templates/
  skill/
    SKILL.md.template      # reference shape for SKILL.md output
    README.md
  package/
    Cargo.toml.template    # reference shape for generated Cargo.toml
    src/
      main.rs.template     # reference shape for generated src/main.rs
```

## Status

All template files in this directory are currently **references**, not
live inputs: `cli-anything-generator` and `cli-anything-skillgen` emit
their output programmatically via `format!` / string builders. Keeping
the intended output shape in a single file lets humans diff-review
format changes without reading through the renderer.

When either renderer grows a real template engine (tera, handlebars,
askama), point it at this directory and delete the redundant
programmatic renderers in one focused PR.

## Conventions

* Placeholders use `{{name}}` / `{{#each ...}}` conventions so the
  templates read the same regardless of which engine is eventually
  wired in.
* Every placeholder used in a template must be documented in the
  header comment of that file.
* Keep templates in sync with the curated packages
  (`packages/{gimp,blender,drawio}/`). If the shape diverges, either
  the template or the curated package is wrong; don't let them drift.
