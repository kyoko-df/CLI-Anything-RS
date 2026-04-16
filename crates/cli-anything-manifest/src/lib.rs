//! Schema, parsing, and curated specs for `cli-anything.toml`.
//!
//! This crate owns the manifest domain model that describes a generated
//! package: backend binary, project format, command groups, and examples.
//! It is kept separate from `cli-anything-core` so that consumers who only
//! need to read or validate a manifest do not have to pull in the rest of
//! the framework surface.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliAnythingManifest {
    pub name: String,
    pub version: String,
    pub binary: String,
    pub description: String,
    #[serde(default)]
    pub repl_default: bool,
    #[serde(default)]
    pub supports_json: bool,
    pub backend: BackendConfig,
    pub project: ProjectConfig,
    pub skill: SkillConfig,
    #[serde(default)]
    pub command_groups: Vec<CommandGroup>,
    #[serde(default)]
    pub examples: Vec<ExampleSpec>,
}

impl CliAnythingManifest {
    pub fn package_name(&self) -> &str {
        &self.binary
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendConfig {
    pub command: String,
    pub system_package: String,
    #[serde(default = "default_true")]
    pub hard_dependency: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub format: String,
    pub state_file: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillConfig {
    pub output: String,
    pub template: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandGroup {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub commands: Vec<CommandSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSpec {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExampleSpec {
    pub title: String,
    pub description: String,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownPackageSpec {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub backend_command: String,
    pub system_package: String,
    pub project_format: String,
    pub state_file: String,
    pub command_groups: Vec<CommandGroup>,
    pub examples: Vec<ExampleSpec>,
}

pub fn parse_manifest(input: &str) -> Result<CliAnythingManifest> {
    toml::from_str::<CliAnythingManifest>(input).context("failed to parse cli-anything manifest")
}

pub fn load_manifest_from_path(path: &Path) -> Result<CliAnythingManifest> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read manifest at {}", path.display()))?;
    parse_manifest(&content)
}

pub fn builtin_package_specs() -> Vec<KnownPackageSpec> {
    vec![
        gimp_package_spec(),
        blender_package_spec(),
        drawio_package_spec(),
    ]
}

pub fn builtin_package_spec(name: &str) -> Option<KnownPackageSpec> {
    builtin_package_specs()
        .into_iter()
        .find(|spec| spec.name == name.trim().to_lowercase())
}

fn default_true() -> bool {
    true
}

fn gimp_package_spec() -> KnownPackageSpec {
    KnownPackageSpec {
        name: "gimp".to_string(),
        display_name: "GIMP".to_string(),
        description: "Raster image processing via gimp -i -b (batch mode)".to_string(),
        backend_command: "gimp".to_string(),
        system_package: "gimp (apt install gimp)".to_string(),
        project_format: "xcf".to_string(),
        state_file: ".gimp-cli.json".to_string(),
        command_groups: vec![
            command_group(
                "project",
                "Project management commands",
                &[
                    ("new", "Create a new image project"),
                    ("info", "Show project information"),
                ],
            ),
            command_group(
                "layer",
                "Layer management commands",
                &[
                    ("new", "Create a blank layer"),
                    ("list", "List project layers"),
                ],
            ),
            command_group(
                "canvas",
                "Canvas inspection and resize commands",
                &[
                    ("info", "Show canvas metadata"),
                    ("resize", "Resize the canvas"),
                ],
            ),
            command_group(
                "filter",
                "Filter application commands",
                &[
                    ("add", "Apply a filter to a layer"),
                    ("list", "List supported filters"),
                ],
            ),
            command_group(
                "media",
                "Media import and asset inspection commands",
                &[
                    ("import", "Import media into the project"),
                    ("list", "List project media"),
                ],
            ),
            command_group(
                "export",
                "Export commands",
                &[
                    ("image", "Export the current composition"),
                    ("presets", "List export presets"),
                ],
            ),
            command_group(
                "session",
                "Session history and persistence commands",
                &[
                    ("status", "Show session status"),
                    ("undo", "Undo the last action"),
                ],
            ),
            command_group(
                "draw",
                "Drawing primitive commands",
                &[("line", "Draw a line"), ("rectangle", "Draw a rectangle")],
            ),
        ],
        examples: vec![
            example(
                "Create poster",
                "Create a new raster project for poster work.",
                "cli-anything-gimp project new --width 1920 --height 1080 -o poster.json",
            ),
            example(
                "Add filter",
                "Apply a brightness filter to the active layer.",
                "cli-anything-gimp filter add brightness --layer 0 --param factor=1.3",
            ),
        ],
    }
}

fn blender_package_spec() -> KnownPackageSpec {
    KnownPackageSpec {
        name: "blender".to_string(),
        display_name: "Blender".to_string(),
        description: "3D modeling, animation, and rendering via blender --background --python"
            .to_string(),
        backend_command: "blender".to_string(),
        system_package: "blender".to_string(),
        project_format: "blend".to_string(),
        state_file: ".blender-cli.json".to_string(),
        command_groups: vec![
            command_group(
                "scene",
                "Scene management commands",
                &[
                    ("new", "Create a new scene"),
                    ("info", "Inspect the active scene"),
                ],
            ),
            command_group(
                "object",
                "Object creation and transformation commands",
                &[("add", "Add a new object"), ("list", "List scene objects")],
            ),
            command_group(
                "material",
                "Material authoring commands",
                &[("assign", "Assign a material"), ("list", "List materials")],
            ),
            command_group(
                "modifier",
                "Modifier stack commands",
                &[("add", "Add a modifier"), ("apply", "Apply a modifier")],
            ),
            command_group(
                "camera",
                "Camera rig commands",
                &[("add", "Add a camera"), ("list", "List cameras")],
            ),
            command_group(
                "light",
                "Lighting commands",
                &[("add", "Add a light"), ("list", "List lights")],
            ),
            command_group(
                "animation",
                "Animation timeline commands",
                &[
                    ("keyframe", "Insert a keyframe"),
                    ("playblast", "Preview the animation"),
                ],
            ),
            command_group(
                "render",
                "Rendering commands",
                &[
                    ("frame", "Render a frame"),
                    ("info", "Inspect render settings"),
                ],
            ),
            command_group(
                "session",
                "Session tracking commands",
                &[
                    ("status", "Show session state"),
                    ("history", "Inspect action history"),
                ],
            ),
        ],
        examples: vec![
            example(
                "Create scene",
                "Create a fresh Blender scene file.",
                "cli-anything-blender scene new -o demo.blend",
            ),
            example(
                "Render frame",
                "Render the active frame to a PNG file.",
                "cli-anything-blender render frame --output frame.png",
            ),
        ],
    }
}

fn drawio_package_spec() -> KnownPackageSpec {
    KnownPackageSpec {
        name: "drawio".to_string(),
        display_name: "Draw.io".to_string(),
        description: "Diagram creation and export via draw.io CLI".to_string(),
        backend_command: "draw.io".to_string(),
        system_package: "draw.io desktop app".to_string(),
        project_format: "drawio".to_string(),
        state_file: ".drawio-cli.json".to_string(),
        command_groups: vec![
            command_group(
                "project",
                "Diagram project commands",
                &[
                    ("new", "Create a new diagram"),
                    ("info", "Show project metadata"),
                ],
            ),
            command_group(
                "shape",
                "Shape creation commands",
                &[("add", "Add a shape"), ("types", "List shape types")],
            ),
            command_group(
                "connect",
                "Connector authoring commands",
                &[
                    ("add", "Create a connector"),
                    ("styles", "List connector styles"),
                ],
            ),
            command_group(
                "page",
                "Page management commands",
                &[("add", "Add a page"), ("list", "List pages")],
            ),
            command_group(
                "export",
                "Export commands",
                &[("diagram", "Export a diagram"), ("formats", "List formats")],
            ),
            command_group(
                "session",
                "Session management commands",
                &[
                    ("status", "Show current session"),
                    ("save", "Persist session state"),
                ],
            ),
        ],
        examples: vec![
            example(
                "Create diagram",
                "Create a new diagram file.",
                "cli-anything-drawio project new -o architecture.drawio",
            ),
            example(
                "Add rectangle",
                "Add a rectangle shape to the current page.",
                "cli-anything-drawio shape add rectangle --text API",
            ),
        ],
    }
}

fn command_group(name: &str, description: &str, commands: &[(&str, &str)]) -> CommandGroup {
    CommandGroup {
        name: name.to_string(),
        description: description.to_string(),
        commands: commands
            .iter()
            .map(|(name, description)| CommandSpec {
                name: (*name).to_string(),
                description: (*description).to_string(),
            })
            .collect(),
    }
}

fn example(title: &str, description: &str, code: &str) -> ExampleSpec {
    ExampleSpec {
        title: title.to_string(),
        description: description.to_string(),
        code: code.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_MANIFEST: &str = r#"
name = "shotcut"
version = "1.0.0"
binary = "cli-anything-shotcut"
description = "Rust CLI harness for Shotcut"
repl_default = true
supports_json = true

[backend]
command = "melt"
system_package = "melt ffmpeg"
hard_dependency = true

[project]
format = "mlt"
state_file = ".shotcut-cli.json"

[skill]
output = "packages/shotcut/skills/SKILL.md"
template = "templates/skill/SKILL.md.template"

[[command_groups]]
name = "project"
description = "Project lifecycle commands"

[[command_groups.commands]]
name = "new"
description = "Create a new project"

[[examples]]
title = "Create project"
description = "Create a new Shotcut project"
code = "cli-anything-shotcut project new -o demo.mlt"
"#;

    #[test]
    fn parses_manifest_with_nested_sections() {
        let manifest = parse_manifest(SAMPLE_MANIFEST).expect("manifest should parse");

        assert_eq!(manifest.name, "shotcut");
        assert_eq!(manifest.backend.command, "melt");
        assert_eq!(manifest.command_groups.len(), 1);
        assert_eq!(manifest.command_groups[0].commands[0].name, "new");
        assert_eq!(manifest.examples.len(), 1);
    }

    #[test]
    fn defaults_hard_dependency_to_true_when_omitted() {
        let input = r#"
name = "inkscape"
version = "1.0.0"
binary = "cli-anything-inkscape"
description = "Rust CLI harness for Inkscape"
repl_default = true
supports_json = true

[backend]
command = "inkscape"
system_package = "inkscape"

[project]
format = "svg"
state_file = ".inkscape-cli.json"

[skill]
output = "packages/inkscape/skills/SKILL.md"
template = "templates/skill/SKILL.md.template"
"#;

        let manifest = parse_manifest(input).expect("manifest should parse");

        assert!(manifest.backend.hard_dependency);
    }

    #[test]
    fn rejects_manifest_without_backend_section() {
        let input = r#"
name = "gimp"
version = "1.0.0"
binary = "cli-anything-gimp"
description = "Rust CLI harness for GIMP"
repl_default = true
supports_json = true

[project]
format = "xcf"
state_file = ".gimp-cli.json"

[skill]
output = "packages/gimp/skills/SKILL.md"
template = "templates/skill/SKILL.md.template"
"#;

        let error = parse_manifest(input).expect_err("manifest should be rejected");

        let chain = error
            .chain()
            .map(|cause| cause.to_string())
            .collect::<Vec<_>>();
        assert!(chain.iter().any(|message| message.contains("backend")));
    }

    #[test]
    fn includes_curated_package_specs_for_second_stage() {
        let specs = builtin_package_specs();

        assert_eq!(specs.len(), 3);
        assert!(specs.iter().any(|spec| spec.name == "gimp"));
        assert!(specs.iter().any(|spec| spec.name == "blender"));
        assert!(specs.iter().any(|spec| spec.name == "drawio"));
    }

    #[test]
    fn gimp_curated_package_spec_exposes_expected_command_groups() {
        let spec = builtin_package_spec("gimp").expect("gimp spec should exist");

        assert_eq!(spec.project_format, "xcf");
        assert!(spec.command_groups.iter().any(|group| group.name == "layer"
            && group.commands.iter().any(|command| command.name == "list")));
        assert!(
            spec.command_groups
                .iter()
                .any(|group| group.name == "filter")
        );
    }
}
