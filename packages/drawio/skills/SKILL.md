---
name: cli-anything-drawio
description: Diagram creation and export via draw.io CLI
---

# cli-anything-drawio

cli-anything-drawio exposes a stateful Rust CLI workflow for drawio.

## Installation

This CLI is installed as part of the `cli-anything-drawio` package.

```bash
cargo install --path packages/drawio
```

**Prerequisites:**

- drawio must be installed on your system
- Install drawio: `draw.io desktop app`

## Usage

### Basic Commands

```bash
cli-anything-drawio --help
cli-anything-drawio
cli-anything-drawio --json
```

### REPL Mode

Invoke `cli-anything-drawio` without a subcommand to enter an interactive session.

## Command Groups

### diagram

Diagram project commands

| Command | Description |
|---------|-------------|
| `new` | Create a new diagram |
| `info` | Inspect the active diagram |

### page

Page management commands

| Command | Description |
|---------|-------------|
| `add` | Add a page |
| `list` | List pages |

### shape

Shape authoring commands

| Command | Description |
|---------|-------------|
| `add` | Add a shape |
| `list` | List shapes |

### connection

Connection authoring commands

| Command | Description |
|---------|-------------|
| `add` | Connect two shapes |
| `list` | List connections |

### style

Style management commands

| Command | Description |
|---------|-------------|
| `apply` | Apply a style |
| `list` | List available styles |

### export

Export commands

| Command | Description |
|---------|-------------|
| `svg` | Export diagram as SVG |
| `png` | Export diagram as PNG |
| `pdf` | Export diagram as PDF |

### session

Session management commands

| Command | Description |
|---------|-------------|
| `status` | Show current session |
| `undo` | Undo the last action |
| `redo` | Redo the last undone action |
| `history` | List recorded actions |
| `save` | Mark the session clean |

## Examples

### Create diagram

Create a new diagram session.

```bash
cli-anything-drawio diagram new --name architecture --template flowchart
```

### Add rectangle

Add a rectangle shape to the current page.

```bash
cli-anything-drawio shape add --kind rectangle --x 120 --y 80
```

## State Management

- Undo/redo friendly command execution
- Project persistence through state files
- Session tracking for modified buffers

## Output Formats

- Human-readable output for operators
- Machine-readable JSON output for agents

## For AI Agents

1. Prefer `cli-anything-drawio --json` when structured output is available
2. Check exit codes before reading generated files
3. Use absolute paths for package and fixture operations

## Version

1.0.0
