---
name: cli-anything-gimp
description: Raster image processing via gimp -i -b (batch mode)
---

# cli-anything-gimp

cli-anything-gimp exposes a stateful Rust CLI workflow for gimp.

## Installation

This CLI is installed as part of the `cli-anything-gimp` package.

```bash
cargo install --path packages/gimp
```

**Prerequisites:**

- gimp must be installed on your system

- Install gimp: `gimp (apt install gimp)`

## Usage

### Basic Commands

```bash
cli-anything-gimp --help
cli-anything-gimp
cli-anything-gimp --json
```

### REPL Mode

Invoke `cli-anything-gimp` without a subcommand to enter an interactive session.

## Command Groups

### project

Project management commands

| Command | Description |
|---------|-------------|

| `new` | Create a new image project |

| `info` | Show project information |



### layer

Layer management commands

| Command | Description |
|---------|-------------|

| `new` | Create a blank layer |

| `list` | List project layers |



### canvas

Canvas inspection and resize commands

| Command | Description |
|---------|-------------|

| `info` | Show canvas metadata |

| `resize` | Resize the canvas |



### filter

Filter application commands

| Command | Description |
|---------|-------------|

| `add` | Apply a filter to a layer |

| `list` | List supported filters |



### media

Media import and asset inspection commands

| Command | Description |
|---------|-------------|

| `import` | Import media into the project |

| `list` | List project media |



### export

Export commands

| Command | Description |
|---------|-------------|

| `image` | Export the current composition |

| `presets` | List export presets |



### session

Session history and persistence commands

| Command | Description |
|---------|-------------|

| `status` | Show session status |

| `undo` | Undo the last action |



### draw

Drawing primitive commands

| Command | Description |
|---------|-------------|

| `line` | Draw a line |

| `rectangle` | Draw a rectangle |



## Examples

### Create poster

Create a new raster project for poster work.

```bash
cli-anything-gimp project new --width 1920 --height 1080 -o poster.json
```

### Add filter

Apply a brightness filter to the active layer.

```bash
cli-anything-gimp filter add brightness --layer 0 --param factor=1.3
```

## State Management

- Undo/redo friendly command execution

- Project persistence through state files

- Session tracking for modified buffers

## Output Formats

- Human-readable output for operators

- Machine-readable JSON output for agents

## For AI Agents

1. Prefer `cli-anything-gimp --json` when structured output is available

2. Check exit codes before reading generated files

3. Use absolute paths for package and fixture operations

## Version

1.0.0
