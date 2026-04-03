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

pub fn parse_manifest(input: &str) -> anyhow::Result<CliAnythingManifest> {
    let manifest = toml::from_str::<CliAnythingManifest>(input)?;
    Ok(manifest)
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::parse_manifest;

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
output = "crates/shotcut-cli/skills/SKILL.md"
template = "templates/SKILL.md.template"

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
output = "crates/inkscape-cli/skills/SKILL.md"
template = "templates/SKILL.md.template"
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
output = "crates/gimp-cli/skills/SKILL.md"
template = "templates/SKILL.md.template"
"#;

        let error = parse_manifest(input).expect_err("manifest should be rejected");

        assert!(error.to_string().contains("backend"));
    }
}
