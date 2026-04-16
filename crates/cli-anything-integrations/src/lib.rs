//! Output adapters that render a CLI-Anything-RS package into the
//! format expected by third-party agent frameworks.
//!
//! Each target (Claude, OpenCode, Codex, Hub, ...) is represented by a
//! [`IntegrationTarget`] variant. Callers obtain an [`IntegrationOutput`]
//! from [`render_integration`] and decide where to write it – the crate
//! itself does not touch the filesystem. This keeps the adapters pure
//! and trivially testable.

use cli_anything_manifest::CliAnythingManifest;
use serde::{Deserialize, Serialize};

/// Known integration targets. New targets should prefer extending this
/// enum over introducing a parallel string-based API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntegrationTarget {
    Claude,
    OpenCode,
    Codex,
    Hub,
}

impl IntegrationTarget {
    pub fn id(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::OpenCode => "opencode",
            Self::Codex => "codex",
            Self::Hub => "hub",
        }
    }

    pub fn all() -> [IntegrationTarget; 4] {
        [Self::Claude, Self::OpenCode, Self::Codex, Self::Hub]
    }
}

/// Rendered output for a single integration target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationOutput {
    pub target: IntegrationTarget,
    pub filename: String,
    pub content: String,
}

pub fn render_integration(
    manifest: &CliAnythingManifest,
    target: IntegrationTarget,
) -> IntegrationOutput {
    match target {
        IntegrationTarget::Claude => render_claude(manifest),
        IntegrationTarget::OpenCode => render_opencode(manifest),
        IntegrationTarget::Codex => render_codex(manifest),
        IntegrationTarget::Hub => render_hub(manifest),
    }
}

pub fn render_all_integrations(manifest: &CliAnythingManifest) -> Vec<IntegrationOutput> {
    IntegrationTarget::all()
        .into_iter()
        .map(|target| render_integration(manifest, target))
        .collect()
}

fn render_claude(manifest: &CliAnythingManifest) -> IntegrationOutput {
    let mut sections = vec![
        format!("# {}", manifest.binary),
        String::new(),
        format!(
            "{} – {} (v{})",
            manifest.name, manifest.description, manifest.version
        ),
        String::new(),
        "## Usage".to_string(),
        format!(
            "- Invoke `{binary} --help` to inspect the full command surface.",
            binary = manifest.binary
        ),
        format!(
            "- Prefer `{binary} --json` when orchestrating from Claude tools.",
            binary = manifest.binary
        ),
    ];

    if !manifest.command_groups.is_empty() {
        sections.push(String::new());
        sections.push("## Command Groups".to_string());
        for group in &manifest.command_groups {
            sections.push(format!("- `{}`: {}", group.name, group.description));
        }
    }

    IntegrationOutput {
        target: IntegrationTarget::Claude,
        filename: "CLAUDE.md".to_string(),
        content: sections.join("\n"),
    }
}

fn render_opencode(manifest: &CliAnythingManifest) -> IntegrationOutput {
    let content = format!(
        "# OpenCode integration\n\nbinary: {binary}\nversion: {version}\nformat: {format}\n",
        binary = manifest.binary,
        version = manifest.version,
        format = manifest.project.format,
    );
    IntegrationOutput {
        target: IntegrationTarget::OpenCode,
        filename: "opencode.md".to_string(),
        content,
    }
}

fn render_codex(manifest: &CliAnythingManifest) -> IntegrationOutput {
    let groups = manifest
        .command_groups
        .iter()
        .map(|group| format!("  - {}", group.name))
        .collect::<Vec<_>>()
        .join("\n");
    let content = format!(
        "codex:\n  binary: {binary}\n  software: {name}\n  description: \"{description}\"\n  command_groups:\n{groups}\n",
        binary = manifest.binary,
        name = manifest.name,
        description = manifest.description,
        groups = groups,
    );
    IntegrationOutput {
        target: IntegrationTarget::Codex,
        filename: "codex.yaml".to_string(),
        content,
    }
}

fn render_hub(manifest: &CliAnythingManifest) -> IntegrationOutput {
    let content = format!(
        "# Hub listing\n\n- name: {name}\n- binary: {binary}\n- version: {version}\n- description: {description}\n",
        name = manifest.name,
        binary = manifest.binary,
        version = manifest.version,
        description = manifest.description,
    );
    IntegrationOutput {
        target: IntegrationTarget::Hub,
        filename: "hub.md".to_string(),
        content,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cli_anything_manifest::parse_manifest;

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
"#;

    #[test]
    fn claude_output_lists_command_groups() {
        let manifest = parse_manifest(SAMPLE_MANIFEST).expect("manifest should parse");
        let output = render_integration(&manifest, IntegrationTarget::Claude);

        assert_eq!(output.filename, "CLAUDE.md");
        assert!(output.content.contains("cli-anything-shotcut"));
        assert!(output.content.contains("## Command Groups"));
        assert!(output.content.contains("project"));
    }

    #[test]
    fn render_all_integrations_covers_every_target() {
        let manifest = parse_manifest(SAMPLE_MANIFEST).expect("manifest should parse");
        let outputs = render_all_integrations(&manifest);

        assert_eq!(outputs.len(), IntegrationTarget::all().len());
        let ids = outputs
            .iter()
            .map(|output| output.target.id())
            .collect::<Vec<_>>();
        assert!(ids.contains(&"claude"));
        assert!(ids.contains(&"opencode"));
        assert!(ids.contains(&"codex"));
        assert!(ids.contains(&"hub"));
    }

    #[test]
    fn codex_output_uses_yaml_shape() {
        let manifest = parse_manifest(SAMPLE_MANIFEST).expect("manifest should parse");
        let output = render_integration(&manifest, IntegrationTarget::Codex);

        assert_eq!(output.filename, "codex.yaml");
        assert!(output.content.starts_with("codex:"));
        assert!(output.content.contains("binary: cli-anything-shotcut"));
    }
}
