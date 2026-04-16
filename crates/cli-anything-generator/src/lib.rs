//! Code generation for Rust packages produced by CLI-Anything-RS.
//!
//! Given a manifest, this crate renders the Cargo.toml, `src/main.rs`,
//! smoke tests, and drives `cli-anything-skillgen` to write the SKILL.md
//! file. It is split out of `cli-anything-cli` so that other front-ends
//! (plugins, tests, custom tooling) can reuse the generator without
//! pulling in the command-line binary.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cli_anything_manifest::{
    BackendConfig, CliAnythingManifest, CommandGroup, CommandSpec, ExampleSpec, ProjectConfig,
    SkillConfig, builtin_package_spec,
};
use cli_anything_skillgen::generate_skill_file;

/// Fully-qualified paths for a package scaffold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageLayout {
    pub software_name: String,
    pub root: PathBuf,
    pub cargo_toml: PathBuf,
    pub manifest: PathBuf,
    pub src_dir: PathBuf,
    pub src_main: PathBuf,
    pub skills_dir: PathBuf,
    pub skill_file: PathBuf,
    pub tests_dir: PathBuf,
    pub fixtures_dir: PathBuf,
}

pub fn package_layout(workspace_root: &Path, software_name: &str) -> PackageLayout {
    let root = workspace_root.join("packages").join(software_name);
    let src_dir = root.join("src");
    let skills_dir = root.join("skills");
    PackageLayout {
        software_name: software_name.to_string(),
        cargo_toml: root.join("Cargo.toml"),
        manifest: root.join("cli-anything.toml"),
        src_main: src_dir.join("main.rs"),
        skill_file: skills_dir.join("SKILL.md"),
        tests_dir: root.join("tests"),
        fixtures_dir: root.join("fixtures"),
        root,
        src_dir,
        skills_dir,
    }
}

/// Result of generating a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedPackage {
    pub manifest: CliAnythingManifest,
    pub layout: PackageLayout,
    pub files: Vec<PathBuf>,
}

/// Build a manifest for `software_name`, using curated metadata when
/// available and falling back to a generic two-group scaffold otherwise.
pub fn scaffold_manifest(software_name: &str) -> CliAnythingManifest {
    if let Some(spec) = builtin_package_spec(software_name) {
        return CliAnythingManifest {
            name: spec.name.clone(),
            version: "1.0.0".to_string(),
            binary: format!("cli-anything-{}", spec.name),
            description: spec.description,
            repl_default: true,
            supports_json: true,
            backend: BackendConfig {
                command: spec.backend_command,
                system_package: spec.system_package,
                hard_dependency: true,
            },
            project: ProjectConfig {
                format: spec.project_format,
                state_file: spec.state_file,
            },
            skill: SkillConfig {
                output: format!("packages/{software_name}/skills/SKILL.md"),
                template: "cli-anything-plugin/templates/SKILL.md.template".to_string(),
            },
            command_groups: spec.command_groups,
            examples: spec.examples,
        };
    }

    CliAnythingManifest {
        name: software_name.to_string(),
        version: "0.1.0".to_string(),
        binary: format!("cli-anything-{software_name}"),
        description: format!("Rust CLI package for {software_name}"),
        repl_default: true,
        supports_json: true,
        backend: BackendConfig {
            command: software_name.to_string(),
            system_package: software_name.to_string(),
            hard_dependency: true,
        },
        project: ProjectConfig {
            format: "json".to_string(),
            state_file: format!(".{software_name}-cli.json"),
        },
        skill: SkillConfig {
            output: format!("packages/{software_name}/skills/SKILL.md"),
            template: "cli-anything-plugin/templates/SKILL.md.template".to_string(),
        },
        command_groups: vec![
            CommandGroup {
                name: "project".to_string(),
                description: "Project lifecycle commands".to_string(),
                commands: vec![
                    CommandSpec {
                        name: "new".to_string(),
                        description: "Create a new project or scene".to_string(),
                    },
                    CommandSpec {
                        name: "info".to_string(),
                        description: "Inspect the current project state".to_string(),
                    },
                ],
            },
            CommandGroup {
                name: "session".to_string(),
                description: "Session state and history commands".to_string(),
                commands: vec![
                    CommandSpec {
                        name: "undo".to_string(),
                        description: "Undo the last operation".to_string(),
                    },
                    CommandSpec {
                        name: "redo".to_string(),
                        description: "Redo the last undone operation".to_string(),
                    },
                ],
            },
        ],
        examples: vec![ExampleSpec {
            title: "Start a new project".to_string(),
            description: "Create a fresh state file for the generated package.".to_string(),
            code: format!("cli-anything-{software_name} project new -o demo.json"),
        }],
    }
}

