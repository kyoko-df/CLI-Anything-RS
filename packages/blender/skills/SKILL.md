---
name: cli-anything-blender
description: 3D modeling, animation, and rendering via blender --background --python
---

# cli-anything-blender

cli-anything-blender exposes a stateful Rust CLI workflow for blender.

## Installation

This CLI is installed as part of the `cli-anything-blender` package.

```bash
cargo install --path packages/blender
```

**Prerequisites:**

- blender must be installed on your system
- Install blender: `blender`

## Usage

### Basic Commands

```bash
cli-anything-blender --help
cli-anything-blender
cli-anything-blender --json
```

### REPL Mode

Invoke `cli-anything-blender` without a subcommand to enter an interactive session.

## Command Groups

### scene

Scene management commands

| Command | Description |
|---------|-------------|
| `new` | Create a new scene |
| `info` | Inspect the active scene |

### object

Object creation and transformation commands

| Command | Description |
|---------|-------------|
| `add` | Add a new object |
| `list` | List scene objects |

### material

Material authoring commands

| Command | Description |
|---------|-------------|
| `assign` | Assign a material |
| `list` | List materials |

### modifier

Modifier stack commands

| Command | Description |
|---------|-------------|
| `add` | Add a modifier |
| `apply` | Apply a modifier |

### camera

Camera rig commands

| Command | Description |
|---------|-------------|
| `add` | Add a camera |
| `list` | List cameras |

### light

Lighting commands

| Command | Description |
|---------|-------------|
| `add` | Add a light |
| `list` | List lights |

### animation

Animation timeline commands

| Command | Description |
|---------|-------------|
| `keyframe` | Insert a keyframe |
| `playblast` | Preview the animation |

### render

Rendering commands

| Command | Description |
|---------|-------------|
| `frame` | Render a frame |
| `info` | Inspect render settings |

### session

Session tracking commands

| Command | Description |
|---------|-------------|
| `status` | Show session state |
| `history` | Inspect action history |

## Examples

### Create scene

Create a fresh Blender scene file.

```bash
cli-anything-blender scene new -o demo.blend
```

### Render frame

Render the active frame to a PNG file.

```bash
cli-anything-blender render frame --output frame.png
```

## State Management

- Undo/redo friendly command execution
- Project persistence through state files
- Session tracking for modified buffers

## Output Formats

- Human-readable output for operators
- Machine-readable JSON output for agents

## For AI Agents

1. Prefer `cli-anything-blender --json` when structured output is available
2. Check exit codes before reading generated files
3. Use absolute paths for package and fixture operations

## Version

1.0.0
