use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use cli_anything_core::{
    BuiltinCommandId, CliAnythingManifest, ValidationCategory, ValidationCheck, ValidationReport,
    builtin_command_documents, command_document, load_manifest_from_path, package_layout,
    parse_source_target,
};
use cli_anything_generator::{GeneratedPackage, generate_package};
use cli_anything_integrations::{IntegrationOutput, render_all_integrations};
use cli_anything_repl::Skin;
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
    Init {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Build {
        source: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        emit_integrations: bool,
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
struct InitResult {
    workspace_root: PathBuf,
    created: bool,
    files: Vec<PathBuf>,
    directories: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct BuildResult {
    software_name: String,
    source: String,
    dry_run: bool,
    package_root: PathBuf,
    generated_files: Vec<PathBuf>,
    integrations: Vec<IntegrationSummary>,
    manifest: CliAnythingManifest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct IntegrationSummary {
    target: String,
    path: PathBuf,
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
        Action::Init { path, json } => {
            let result = execute_init(&path)?;
            render_value(result, json, render_init_text)?
        }
        Action::Build {
            source,
            dry_run,
            json,
            emit_integrations,
        } => {
            let result = execute_build(&workspace_root, &source, dry_run, emit_integrations)?;
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

const INIT_WORKSPACE_CARGO_TOML: &str = r#"[workspace]
members = [
    "crates/*",
    "packages/*",
]
resolver = "3"

[workspace.package]
edition = "2024"
version = "0.1.0"
license = "MIT"
authors = ["cli-anything contributors"]

[workspace.dependencies]
anyhow = "1.0"
clap = { version = "4.5", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"
"#;

const INIT_README: &str = r#"# cli-anything workspace

This workspace was scaffolded with `cli-anything init`. Generated packages live
under `packages/<software>/`; shared framework crates live under `crates/`.

## Next steps

- Run `cli-anything build <software-path-or-repo>` to scaffold a package.
- Run `cli-anything list` to see what has been generated so far.
- Run `cli-anything validate <software>` to verify the package layout.
"#;

const INIT_GITIGNORE: &str = "/target\n";

fn execute_init(path: &Path) -> Result<InitResult> {
    let already_initialized = path.join("Cargo.toml").exists();
    let directories = vec![
        path.to_path_buf(),
        path.join("crates"),
        path.join("packages"),
    ];
    let files = vec![
        path.join("Cargo.toml"),
        path.join("README.md"),
        path.join(".gitignore"),
    ];

    for dir in &directories {
        fs::create_dir_all(dir).with_context(|| format!("failed to create {}", dir.display()))?;
    }

    if !already_initialized {
        fs::write(&files[0], INIT_WORKSPACE_CARGO_TOML)
            .with_context(|| format!("failed to write {}", files[0].display()))?;
    }
    if !files[1].exists() {
        fs::write(&files[1], INIT_README)
            .with_context(|| format!("failed to write {}", files[1].display()))?;
    }
    if !files[2].exists() {
        fs::write(&files[2], INIT_GITIGNORE)
            .with_context(|| format!("failed to write {}", files[2].display()))?;
    }

    Ok(InitResult {
        workspace_root: path.to_path_buf(),
        created: !already_initialized,
        files,
        directories,
    })
}

fn execute_build(
    workspace_root: &Path,
    source: &str,
    dry_run: bool,
    emit_integrations: bool,
) -> Result<BuildResult> {
    let target = parse_source_target(source)?;
    let GeneratedPackage {
        manifest,
        layout,
        mut files,
    } = generate_package(workspace_root, &target.software_name, dry_run)?;

    let integrations = if emit_integrations {
        let outputs = render_all_integrations(&manifest);
        write_integration_outputs(&layout.root, &outputs, dry_run)?;
        for output in &outputs {
            files.push(layout.root.join("integrations").join(&output.filename));
        }
        outputs
            .iter()
            .map(|output| IntegrationSummary {
                target: output.target.id().to_string(),
                path: layout.root.join("integrations").join(&output.filename),
            })
            .collect()
    } else {
        Vec::new()
    };

    Ok(BuildResult {
        software_name: target.software_name,
        source: source.to_string(),
        dry_run,
        package_root: layout.root,
        generated_files: files,
        integrations,
        manifest,
    })
}

fn write_integration_outputs(
    package_root: &Path,
    outputs: &[IntegrationOutput],
    dry_run: bool,
) -> Result<()> {
    if dry_run {
        return Ok(());
    }
    let integrations_dir = package_root.join("integrations");
    fs::create_dir_all(&integrations_dir)
        .with_context(|| format!("failed to create {}", integrations_dir.display()))?;
    for output in outputs {
        let path = integrations_dir.join(&output.filename);
        fs::write(&path, &output.content)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }
    Ok(())
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

fn render_init_text(result: &InitResult) -> String {
    let mut lines = vec![
        format!("workspace: {}", result.workspace_root.display()),
        format!(
            "status: {}",
            if result.created {
                "initialized"
            } else {
                "already-initialized"
            }
        ),
        "directories:".to_string(),
    ];
    lines.extend(
        result
            .directories
            .iter()
            .map(|path| format!("- {}", path.display())),
    );
    lines.push("files:".to_string());
    lines.extend(
        result
            .files
            .iter()
            .map(|path| format!("- {}", path.display())),
    );
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
    if !result.integrations.is_empty() {
        lines.push("integrations:".to_string());
        for integration in &result.integrations {
            lines.push(format!(
                "- {}: {}",
                integration.target,
                integration.path.display()
            ));
        }
    }
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

    fn unique_test_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        std::env::temp_dir().join(format!("cli-anything-rs-{prefix}-{nanos}"))
    }

    #[test]
    fn build_creates_package_scaffold() {
        let workspace = unique_test_dir("build");
        fs::create_dir_all(&workspace).expect("workspace should exist");

        let result = execute_build(
            &workspace,
            "https://github.com/blender/blender",
            false,
            false,
        )
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

        let result =
            execute_build(&workspace, "./drawio", false, false).expect("build should succeed");

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
        execute_build(&workspace, "./gimp", false, false).expect("build should succeed");

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
        execute_build(&workspace, "./drawio", false, false).expect("build should succeed");

        let report = validate_package(&workspace, "./drawio").expect("validation should succeed");

        assert!(report.is_pass());

        fs::remove_dir_all(&workspace).expect("workspace should be removed");
    }

    #[test]
    fn list_discovers_generated_packages() {
        let workspace = unique_test_dir("list");
        fs::create_dir_all(&workspace).expect("workspace should exist");
        execute_build(&workspace, "./shotcut", false, false).expect("build should succeed");

        let entries = list_packages(&workspace, Some(4)).expect("list should succeed");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].software_name, "shotcut");

        fs::remove_dir_all(&workspace).expect("workspace should be removed");
    }

    #[test]
    fn build_emit_integrations_writes_files_on_disk() {
        let workspace = unique_test_dir("emit-integrations");
        fs::create_dir_all(&workspace).expect("workspace should exist");

        let result = execute_build(&workspace, "./gimp", false, true)
            .expect("build should succeed with integrations");

        assert_eq!(result.integrations.len(), 4);
        let integrations_dir = result.package_root.join("integrations");
        assert!(integrations_dir.join("CLAUDE.md").exists());
        assert!(integrations_dir.join("opencode.md").exists());
        assert!(integrations_dir.join("codex.yaml").exists());
        assert!(integrations_dir.join("hub.md").exists());

        let claude = fs::read_to_string(integrations_dir.join("CLAUDE.md"))
            .expect("claude file should be readable");
        assert!(claude.contains("cli-anything-gimp"));

        fs::remove_dir_all(&workspace).expect("workspace should be removed");
    }

    #[test]
    fn build_without_emit_integrations_skips_integrations_directory() {
        let workspace = unique_test_dir("no-emit-integrations");
        fs::create_dir_all(&workspace).expect("workspace should exist");

        let result = execute_build(&workspace, "./gimp", false, false)
            .expect("build should succeed without integrations");

        assert!(result.integrations.is_empty());
        assert!(!result.package_root.join("integrations").exists());

        fs::remove_dir_all(&workspace).expect("workspace should be removed");
    }

    #[test]
    fn init_creates_workspace_scaffold() {
        let workspace = unique_test_dir("init");
        let result = execute_init(&workspace).expect("init should succeed");

        assert!(result.created);
        assert!(workspace.join("Cargo.toml").exists());
        assert!(workspace.join("README.md").exists());
        assert!(workspace.join(".gitignore").exists());
        assert!(workspace.join("crates").is_dir());
        assert!(workspace.join("packages").is_dir());

        fs::remove_dir_all(&workspace).expect("workspace should be removed");
    }

    #[test]
    fn init_is_idempotent_on_existing_workspace() {
        let workspace = unique_test_dir("init-idem");
        execute_init(&workspace).expect("first init should succeed");
        let second = execute_init(&workspace).expect("second init should succeed");

        assert!(!second.created);
        assert!(workspace.join("Cargo.toml").exists());

        fs::remove_dir_all(&workspace).expect("workspace should be removed");
    }
}
