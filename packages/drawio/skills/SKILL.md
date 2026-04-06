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

### project

Diagram project commands

| Command | Description |
|---------|-------------|

| `new` | Create a new diagram |

| `info` | Show project metadata |



### shape

Shape creation commands

| Command | Description |
|---------|-------------|

| `add` | Add a shape |

| `types` | List shape types |



### connect

Connector authoring commands

| Command | Description |
|---------|-------------|

| `add` | Create a connector |

| `styles` | List connector styles |



### page

Page management commands

| Command | Description |
|---------|-------------|

| `add` | Add a page |

| `list` | List pages |



### export

Export commands

| Command | Description |
|---------|-------------|

| `diagram` | Export a diagram |

| `formats` | List formats |



### session

Session management commands

| Command | Description |
|---------|-------------|

| `status` | Show current session |

| `save` | Persist session state |



## Examples

### Create diagram

Create a new diagram file.

```bash
cli-anything-drawio project new -o architecture.drawio
```

### Add rectangle

Add a rectangle shape to the current page.

```bash
cli-anything-drawio shape add rectangle --text API
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
