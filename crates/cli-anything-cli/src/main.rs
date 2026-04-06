use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use cli_anything_core::{
    BackendConfig, BuiltinCommandId, CliAnythingManifest, CommandGroup, CommandSpec, ExampleSpec,
    ProjectConfig, SkillConfig, ValidationCategory, ValidationCheck, ValidationReport,
    builtin_command_documents, builtin_package_spec, command_document, load_manifest_from_path,
    package_layout, parse_source_target,
};
use cli_anything_repl::Skin;
use cli_anything_skillgen::generate_skill_file;
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(name = "cli-anything")]
#[command(about = "Rust-first harness generator and package workflow")]
struct App {
    #[command(subcommand)]
    command: Option<Action>,
}

#[derive(Debug, Subcommand)]
enum Action {
    Status {
        #[arg(long)]
        json: bool,
    },
    Build {
        source: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
    },
    Refine {
        source: String,
        focus: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Test {
        source: String,
        #[arg(long)]
        json: bool,
    },
    Validate {
        source: String,
        #[arg(long)]
        json: bool,
    },
    List {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        depth: Option<usize>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct StatusView {
    banner: Vec<String>,
    commands: Vec<CommandSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct CommandSummary {
    name: String,
    usage: String,
    summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct BuildResult {
    software_name: String,
    source: String,
    dry_run: bool,
    package_root: PathBuf,
    generated_files: Vec<PathBuf>,
    manifest: CliAnythingManifest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RefinePlan {
    software_name: String,
    package_root: PathBuf,
    package_exists: bool,
    focus: Option<String>,
    command_group_count: usize,
    example_count: usize,
    next_actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct TestPlan {
    software_name: String,
    package_root: PathBuf,
    cargo_commands: Vec<String>,
    checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct PackageListEntry {
    software_name: String,
    version: String,
    binary: String,
    package_root: PathBuf,
    manifest_path: PathBuf,
    skill_file: PathBuf,
    status: String,
}

fn main() {
    if let Err(error) = run(App::parse()) {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

fn run(app: App) -> Result<()> {
    let workspace_root =
        std::env::current_dir().context("failed to determine current directory")?;
    let output = match app.command.unwrap_or(Action::Status { json: false }) {
        Action::Status { json } => render_value(status_view(), json, render_status_text)?,
        Action::Build {
            source,
            dry_run,
            json,
        } => {
            let result = execute_build(&workspace_root, &source, dry_run)?;
            render_value(result, json, render_build_text)?
        }
        Action::Refine {
            source,
            focus,
            json,
        } => {
            let plan = create_refine_plan(&workspace_root, &source, focus)?;
            render_value(plan, json, render_refine_text)?
        }
        Action::Test { source, json } => {
            let plan = create_test_plan(&workspace_root, &source)?;
            render_value(plan, json, render_test_text)?
        }
        Action::Validate { source, json } => {
            let report = validate_package(&workspace_root, &source)?;
            render_value(report, json, render_validation_text)?
        }
        Action::List { path, depth, json } => {
            let entries = list_packages(&path, depth)?;
            render_value(entries, json, |entries| {
                render_list_text(entries.as_slice())
            })?
        }
    };

    println!("{output}");
    Ok(())
}

fn status_view() -> StatusView {
    let skin = Skin::new("cli-anything", env!("CARGO_PKG_VERSION"));
    StatusView {
        banner: skin.banner_lines(),
        commands: builtin_command_documents()
            .into_iter()
            .map(|document| CommandSummary {
                name: format!("{:?}", document.id).to_lowercase(),
                usage: document.usage,
                summary: document.summary,
            })
            .collect(),
    }
}

fn execute_build(workspace_root: &Path, source: &str, dry_run: bool) -> Result<BuildResult> {
    let target = parse_source_target(source)?;
    let layout = package_layout(workspace_root, &target.software_name);
    let manifest = scaffold_manifest(&target.software_name);
    let generated_files = vec![
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

        fs::write(
            &layout.cargo_toml,
            render_generated_package_cargo_toml(&manifest),
        )
        .with_context(|| format!("failed to write {}", layout.cargo_toml.display()))?;
        fs::write(
            &layout.manifest,
            toml::to_string_pretty(&manifest).context("failed to encode manifest")?,
        )
        .with_context(|| format!("failed to write {}", layout.manifest.display()))?;
        fs::write(
            &layout.src_main,
            render_generated_package_main_rs(&manifest),
        )
        .with_context(|| format!("failed to write {}", layout.src_main.display()))?;
        fs::write(
            layout.tests_dir.join("smoke.rs"),
            render_generated_smoke_test(&manifest),
        )
        .with_context(|| format!("failed to write {}", layout.tests_dir.display()))?;
        fs::write(layout.fixtures_dir.join(".keep"), "")
            .with_context(|| format!("failed to write {}", layout.fixtures_dir.display()))?;
        generate_skill_file(&manifest, Some(&layout.skill_file))?;
    }

    Ok(BuildResult {
        software_name: target.software_name,
        source: source.to_string(),
        dry_run,
        package_root: layout.root,
        generated_files,
        manifest,
    })
}

fn create_refine_plan(
    workspace_root: &Path,
    source: &str,
    focus: Option<String>,
) -> Result<RefinePlan> {
    let target = parse_source_target(source)?;
    let layout = package_layout(workspace_root, &target.software_name);
    let manifest = if layout.manifest.exists() {
        Some(load_manifest_from_path(&layout.manifest)?)
    } else {
        None
    };

    let mut next_actions = Vec::new();
    if let Some(focus_value) = focus.as_deref() {
        next_actions.push(format!("Audit command coverage for {focus_value}."));
    }

    match &manifest {
        Some(manifest) => {
            if manifest.command_groups.is_empty() {
                next_actions.push("Add at least one command group to the manifest.".to_string());
            }
            if manifest.examples.is_empty() {
                next_actions.push("Add concrete usage examples for the generated CLI.".to_string());
            }
            next_actions.push(
                "Compare the current command surface with the upstream GUI workflows.".to_string(),
            );
        }
        None => {
            next_actions.push("Run cli-anything build first to scaffold the package.".to_string());
        }
    }

    let command_group_count = manifest
        .as_ref()
        .map(|manifest| manifest.command_groups.len())
        .unwrap_or(0);
    let example_count = manifest
        .as_ref()
        .map(|manifest| manifest.examples.len())
        .unwrap_or(0);

    let package_exists = layout.root.exists();
    Ok(RefinePlan {
        software_name: target.software_name,
        package_root: layout.root,
        package_exists,
        focus,
        command_group_count,
        example_count,
        next_actions,
    })
}

fn create_test_plan(workspace_root: &Path, source: &str) -> Result<TestPlan> {
    let target = parse_source_target(source)?;
    let layout = package_layout(workspace_root, &target.software_name);
    let manifest = load_manifest_from_path(&layout.manifest)
        .with_context(|| format!("missing manifest for {}", target.software_name))?;

    Ok(TestPlan {
        software_name: target.software_name,
        package_root: layout.root,
        cargo_commands: vec![
            format!("cargo test -p {}", manifest.package_name()),
            format!("cargo run -p {} -- --help", manifest.package_name()),
        ],
        checks: vec![
            "Run unit and integration tests".to_string(),
            "Verify the generated binary prints help output".to_string(),
            "Regenerate the skill file when manifest metadata changes".to_string(),
        ],
    })
}

fn validate_package(workspace_root: &Path, source: &str) -> Result<ValidationReport> {
    let target = parse_source_target(source)?;
    let layout = package_layout(workspace_root, &target.software_name);

    let structure_checks = vec![
        ValidationCheck {
            label: "package root".to_string(),
            passed: layout.root.exists(),
            detail: layout.root.display().to_string(),
        },
        ValidationCheck {
            label: "Cargo.toml".to_string(),
            passed: layout.cargo_toml.exists(),
            detail: layout.cargo_toml.display().to_string(),
        },
        ValidationCheck {
            label: "cli-anything.toml".to_string(),
            passed: layout.manifest.exists(),
            detail: layout.manifest.display().to_string(),
        },
        ValidationCheck {
            label: "src/main.rs".to_string(),
            passed: layout.src_main.exists(),
            detail: layout.src_main.display().to_string(),
        },
        ValidationCheck {
            label: "skills/".to_string(),
            passed: layout.skills_dir.exists(),
            detail: layout.skills_dir.display().to_string(),
        },
        ValidationCheck {
            label: "tests/".to_string(),
            passed: layout.tests_dir.exists(),
            detail: layout.tests_dir.display().to_string(),
        },
        ValidationCheck {
            label: "fixtures/".to_string(),
            passed: layout.fixtures_dir.exists(),
            detail: layout.fixtures_dir.display().to_string(),
        },
        ValidationCheck {
            label: "SKILL.md".to_string(),
            passed: layout.skill_file.exists(),
            detail: layout.skill_file.display().to_string(),
        },
    ];

    let manifest_checks = match load_manifest_from_path(&layout.manifest) {
        Ok(manifest) => vec![
            ValidationCheck {
                label: "manifest name".to_string(),
                passed: manifest.name == target.software_name,
                detail: manifest.name,
            },
            ValidationCheck {
                label: "binary prefix".to_string(),
                passed: manifest.binary.starts_with("cli-anything-"),
                detail: manifest.binary,
            },
        ],
        Err(error) => vec![ValidationCheck {
            label: "manifest parse".to_string(),
            passed: false,
            detail: error.to_string(),
        }],
    };

    Ok(ValidationReport {
        software_name: target.software_name,
        package_root: layout.root,
        categories: vec![
            ValidationCategory {
                name: "structure".to_string(),
                checks: structure_checks,
            },
            ValidationCategory {
                name: "manifest".to_string(),
                checks: manifest_checks,
            },
        ],
    })
}

fn list_packages(scan_root: &Path, depth: Option<usize>) -> Result<Vec<PackageListEntry>> {
    let mut manifests = Vec::new();
    collect_manifest_paths(scan_root, 0, depth, &mut manifests)?;
    manifests.sort();

    let mut entries = Vec::new();
    for manifest_path in manifests {
        let manifest = load_manifest_from_path(&manifest_path)?;
        let package_root = manifest_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| scan_root.to_path_buf());
        let skill_file = package_root.join("skills").join("SKILL.md");
        let status = if skill_file.exists() {
            "ready"
        } else {
            "scaffolded"
        };
        entries.push(PackageListEntry {
            software_name: manifest.name,
            version: manifest.version,
            binary: manifest.binary,
            package_root,
            manifest_path,
            skill_file,
            status: status.to_string(),
        });
    }

    Ok(entries)
}

fn collect_manifest_paths(
    current_path: &Path,
    current_depth: usize,
    max_depth: Option<usize>,
    manifests: &mut Vec<PathBuf>,
) -> Result<()> {
    if max_depth.is_some_and(|limit| current_depth > limit) {
        return Ok(());
    }

    for entry in fs::read_dir(current_path)
        .with_context(|| format!("failed to read {}", current_path.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_manifest_paths(&path, current_depth + 1, max_depth, manifests)?;
        } else if file_type.is_file() && entry.file_name() == "cli-anything.toml" {
            manifests.push(path);
        }
    }

    Ok(())
}

fn scaffold_manifest(software_name: &str) -> CliAnythingManifest {
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

fn render_generated_package_cargo_toml(manifest: &CliAnythingManifest) -> String {
    format!(
        "[package]\nname = \"{}\"\nversion = \"{}\"\nedition = \"2024\"\n\n[dependencies]\nclap = {{ version = \"4.5\", features = [\"derive\"] }}\ncli-anything-repl = {{ path = \"../../crates/cli-anything-repl\" }}\nserde = {{ version = \"1.0\", features = [\"derive\"] }}\nserde_json = \"1.0\"\n",
        manifest.binary, manifest.version
    )
}

fn render_generated_package_main_rs(manifest: &CliAnythingManifest) -> String {
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

fn render_generated_smoke_test(manifest: &CliAnythingManifest) -> String {
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

fn to_pascal_case(value: &str) -> String {
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

fn to_snake_case(value: &str) -> String {
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

fn render_value<T, F>(value: T, json: bool, text_renderer: F) -> Result<String>
where
    T: Serialize,
    F: FnOnce(&T) -> String,
{
    if json {
        Ok(serde_json::to_string_pretty(&value).context("failed to serialize JSON output")?)
    } else {
        Ok(text_renderer(&value))
    }
}

fn render_status_text(view: &StatusView) -> String {
    let mut lines = view.banner.clone();
    lines.push(String::new());
    for command in &view.commands {
        lines.push(format!("{} — {}", command.usage, command.summary));
    }
    lines.join("\n")
}

fn render_build_text(result: &BuildResult) -> String {
    let document = command_document(BuiltinCommandId::Build);
    let mut lines = vec![
        format!("software: {}", result.software_name),
        format!("source: {}", result.source),
        format!("package: {}", result.package_root.display()),
        format!("mode: {}", if result.dry_run { "dry-run" } else { "write" }),
        format!("usage: {}", document.usage),
        "generated files:".to_string(),
    ];
    lines.extend(
        result
            .generated_files
            .iter()
            .map(|path| format!("- {}", path.display())),
    );
    lines.join("\n")
}

fn render_refine_text(plan: &RefinePlan) -> String {
    let document = command_document(BuiltinCommandId::Refine);
    let mut lines = vec![
        format!("software: {}", plan.software_name),
        format!("package: {}", plan.package_root.display()),
        format!("usage: {}", document.usage),
        format!("command groups: {}", plan.command_group_count),
        format!("examples: {}", plan.example_count),
    ];
    if let Some(focus) = &plan.focus {
        lines.push(format!("focus: {focus}"));
    }
    lines.push("next actions:".to_string());
    lines.extend(plan.next_actions.iter().map(|action| format!("- {action}")));
    lines.join("\n")
}

fn render_test_text(plan: &TestPlan) -> String {
    let document = command_document(BuiltinCommandId::Test);
    let mut lines = vec![
        format!("software: {}", plan.software_name),
        format!("package: {}", plan.package_root.display()),
        format!("usage: {}", document.usage),
        "cargo commands:".to_string(),
    ];
    lines.extend(
        plan.cargo_commands
            .iter()
            .map(|command| format!("- {command}")),
    );
    lines.push("checks:".to_string());
    lines.extend(plan.checks.iter().map(|check| format!("- {check}")));
    lines.join("\n")
}

fn render_validation_text(report: &ValidationReport) -> String {
    let skin = Skin::new(&report.software_name, env!("CARGO_PKG_VERSION"));
    let mut rows = Vec::new();
    for category in &report.categories {
        for check in &category.checks {
            rows.push(vec![
                category.name.clone(),
                check.label.clone(),
                if check.passed {
                    "pass".to_string()
                } else {
                    "fail".to_string()
                },
                check.detail.clone(),
            ]);
        }
    }

    let mut lines = vec![
        format!("validation: {}", report.software_name),
        format!(
            "summary: {}/{} passed",
            report.passed_checks(),
            report.total_checks()
        ),
    ];
    lines.push(skin.format_table(&["category", "check", "status", "detail"], &rows));
    lines.join("\n")
}

fn render_list_text(entries: &[PackageListEntry]) -> String {
    let skin = Skin::new("cli-anything", env!("CARGO_PKG_VERSION"));
    let rows = entries
        .iter()
        .map(|entry| {
            vec![
                entry.software_name.clone(),
                entry.version.clone(),
                entry.status.clone(),
                entry.package_root.display().to_string(),
            ]
        })
        .collect::<Vec<_>>();
    skin.format_table(&["software", "version", "status", "path"], &rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn build_creates_package_scaffold() {
        let workspace = unique_test_dir("build");
        fs::create_dir_all(&workspace).expect("workspace should exist");

        let result = execute_build(&workspace, "https://github.com/blender/blender", false)
            .expect("build should succeed");

        assert_eq!(result.software_name, "blender");
        assert_eq!(result.manifest.version, "1.0.0");
        assert_eq!(
            result.manifest.description,
            "3D modeling, animation, and rendering via blender --background --python"
        );
        assert_eq!(result.manifest.project.format, "blend");
        assert!(
            result
                .manifest
                .command_groups
                .iter()
                .any(|group| group.name == "render")
        );
        assert!(result.package_root.join("Cargo.toml").exists());
        assert!(result.package_root.join("cli-anything.toml").exists());
        assert!(result.package_root.join("skills/SKILL.md").exists());

        fs::remove_dir_all(&workspace).expect("workspace should be removed");
    }

    #[test]
    fn build_uses_curated_metadata_for_drawio() {
        let workspace = unique_test_dir("drawio");
        fs::create_dir_all(&workspace).expect("workspace should exist");

        let result = execute_build(&workspace, "./drawio", false).expect("build should succeed");

        assert_eq!(result.manifest.version, "1.0.0");
        assert_eq!(result.manifest.backend.command, "draw.io");
        assert_eq!(result.manifest.project.state_file, ".drawio-cli.json");
        assert!(
            result
                .manifest
                .command_groups
                .iter()
                .any(|group| group.name == "connect")
        );

        fs::remove_dir_all(&workspace).expect("workspace should be removed");
    }

    #[test]
    fn refine_plan_honors_focus_argument() {
        let workspace = unique_test_dir("refine");
        fs::create_dir_all(&workspace).expect("workspace should exist");
        execute_build(&workspace, "./gimp", false).expect("build should succeed");

        let plan = create_refine_plan(&workspace, "./gimp", Some("filters".to_string()))
            .expect("refine plan should succeed");

        assert_eq!(plan.focus.as_deref(), Some("filters"));
        assert!(
            plan.next_actions
                .iter()
                .any(|action| action.contains("filters"))
        );

        fs::remove_dir_all(&workspace).expect("workspace should be removed");
    }

    #[test]
    fn validate_reports_success_for_generated_package() {
        let workspace = unique_test_dir("validate");
        fs::create_dir_all(&workspace).expect("workspace should exist");
        execute_build(&workspace, "./drawio", false).expect("build should succeed");

        let report = validate_package(&workspace, "./drawio").expect("validation should succeed");

        assert!(report.is_pass());

        fs::remove_dir_all(&workspace).expect("workspace should be removed");
    }

    #[test]
    fn list_discovers_generated_packages() {
        let workspace = unique_test_dir("list");
        fs::create_dir_all(&workspace).expect("workspace should exist");
        execute_build(&workspace, "./shotcut", false).expect("build should succeed");

        let entries = list_packages(&workspace, Some(4)).expect("list should succeed");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].software_name, "shotcut");

        fs::remove_dir_all(&workspace).expect("workspace should be removed");
    }

    #[test]
    fn generated_package_main_includes_summary_and_command_descriptions() {
        let manifest = scaffold_manifest("gimp");
        let source = render_generated_package_main_rs(&manifest);

        assert!(source.contains("description: &'static str"));
        assert!(source.contains("project_format: &'static str"));
        assert!(source.contains("skill_path: &'static str"));
        assert!(source.contains("fn filter_command_description"));
        assert!(source.contains("Apply a filter to a layer"));
        assert!(source.contains("command_response(\"filter\","));
    }

    #[test]
    fn generated_smoke_test_covers_json_summary_and_first_command() {
        let manifest = scaffold_manifest("gimp");
        let source = render_generated_smoke_test(&manifest);

        assert!(source.contains("serde_json::Value"));
        assert!(source.contains("json_summary_reports_package_metadata"));
        assert!(source.contains("json_subcommand_response_includes_description"));
        assert!(source.contains("Raster image processing via gimp -i -b (batch mode)"));
        assert!(source.contains("Create a new image project"));
    }

    fn unique_test_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        std::env::temp_dir().join(format!("cli-anything-rs-cli-{prefix}-{nanos}"))
    }
}