/// Write the generated package scaffold to disk.
pub fn generate_package(
    workspace_root: &Path,
    software_name: &str,
    dry_run: bool,
) -> Result<GeneratedPackage> {
    let layout = package_layout(workspace_root, software_name);
    let manifest = scaffold_manifest(software_name);
    let files = vec![
        layout.cargo_toml.clone(),
        layout.manifest.clone(),
        layout.src_main.clone(),
        layout.skill_file.clone(),
        layout.tests_dir.join("smoke.rs"),
        layout.fixtures_dir.join(".keep"),
    ];

    if !dry_run {
        fs::create_dir_all(&layout.src_dir)
            .with_context(|| format!("failed to create {}", layout.src_dir.display()))?;
        fs::create_dir_all(&layout.skills_dir)
            .with_context(|| format!("failed to create {}", layout.skills_dir.display()))?;
        fs::create_dir_all(&layout.tests_dir)
            .with_context(|| format!("failed to create {}", layout.tests_dir.display()))?;
        fs::create_dir_all(&layout.fixtures_dir)
            .with_context(|| format!("failed to create {}", layout.fixtures_dir.display()))?;

        fs::write(&layout.cargo_toml, render_package_cargo_toml(&manifest))
            .with_context(|| format!("failed to write {}", layout.cargo_toml.display()))?;
        fs::write(
            &layout.manifest,
            toml::to_string_pretty(&manifest).context("failed to encode manifest")?,
        )
        .with_context(|| format!("failed to write {}", layout.manifest.display()))?;
        fs::write(&layout.src_main, render_package_main_rs(&manifest))
            .with_context(|| format!("failed to write {}", layout.src_main.display()))?;
        fs::write(
            layout.tests_dir.join("smoke.rs"),
            render_smoke_test(&manifest),
        )
        .with_context(|| format!("failed to write {}", layout.tests_dir.display()))?;
        fs::write(layout.fixtures_dir.join(".keep"), "")
            .with_context(|| format!("failed to write {}", layout.fixtures_dir.display()))?;
        generate_skill_file(&manifest, Some(&layout.skill_file))?;
    }

    Ok(GeneratedPackage {
        manifest,
        layout,
        files,
    })
}

pub fn render_package_cargo_toml(manifest: &CliAnythingManifest) -> String {
    format!(
        "[package]\nname = \"{}\"\nversion = \"{}\"\nedition = \"2024\"\n\n[dependencies]\nclap = {{ version = \"4.5\", features = [\"derive\"] }}\ncli-anything-repl = {{ path = \"../../crates/cli-anything-repl\" }}\nserde = {{ version = \"1.0\", features = [\"derive\"] }}\nserde_json = \"1.0\"\n",
        manifest.binary, manifest.version
    )
}

