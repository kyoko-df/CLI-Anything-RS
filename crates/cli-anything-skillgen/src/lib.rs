use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cli_anything_core::{CliAnythingManifest, CommandGroup, ExampleSpec, load_manifest_from_path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillMetadata {
    pub software_name: String,
    pub skill_name: String,
    pub skill_description: String,
    pub skill_intro: String,
    pub version: String,
    pub binary: String,
    pub system_package: String,
    pub command_groups: Vec<CommandGroup>,
    pub examples: Vec<ExampleSpec>,
    pub repl_default: bool,
    pub supports_json: bool,
}

pub fn extract_skill_metadata(manifest: &CliAnythingManifest) -> SkillMetadata {
    let software_name = manifest.name.clone();
    SkillMetadata {
        skill_name: manifest.binary.clone(),
        skill_description: manifest.description.clone(),
        skill_intro: format!(
            "{} exposes a stateful Rust CLI workflow for {}.",
            manifest.binary, software_name
        ),
        version: manifest.version.clone(),
        binary: manifest.binary.clone(),
        system_package: manifest.backend.system_package.clone(),
        command_groups: manifest.command_groups.clone(),
        examples: manifest.examples.clone(),
        repl_default: manifest.repl_default,
        supports_json: manifest.supports_json,
        software_name,
    }
}

pub fn extract_skill_metadata_from_package_dir(package_dir: &Path) -> Result<SkillMetadata> {
    let manifest_path = package_dir.join("cli-anything.toml");
    let manifest = load_manifest_from_path(&manifest_path)?;
    Ok(extract_skill_metadata(&manifest))
}

pub fn render_skill_markdown(manifest: &CliAnythingManifest) -> String {
    render_skill_markdown_from_metadata(&extract_skill_metadata(manifest))
}

pub fn render_skill_markdown_from_metadata(metadata: &SkillMetadata) -> String {
    let mut sections = vec![
        format!(
            "---\nname: {}\ndescription: {}\n---\n",
            metadata.skill_name, metadata.skill_description
        ),
        format!("# {}\n", metadata.skill_name),
        format!("{}\n", metadata.skill_intro),
        "## Installation\n".to_string(),
        format!(
            "This CLI is installed as part of the `{}` package.\n",
            metadata.binary
        ),
        format!(
            "```bash\ncargo install --path packages/{}\n```\n",
            metadata.software_name
        ),
        "**Prerequisites:**\n".to_string(),
        format!(
            "- {} must be installed on your system\n",
            metadata.software_name
        ),
    ];

    if !metadata.system_package.trim().is_empty() {
        sections.push(format!(
            "- Install {}: `{}`\n",
            metadata.software_name, metadata.system_package
        ));
    }

    sections.push("## Usage\n".to_string());
    sections.push("### Basic Commands\n".to_string());
    sections.push(format!(
        "```bash\n{} --help\n{}\n{} --json\n```\n",
        metadata.binary, metadata.binary, metadata.binary
    ));

    if metadata.repl_default {
        sections.push("### REPL Mode\n".to_string());
        sections.push(format!(
            "Invoke `{}` without a subcommand to enter an interactive session.\n",
            metadata.binary
        ));
    }

    if !metadata.command_groups.is_empty() {
        sections.push("## Command Groups\n".to_string());
        for group in &metadata.command_groups {
            sections.push(format!("### {}\n", group.name));
            sections.push(format!("{}\n", group.description));
            sections.push("| Command | Description |\n|---------|-------------|\n".to_string());
            for command in &group.commands {
                sections.push(format!(
                    "| `{}` | {} |\n",
                    command.name, command.description
                ));
            }
            sections.push("\n".to_string());
        }
    }

    if !metadata.examples.is_empty() {
        sections.push("## Examples\n".to_string());
        for example in &metadata.examples {
            sections.push(format!("### {}\n", example.title));
            sections.push(format!("{}\n", example.description));
            sections.push(format!("```bash\n{}\n```\n", example.code));
        }
    }

    sections.push("## State Management\n".to_string());
    sections.push("- Undo/redo friendly command execution\n".to_string());
    sections.push("- Project persistence through state files\n".to_string());
    sections.push("- Session tracking for modified buffers\n".to_string());

    sections.push("## Output Formats\n".to_string());
    if metadata.supports_json {
        sections.push("- Human-readable output for operators\n".to_string());
        sections.push("- Machine-readable JSON output for agents\n".to_string());
    } else {
        sections.push("- Human-readable output\n".to_string());
    }

    sections.push("## For AI Agents\n".to_string());
    sections.push(format!(
        "1. Prefer `{} --json` when structured output is available\n",
        metadata.binary
    ));
    sections.push("2. Check exit codes before reading generated files\n".to_string());
    sections.push("3. Use absolute paths for package and fixture operations\n".to_string());

    sections.push("## Version\n".to_string());
    sections.push(format!("{}\n", metadata.version));

    sections.join("\n")
}

pub fn generate_skill_file(
    manifest: &CliAnythingManifest,
    output_path: Option<&Path>,
) -> Result<PathBuf> {
    let resolved_output = output_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(&manifest.skill.output));
    let markdown = render_skill_markdown(manifest);
    if let Some(parent) = resolved_output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&resolved_output, markdown)
        .with_context(|| format!("failed to write {}", resolved_output.display()))?;
    Ok(resolved_output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cli_anything_core::parse_manifest;
    use std::time::{SystemTime, UNIX_EPOCH};

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
    fn renders_frontmatter_from_manifest() {
        let manifest = parse_manifest(SAMPLE_MANIFEST).expect("manifest should parse");
        let markdown = render_skill_markdown(&manifest);

        assert!(markdown.contains("name: cli-anything-shotcut"));
        assert!(markdown.contains("# cli-anything-shotcut"));
        assert!(markdown.contains("## Command Groups"));
        assert!(markdown.contains("## Examples"));
        assert!(markdown.contains("melt ffmpeg"));
    }

    #[test]
    fn extracts_metadata_from_package_directory() {
        let package_dir = unique_test_dir("skill-metadata");
        fs::create_dir_all(&package_dir).expect("package dir should exist");
        fs::write(package_dir.join("cli-anything.toml"), SAMPLE_MANIFEST)
            .expect("manifest should be written");

        let metadata =
            extract_skill_metadata_from_package_dir(&package_dir).expect("metadata should load");

        assert_eq!(metadata.software_name, "shotcut");
        assert_eq!(metadata.binary, "cli-anything-shotcut");

        fs::remove_dir_all(&package_dir).expect("test dir should be removed");
    }

    #[test]
    fn writes_skill_file_to_requested_output_path() {
        let manifest = parse_manifest(SAMPLE_MANIFEST).expect("manifest should parse");
        let output_dir = unique_test_dir("skill-output");
        let output_path = output_dir.join("SKILL.md");

        let written_path =
            generate_skill_file(&manifest, Some(&output_path)).expect("skill file should write");
        let content = fs::read_to_string(&written_path).expect("skill file should exist");

        assert_eq!(written_path, output_path);
        assert!(content.contains("## For AI Agents"));

        fs::remove_dir_all(&output_dir).expect("test dir should be removed");
    }

    fn unique_test_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        std::env::temp_dir().join(format!("cli-anything-rs-{prefix}-{nanos}"))
    }
}