pub fn render_package_main_rs(manifest: &CliAnythingManifest) -> String {
    let action_variants = manifest
        .command_groups
        .iter()
        .map(|group| {
            let group_type = to_pascal_case(&group.name);
            format!(
                "    {group_type} {{\n        #[command(subcommand)]\n        command: {group_type}Command,\n    }},\n"
            )
        })
        .collect::<String>();
    let command_enums = manifest
        .command_groups
        .iter()
        .map(render_command_group_enum)
        .collect::<Vec<_>>()
        .join("\n\n");
    let command_name_fns = manifest
        .command_groups
        .iter()
        .map(render_command_name_fn)
        .collect::<Vec<_>>()
        .join("\n\n");
    let command_description_fns = manifest
        .command_groups
        .iter()
        .map(render_command_description_fn)
        .collect::<Vec<_>>()
        .join("\n\n");
    let command_match_arms = manifest
        .command_groups
        .iter()
        .map(|group| {
            let group_type = to_pascal_case(&group.name);
            let helper_name = format!("{}_command_name", to_snake_case(&group.name));
            let helper_description =
                format!("{}_command_description", to_snake_case(&group.name));
            format!(
                "        Action::{group_type} {{ command }} => command_response(\"{group_name}\", {helper_name}(&command), {helper_description}(&command)),\n",
                group_name = group.name,
            )
        })
        .collect::<String>();
    let command_groups = manifest
        .command_groups
        .iter()
        .map(|group| format!("\"{}\"", group.name))
        .collect::<Vec<_>>()
        .join(", ");
    let command_group_list = manifest
        .command_groups
        .iter()
        .map(|group| group.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let skill_path = manifest.skill.output.clone();

    format!(
        "use clap::{{Parser, Subcommand}};\nuse cli_anything_repl::Skin;\nuse serde::Serialize;\n\n#[derive(Debug, Parser)]\n#[command(name = \"{binary}\")]\n#[command(about = \"{description}\")]\nstruct App {{\n    #[arg(long)]\n    json: bool,\n    #[command(subcommand)]\n    action: Option<Action>,\n}}\n\n#[derive(Debug, Subcommand)]\nenum Action {{\n{action_variants}}}\n\n{command_enums}\n\n#[derive(Debug, Serialize)]\nstruct PackageSummary {{\n    name: &'static str,\n    binary: &'static str,\n    version: &'static str,\n    description: &'static str,\n    project_format: &'static str,\n    skill_path: &'static str,\n    command_groups: Vec<&'static str>,\n    supports_json: bool,\n    repl_default: bool,\n}}\n\n#[derive(Debug, Serialize)]\nstruct CommandResponse {{\n    software: &'static str,\n    binary: &'static str,\n    group: &'static str,\n    command: &'static str,\n    description: &'static str,\n}}\n\nfn main() {{\n    let app = App::parse();\n    let skin = Skin::new(\"{name}\", \"{version}\").with_skill_path(\"skills/SKILL.md\");\n\n    match app.action {{\n        Some(action) => {{\n            let response = match action {{\n{command_match_arms}            }};\n            if app.json {{\n                println!(\"{{}}\", serde_json::to_string_pretty(&response).expect(\"command response should serialize\"));\n            }} else {{\n                println!(\"{{}}\", skin.info(&format!(\"{{}} -> {{}}\", response.group, response.command)));\n                println!(\"{{}}\", skin.status(\"detail\", response.description));\n            }}\n        }}\n        None => {{\n            let summary = package_summary();\n            if app.json {{\n                println!(\"{{}}\", serde_json::to_string_pretty(&summary).expect(\"package summary should serialize\"));\n            }} else {{\n                for line in skin.banner_lines() {{\n                    println!(\"{{line}}\");\n                }}\n                println!(\"{{}}\", skin.status(\"binary\", \"{binary}\"));\n                println!(\"{{}}\", skin.status(\"format\", \"{project_format}\"));\n                println!(\"{{}}\", skin.status(\"groups\", \"{command_group_list}\"));\n            }}\n        }}\n    }}\n}}\n\nfn package_summary() -> PackageSummary {{\n    PackageSummary {{\n        name: \"{name}\",\n        binary: \"{binary}\",\n        version: \"{version}\",\n        description: \"{description}\",\n        project_format: \"{project_format}\",\n        skill_path: \"{skill_path}\",\n        command_groups: vec![{command_groups}],\n        supports_json: true,\n        repl_default: true,\n    }}\n}}\n\nfn command_response(group: &'static str, command: &'static str, description: &'static str) -> CommandResponse {{\n    CommandResponse {{\n        software: \"{name}\",\n        binary: \"{binary}\",\n        group,\n        command,\n        description,\n    }}\n}}\n\n{command_name_fns}\n\n{command_description_fns}\n",
        name = manifest.name,
        binary = manifest.binary,
        description = manifest.description,
        version = manifest.version,
        project_format = manifest.project.format,
        skill_path = skill_path,
        action_variants = action_variants,
        command_enums = command_enums,
        command_match_arms = command_match_arms,
        command_groups = command_groups,
        command_group_list = command_group_list,
        command_name_fns = command_name_fns,
        command_description_fns = command_description_fns,
    )
}

pub fn render_smoke_test(manifest: &CliAnythingManifest) -> String {
    let first_group = manifest
        .command_groups
        .first()
        .expect("generated package should include at least one command group");
    let first_command = first_group
        .commands
        .first()
        .expect("generated package should include at least one command");
    let command_group_count = manifest.command_groups.len();

    format!(
        "use std::process::Command;\n\nuse serde_json::Value;\n\nfn run_binary(args: &[&str]) -> std::process::Output {{\n    Command::new(env!(\"CARGO_BIN_EXE_{binary}\"))\n        .args(args)\n        .output()\n        .expect(\"generated binary should run\")\n}}\n\n#[test]\nfn binary_name_is_stable() {{\n    assert_eq!(\"{binary}\", \"{binary}\");\n}}\n\n#[test]\nfn json_summary_reports_package_metadata() {{\n    let output = run_binary(&[\"--json\"]);\n\n    assert!(output.status.success());\n\n    let payload: Value = serde_json::from_slice(&output.stdout)\n        .expect(\"summary output should be valid json\");\n\n    assert_eq!(payload[\"name\"], \"{name}\");\n    assert_eq!(payload[\"binary\"], \"{binary}\");\n    assert_eq!(payload[\"version\"], \"{version}\");\n    assert_eq!(payload[\"description\"], \"{description}\");\n    assert_eq!(payload[\"project_format\"], \"{project_format}\");\n    assert_eq!(\n        payload[\"command_groups\"].as_array().map(Vec::len),\n        Some({command_group_count})\n    );\n}}\n\n#[test]\nfn json_subcommand_response_includes_description() {{\n    let output = run_binary(&[\"--json\", \"{first_group}\", \"{first_command}\"]);\n\n    assert!(output.status.success());\n\n    let payload: Value = serde_json::from_slice(&output.stdout)\n        .expect(\"command output should be valid json\");\n\n    assert_eq!(payload[\"software\"], \"{name}\");\n    assert_eq!(payload[\"group\"], \"{first_group}\");\n    assert_eq!(payload[\"command\"], \"{first_command}\");\n    assert_eq!(payload[\"description\"], \"{first_description}\");\n}}\n",
        name = manifest.name,
        binary = manifest.binary,
        version = manifest.version,
        description = manifest.description,
        project_format = manifest.project.format,
        command_group_count = command_group_count,
        first_group = first_group.name,
        first_command = first_command.name,
        first_description = first_command.description,
    )
}

fn render_command_group_enum(group: &CommandGroup) -> String {
    let group_type = to_pascal_case(&group.name);
    let variants = group
        .commands
        .iter()
        .map(|command| format!("    {},\n", to_pascal_case(&command.name)))
        .collect::<String>();

    format!("#[derive(Debug, Subcommand)]\nenum {group_type}Command {{\n{variants}}}")
}

fn render_command_name_fn(group: &CommandGroup) -> String {
    let group_type = to_pascal_case(&group.name);
    let function_name = format!("{}_command_name", to_snake_case(&group.name));
    let match_arms = group
        .commands
        .iter()
        .map(|command| {
            format!(
                "        {group_type}Command::{} => \"{}\",\n",
                to_pascal_case(&command.name),
                command.name
            )
        })
        .collect::<String>();

    format!(
        "fn {function_name}(command: &{group_type}Command) -> &'static str {{\n    match command {{\n{match_arms}    }}\n}}"
    )
}

fn render_command_description_fn(group: &CommandGroup) -> String {
    let group_type = to_pascal_case(&group.name);
    let function_name = format!("{}_command_description", to_snake_case(&group.name));
    let match_arms = group
        .commands
        .iter()
        .map(|command| {
            format!(
                "        {group_type}Command::{} => \"{}\",\n",
                to_pascal_case(&command.name),
                command.description
            )
        })
        .collect::<String>();

    format!(
        "fn {function_name}(command: &{group_type}Command) -> &'static str {{\n    match command {{\n{match_arms}    }}\n}}"
    )
}

pub fn to_pascal_case(value: &str) -> String {
    value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut characters = segment.chars();
            match characters.next() {
                Some(first) => {
                    let mut title = first.to_uppercase().collect::<String>();
                    title.push_str(&characters.as_str().to_lowercase());
                    title
                }
                None => String::new(),
            }
        })
        .collect::<String>()
}

pub fn to_snake_case(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_test_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        std::env::temp_dir().join(format!("cli-anything-rs-generator-{prefix}-{nanos}"))
    }

    #[test]
    fn scaffold_manifest_uses_curated_metadata_for_known_targets() {
        let manifest = scaffold_manifest("blender");

        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.project.format, "blend");
        assert!(
            manifest
                .command_groups
                .iter()
                .any(|group| group.name == "render")
        );
    }

    #[test]
    fn scaffold_manifest_falls_back_to_generic_template() {
        let manifest = scaffold_manifest("shotcut");

        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.project.format, "json");
        assert_eq!(manifest.command_groups.len(), 2);
    }

    #[test]
    fn generate_package_writes_scaffold_to_disk() {
        let workspace = unique_test_dir("build");
        fs::create_dir_all(&workspace).expect("workspace should exist");

        let result = generate_package(&workspace, "drawio", false).expect("build should succeed");

        assert!(result.layout.cargo_toml.exists());
        assert!(result.layout.manifest.exists());
        assert!(result.layout.src_main.exists());
        assert!(result.layout.skill_file.exists());
        assert!(result.layout.tests_dir.join("smoke.rs").exists());

        fs::remove_dir_all(&workspace).expect("workspace should be removed");
    }

    #[test]
    fn generated_main_includes_summary_and_command_descriptions() {
        let manifest = scaffold_manifest("gimp");
        let source = render_package_main_rs(&manifest);

        assert!(source.contains("description: &'static str"));
        assert!(source.contains("project_format: &'static str"));
        assert!(source.contains("Action::Project"));
        assert!(source.contains("Create a new image project"));
    }

    #[test]
    fn generated_smoke_test_covers_json_summary_and_first_command() {
        let manifest = scaffold_manifest("drawio");
        let source = render_smoke_test(&manifest);

        assert!(source.contains("json_summary_reports_package_metadata"));
        assert!(source.contains("json_subcommand_response_includes_description"));
        assert!(source.contains("cli-anything-drawio"));
    }
}
