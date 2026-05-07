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
        "[package]\nname = \"{}\"\nversion = \"{}\"\nedition = \"2024\"\n\n[dependencies]\nanyhow = \"1.0\"\nclap = {{ version = \"4.5\", features = [\"derive\"] }}\ncli-anything-core = {{ path = \"../../crates/cli-anything-core\" }}\ncli-anything-project = {{ path = \"../../crates/cli-anything-project\" }}\ncli-anything-repl = {{ path = \"../../crates/cli-anything-repl\" }}\nserde = {{ version = \"1.0\", features = [\"derive\"] }}\nserde_json = \"1.0\"\n",
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
    let response_functions = manifest
        .command_groups
        .iter()
        .filter(|group| group.name != "session")
        .map(|group| render_group_response_fn(group))
        .collect::<Vec<_>>()
        .join("\n\n");
    let execute_match_arms = manifest
        .command_groups
        .iter()
        .map(|group| {
            let group_type = to_pascal_case(&group.name);
            let snake = to_snake_case(&group.name);
            if group.name == "session" {
                format!(
                    "        Action::{group_type} {{ command }} => session_response(command, state),\n"
                )
            } else if group.name == "export" {
                format!(
                    "        Action::{group_type} {{ command }} => record({snake}_response(command, backend), state),\n"
                )
            } else {
                format!(
                    "        Action::{group_type} {{ command }} => record({snake}_response(command), state),\n"
                )
            }
        })
        .collect::<String>();
    let command_groups = manifest
        .command_groups
        .iter()
        .map(|group| rust_string(&group.name))
        .collect::<Vec<_>>()
        .join(", ");
    let session_helpers = if manifest
        .command_groups
        .iter()
        .any(|group| group.name == "session")
    {
        render_session_helpers()
    } else {
        String::new()
    };

    let mut source = String::new();
    source.push_str("use anyhow::{Context, Result};\n");
    source.push_str("use clap::{Parser, Subcommand};\n");
    source.push_str("use cli_anything_core::{CommandResponse, PackageSummary, ResponseDetails};\n");
    source.push_str(
        "use cli_anything_project::backend::{\n    Backend, BackendInvocation, BackendOutcome, BackendStatus, backend_from_env,\n};\n",
    );
    source.push_str(
        "use cli_anything_project::{\n    ActionRecord, ProjectState, load_or_seed_state, resolve_state_file, save_state,\n};\n",
    );
    source.push_str("use cli_anything_repl::{DispatchOutcome, Repl, Skin};\n");
    source.push_str("use serde_json::{Value, json};\n");
    source.push_str("use std::collections::BTreeMap;\n");
    source.push_str("use std::io::{self, IsTerminal};\n");
    source.push_str("use std::sync::Arc;\n\n");
    source.push_str(&format!(
        "const SOFTWARE: &str = {};\n",
        rust_string(&manifest.name)
    ));
    source.push_str(&format!(
        "const BINARY: &str = {};\n",
        rust_string(&manifest.binary)
    ));
    source.push_str(&format!(
        "const VERSION: &str = {};\n",
        rust_string(&manifest.version)
    ));
    source.push_str(&format!(
        "const PROJECT_FORMAT: &str = {};\n",
        rust_string(&manifest.project.format)
    ));
    source.push_str(&format!(
        "const BACKEND_CMD: &str = {};\n\n",
        rust_string(&manifest.backend.command)
    ));
    source.push_str("#[derive(Debug, Parser)]\n");
    source.push_str(&format!(
        "#[command(name = {})]\n",
        rust_string(&manifest.binary)
    ));
    source.push_str(&format!(
        "#[command(about = {})]\n",
        rust_string(&manifest.description)
    ));
    source.push_str("struct App {\n");
    source.push_str("    #[arg(long)]\n");
    source.push_str("    json: bool,\n");
    source.push_str("    #[command(subcommand)]\n");
    source.push_str("    action: Option<Action>,\n");
    source.push_str("}\n\n");
    source.push_str("#[derive(Debug, Subcommand)]\n");
    source.push_str("enum Action {\n");
    source.push_str(&action_variants);
    source.push_str("}\n\n");
    source.push_str(&command_enums);
    source.push_str("\n\n");
    source.push_str("fn main() -> Result<()> {\n");
    source.push_str("    let app = App::parse();\n");
    source.push_str("    let state_path = resolve_state_file(SOFTWARE);\n");
    source.push_str(
        "    let mut state = load_or_seed_state(&state_path, SOFTWARE, BINARY, PROJECT_FORMAT)\n",
    );
    source.push_str(
        "        .with_context(|| format!(\"failed to load state from {}\", state_path.display()))?;\n",
    );
    source.push_str("    let backend = backend_from_env();\n");
    source.push_str(
        "    let skin = Skin::new(SOFTWARE, VERSION).with_skill_path(\"skills/SKILL.md\");\n\n",
    );
    source.push_str("    match app.action {\n");
    source.push_str("        Some(action) => {\n");
    source.push_str("            let response = execute(action, &mut state, backend.as_ref());\n");
    source.push_str(
        "            save_state(&state_path, &state)\n                .with_context(|| format!(\"failed to save state to {}\", state_path.display()))?;\n",
    );
    source.push_str("            print_response(&skin, &response, app.json);\n");
    source.push_str("        }\n");
    source.push_str("        None if app.json => {\n");
    source.push_str(
        "            println!(\"{}\", serde_json::to_string_pretty(&package_summary()).expect(\"package summary should serialize\"));\n",
    );
    source.push_str("        }\n");
    source.push_str("        None if io::stdin().is_terminal() => {\n");
    source.push_str("            run_repl(skin, state, state_path, backend)?;\n");
    source.push_str("        }\n");
    source.push_str("        None => {\n");
    source.push_str("            let summary = package_summary();\n");
    source.push_str("            for line in skin.banner_lines() {\n");
    source.push_str("                println!(\"{line}\");\n");
    source.push_str("            }\n");
    source.push_str("            println!(\"{}\", skin.status(\"binary\", BINARY));\n");
    source.push_str("            println!(\"{}\", skin.status(\"format\", PROJECT_FORMAT));\n");
    source.push_str(
        "            println!(\"{}\", skin.status(\"groups\", &summary.command_groups.join(\", \")));\n",
    );
    source.push_str("        }\n");
    source.push_str("    }\n\n");
    source.push_str("    Ok(())\n");
    source.push_str("}\n\n");
    source.push_str(
        "fn run_repl(\n    skin: Skin,\n    mut state: ProjectState,\n    state_path: std::path::PathBuf,\n    backend: Arc<dyn Backend>,\n) -> Result<()> {\n",
    );
    source.push_str(
        "    let mut repl = Repl::new(skin.clone())\n        .with_project_name(\n            state\n                .active_project\n                .clone()\n                .unwrap_or_else(|| \"(unsaved)\".to_string()),\n        )\n        .with_modified(state.dirty);\n\n",
    );
    source.push_str("    let stdin = io::stdin();\n");
    source.push_str("    let stdout = io::stdout();\n");
    source.push_str("    repl.run(stdin.lock(), stdout.lock(), |tokens| {\n");
    source.push_str("        let mut args: Vec<String> = Vec::with_capacity(tokens.len() + 1);\n");
    source.push_str("        args.push(BINARY.to_string());\n");
    source.push_str("        args.extend(tokens.iter().cloned());\n");
    source.push_str("        match App::try_parse_from(args) {\n");
    source.push_str("            Ok(parsed) => match parsed.action {\n");
    source.push_str("                Some(action) => {\n");
    source.push_str(
        "                    let response = execute(action, &mut state, backend.as_ref());\n",
    );
    source.push_str(
        "                    let rendered = serde_json::to_string_pretty(&response)\n                        .unwrap_or_else(|err| format!(\"{{\\\"error\\\":\\\"{err}\\\"}}\"));\n",
    );
    source.push_str("                    if let Err(err) = save_state(&state_path, &state) {\n");
    source.push_str(
        "                        return DispatchOutcome::Failed(format!(\"command ran but state save failed: {err}\"));\n",
    );
    source.push_str("                    }\n");
    source.push_str("                    DispatchOutcome::Rendered(rendered)\n");
    source.push_str("                }\n");
    source.push_str("                None => DispatchOutcome::Rendered(\n");
    source.push_str(
        "                    \"enter a subcommand (project/layer/canvas/...); type 'help' for builtins\"\n                        .to_string(),\n",
    );
    source.push_str("                ),\n");
    source.push_str("            },\n");
    source.push_str(
        "            Err(err) => DispatchOutcome::Failed(err.to_string().trim().to_string()),\n",
    );
    source.push_str("        }\n");
    source.push_str("    })?;\n");
    source.push_str("    Ok(())\n");
    source.push_str("}\n\n");
    source
        .push_str("fn print_response(skin: &Skin, response: &CommandResponse, as_json: bool) {\n");
    source.push_str("    if as_json {\n");
    source.push_str(
        "        println!(\"{}\", serde_json::to_string_pretty(response).expect(\"command response should serialize\"));\n",
    );
    source.push_str("    } else {\n");
    source.push_str(
        "        println!(\"{}\", skin.info(&format!(\"{} -> {}\", response.group, response.command)));\n",
    );
    source.push_str("        println!(\"{}\", skin.status(\"detail\", &response.description));\n");
    source.push_str("        if !response.details.is_empty() {\n");
    source.push_str(
        "            println!(\"{}\", serde_json::to_string_pretty(&response.details).expect(\"response details should serialize\"));\n",
    );
    source.push_str("        }\n");
    source.push_str("    }\n");
    source.push_str("}\n\n");
    source.push_str(
        "fn execute(action: Action, state: &mut ProjectState, backend: &dyn Backend) -> CommandResponse {\n",
    );
    source.push_str("    let response = match action {\n");
    source.push_str(&execute_match_arms);
    source.push_str("    };\n");
    source.push_str("    stamp_backend(response, backend)\n");
    source.push_str("}\n\n");
    source.push_str(
        "fn stamp_backend(mut response: CommandResponse, backend: &dyn Backend) -> CommandResponse {\n",
    );
    source
        .push_str("    response.details.insert(\"backend\".to_string(), json!(backend.name()));\n");
    source.push_str("    response\n");
    source.push_str("}\n\n");
    source.push_str("fn outcome_to_json(outcome: &BackendOutcome) -> Value {\n");
    source.push_str("    let status = match outcome.status {\n");
    source.push_str("        BackendStatus::DryRun => \"dry-run\",\n");
    source.push_str("        BackendStatus::Success => \"success\",\n");
    source.push_str("        BackendStatus::Failed => \"failed\",\n");
    source.push_str("    };\n");
    source.push_str("    json!({\n");
    source.push_str("        \"program\": outcome.invocation.program,\n");
    source.push_str("        \"args\": outcome.invocation.args,\n");
    source.push_str("        \"label\": outcome.invocation.label,\n");
    source.push_str("        \"command\": outcome.invocation.display_command(),\n");
    source.push_str("        \"status\": status,\n");
    source.push_str("        \"exit_code\": outcome.exit_code,\n");
    source.push_str("    })\n");
    source.push_str("}\n\n");
    source.push_str(
        "fn record(response: CommandResponse, state: &mut ProjectState) -> CommandResponse {\n",
    );
    source.push_str("    if let Some(name) = active_project_from_response(&response) {\n");
    source.push_str("        state.active_project = Some(name);\n");
    source.push_str("    }\n");
    source.push_str("    state.push_action(ActionRecord {\n");
    source.push_str("        group: response.group.to_string(),\n");
    source.push_str("        command: response.command.to_string(),\n");
    source.push_str("        description: response.description.to_string(),\n");
    source.push_str("        payload: if response.details.is_empty() {\n");
    source.push_str("            None\n");
    source.push_str("        } else {\n");
    source.push_str(
        "            Some(serde_json::to_value(&response.details).unwrap_or(Value::Null))\n",
    );
    source.push_str("        },\n");
    source.push_str("    });\n");
    source.push_str("    response\n");
    source.push_str("}\n\n");
    source.push_str(
        "fn active_project_from_response(response: &CommandResponse) -> Option<String> {\n",
    );
    source.push_str("    if response.group == \"project\"\n");
    source.push_str("        && response.command == \"new\"\n");
    source.push_str(
        "        && let Some(Value::Object(project)) = response.details.get(\"project\")\n",
    );
    source.push_str("        && let Some(Value::String(name)) = project.get(\"name\")\n");
    source.push_str("    {\n");
    source.push_str("        return Some(name.clone());\n");
    source.push_str("    }\n");
    source.push_str("    None\n");
    source.push_str("}\n\n");
    source.push_str("fn package_summary() -> PackageSummary {\n");
    source.push_str("    PackageSummary::new(\n");
    source.push_str("        SOFTWARE,\n");
    source.push_str("        BINARY,\n");
    source.push_str("        VERSION,\n");
    source.push_str(&format!(
        "        {},\n",
        rust_string(&manifest.description)
    ));
    source.push_str("        PROJECT_FORMAT,\n");
    source.push_str(&format!(
        "        {},\n",
        rust_string(&manifest.skill.output)
    ));
    source.push_str("    )\n");
    source.push_str(&format!(
        "    .with_command_groups([{command_groups}])\n",
        command_groups = command_groups
    ));
    source.push_str(&format!(
        "    .with_modes({}, {})\n",
        manifest.supports_json, manifest.repl_default
    ));
    source.push_str("}\n\n");
    source.push_str(
        "fn command_response(group: &'static str, command: &'static str, description: &'static str) -> CommandResponse {\n",
    );
    source.push_str(
        "    command_response_with_details(group, command, description, ResponseDetails::new())\n",
    );
    source.push_str("}\n\n");
    source.push_str(
        "fn command_response_with_details(\n    group: &'static str,\n    command: &'static str,\n    description: &'static str,\n    details: ResponseDetails,\n) -> CommandResponse {\n",
    );
    source.push_str(
        "    CommandResponse::new(SOFTWARE, BINARY, group, command, description).with_details(details)\n",
    );
    source.push_str("}\n\n");
    if !response_functions.is_empty() {
        source.push_str(&response_functions);
        source.push_str("\n\n");
    }
    if !session_helpers.is_empty() {
        source.push_str(&session_helpers);
        source.push_str("\n\n");
    }
    source.push_str(&command_name_fns);
    source.push_str("\n\n");
    source.push_str(&command_description_fns);
    source
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
    let rich_contracts = has_command(manifest, "project", "new")
        && has_command(manifest, "session", "status")
        && has_command(manifest, "session", "undo")
        && has_command(manifest, "session", "redo")
        && has_command(manifest, "session", "save")
        && has_command(manifest, "export", "image");

    let mut source = String::new();
    source.push_str("use std::path::{Path, PathBuf};\n");
    source.push_str("use std::process::Command;\n");
    source.push_str("use std::sync::atomic::{AtomicU64, Ordering};\n");
    source.push_str("use std::time::{SystemTime, UNIX_EPOCH};\n\n");
    source.push_str("use serde_json::Value;\n\n");
    source.push_str("static COUNTER: AtomicU64 = AtomicU64::new(0);\n\n");
    source.push_str("fn unique_state_file(label: &str) -> PathBuf {\n");
    source.push_str(
        "    let nanos = SystemTime::now()\n        .duration_since(UNIX_EPOCH)\n        .expect(\"system time should be valid\")\n        .as_nanos();\n",
    );
    source.push_str("    let bump = COUNTER.fetch_add(1, Ordering::Relaxed);\n");
    source.push_str(
        "    std::env::temp_dir().join(format!(\"cli-anything-rs-generated-{label}-{nanos}-{bump}.json\"))\n",
    );
    source.push_str("}\n\n");
    source.push_str(
        "fn run_binary(args: &[&str], state_file: Option<&Path>) -> std::process::Output {\n",
    );
    source.push_str(&format!(
        "    let mut command = Command::new(env!(\"CARGO_BIN_EXE_{}\"));\n",
        manifest.binary
    ));
    source.push_str("    command.args(args);\n");
    source.push_str("    if let Some(path) = state_file {\n");
    source.push_str("        command.env(\"CLI_ANYTHING_STATE_FILE\", path);\n");
    source.push_str("    }\n");
    source.push_str("    command.output().expect(\"generated binary should run\")\n");
    source.push_str("}\n\n");
    source.push_str("fn run_json(args: &[&str], state_file: Option<&Path>) -> Value {\n");
    source.push_str("    let output = run_binary(args, state_file);\n");
    source.push_str("    assert!(\n");
    source.push_str("        output.status.success(),\n");
    source.push_str("        \"binary exited with {:?} for args {:?}\\nstderr: {}\",\n");
    source.push_str("        output.status,\n");
    source.push_str("        args,\n");
    source.push_str("        String::from_utf8_lossy(&output.stderr)\n");
    source.push_str("    );\n");
    source.push_str(
        "    serde_json::from_slice(&output.stdout).expect(\"command output should be valid json\")\n",
    );
    source.push_str("}\n\n");
    source.push_str("#[test]\nfn binary_name_is_stable() {\n");
    source.push_str(&format!(
        "    assert_eq!({}, {});\n",
        rust_string(&manifest.binary),
        rust_string(&manifest.binary)
    ));
    source.push_str("}\n\n");
    source.push_str("#[test]\nfn json_summary_reports_package_metadata() {\n");
    source.push_str("    let state_file = unique_state_file(\"summary\");\n");
    source.push_str("    let payload = run_json(&[\"--json\"], Some(&state_file));\n\n");
    source.push_str(&format!(
        "    assert_eq!(payload[\"name\"], {});\n",
        rust_string(&manifest.name)
    ));
    source.push_str(&format!(
        "    assert_eq!(payload[\"binary\"], {});\n",
        rust_string(&manifest.binary)
    ));
    source.push_str(&format!(
        "    assert_eq!(payload[\"version\"], {});\n",
        rust_string(&manifest.version)
    ));
    source.push_str(&format!(
        "    assert_eq!(payload[\"description\"], {});\n",
        rust_string(&manifest.description)
    ));
    source.push_str(&format!(
        "    assert_eq!(payload[\"project_format\"], {});\n",
        rust_string(&manifest.project.format)
    ));
    source.push_str(&format!(
        "    assert_eq!(payload[\"command_groups\"].as_array().map(Vec::len), Some({}));\n",
        command_group_count
    ));
    source.push_str("}\n\n");
    source.push_str("#[test]\nfn json_subcommand_response_includes_description() {\n");
    source.push_str("    let state_file = unique_state_file(\"subcommand\");\n");
    source.push_str(&format!(
        "    let payload = run_json(&[\"--json\", {}, {}], Some(&state_file));\n\n",
        rust_string(&first_group.name),
        rust_string(&first_command.name)
    ));
    source.push_str(&format!(
        "    assert_eq!(payload[\"software\"], {});\n",
        rust_string(&manifest.name)
    ));
    source.push_str(&format!(
        "    assert_eq!(payload[\"group\"], {});\n",
        rust_string(&first_group.name)
    ));
    source.push_str(&format!(
        "    assert_eq!(payload[\"command\"], {});\n",
        rust_string(&first_command.name)
    ));
    source.push_str(&format!(
        "    assert_eq!(payload[\"description\"], {});\n",
        rust_string(&first_command.description)
    ));
    source.push_str("}\n");
    if rich_contracts {
        source.push_str("\n#[test]\nfn project_new_persists_state_for_session_status() {\n");
        source.push_str("    let state_file = unique_state_file(\"project-state\");\n");
        source.push_str(
            "    let _ = run_json(&[\"--json\", \"project\", \"new\", \"--name\", \"demo\"], Some(&state_file));\n",
        );
        source.push_str(
            "    let payload = run_json(&[\"--json\", \"session\", \"status\"], Some(&state_file));\n",
        );
        source.push_str("    assert_eq!(payload[\"session\"][\"active_project\"], \"demo\");\n");
        source.push_str("    assert_eq!(payload[\"session\"][\"history_depth\"], 1);\n");
        source.push_str("    assert_eq!(payload[\"session\"][\"dirty\"], true);\n");
        source.push_str("}\n");
        source.push_str("\n#[test]\nfn session_undo_redo_and_save_round_trip() {\n");
        source.push_str("    let state_file = unique_state_file(\"session-roundtrip\");\n");
        source.push_str(
            "    let _ = run_json(&[\"--json\", \"project\", \"new\", \"--name\", \"demo\"], Some(&state_file));\n",
        );
        source.push_str(
            "    let undone = run_json(&[\"--json\", \"session\", \"undo\"], Some(&state_file));\n",
        );
        source.push_str("    assert_eq!(undone[\"status\"], \"undone\");\n");
        source.push_str("    assert_eq!(undone[\"undone_action\"][\"command\"], \"new\");\n");
        source.push_str(
            "    let redone = run_json(&[\"--json\", \"session\", \"redo\"], Some(&state_file));\n",
        );
        source.push_str("    assert_eq!(redone[\"status\"], \"redone\");\n");
        source.push_str("    assert_eq!(redone[\"redone_action\"][\"command\"], \"new\");\n");
        source.push_str(
            "    let saved = run_json(&[\"--json\", \"session\", \"save\"], Some(&state_file));\n",
        );
        source.push_str("    assert_eq!(saved[\"status\"], \"saved\");\n");
        source.push_str(
            "    let status = run_json(&[\"--json\", \"session\", \"status\"], Some(&state_file));\n",
        );
        source.push_str("    assert_eq!(status[\"session\"][\"dirty\"], false);\n");
        source.push_str("}\n");
        source.push_str("\n#[test]\nfn export_image_reports_dry_run_backend_invocation() {\n");
        source.push_str("    let state_file = unique_state_file(\"export-image\");\n");
        source.push_str(
            "    let payload = run_json(&[\"--json\", \"export\", \"image\"], Some(&state_file));\n",
        );
        source.push_str("    assert_eq!(payload[\"backend\"], \"dry-run\");\n");
        source.push_str("    assert_eq!(payload[\"invocation\"][\"status\"], \"dry-run\");\n");
        source.push_str("    assert!(payload[\"invocation\"][\"command\"]\n");
        source.push_str("        .as_str()\n");
        source.push_str("        .expect(\"command should be a string\")\n");
        source.push_str(&format!(
            "        .contains({}));\n",
            rust_string(&manifest.backend.command)
        ));
        source.push_str("}\n");
    }
    source
}

fn render_command_group_enum(group: &CommandGroup) -> String {
    let group_type = to_pascal_case(&group.name);
    let variants = group
        .commands
        .iter()
        .map(|command| render_command_variant(group, command))
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
                "        {group_type}Command::{} => {},\n",
                render_command_pattern(group, command),
                rust_string(&command.name)
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
                "        {group_type}Command::{} => {},\n",
                render_command_pattern(group, command),
                rust_string(&command.description)
            )
        })
        .collect::<String>();

    format!(
        "fn {function_name}(command: &{group_type}Command) -> &'static str {{\n    match command {{\n{match_arms}    }}\n}}"
    )
}

fn render_group_response_fn(group: &CommandGroup) -> String {
    let group_type = to_pascal_case(&group.name);
    let function_name = format!("{}_response", to_snake_case(&group.name));
    let command_name_fn = format!("{}_command_name", to_snake_case(&group.name));
    let command_description_fn = format!("{}_command_description", to_snake_case(&group.name));
    let signature = if group.name == "export" {
        format!(
            "fn {function_name}(command: {group_type}Command, backend: &dyn Backend) -> CommandResponse {{\n"
        )
    } else {
        format!("fn {function_name}(command: {group_type}Command) -> CommandResponse {{\n")
    };
    let match_arms = group
        .commands
        .iter()
        .map(|command| render_response_match_arm(group, command))
        .collect::<String>();

    format!(
        "{signature}    let command_name = {command_name_fn}(&command);\n    let description = {command_description_fn}(&command);\n\n    match command {{\n{match_arms}    }}\n}}"
    )
}

fn render_response_match_arm(group: &CommandGroup, command: &CommandSpec) -> String {
    let pattern = response_command_pattern(group, command);
    match (group.name.as_str(), command.name.as_str()) {
        ("project", "new") => format!(
            "        {pattern} => {{\n            let mut details = BTreeMap::new();\n            details.insert(\n                \"project\".to_string(),\n                json!({{\n                    \"name\": name,\n                    \"width\": width,\n                    \"height\": height,\n                    \"color_mode\": color_mode,\n                    \"background\": \"transparent\",\n                    \"dpi\": 300,\n                    \"layer_count\": 1\n                }}),\n            );\n            command_response_with_details(\"project\", command_name, description, details)\n        }},\n"
        ),
        ("project", "info") => format!(
            "        {pattern} => {{\n            let mut details = BTreeMap::new();\n            details.insert(\"project_format\".to_string(), json!(PROJECT_FORMAT));\n            details.insert(\n                \"default_template\".to_string(),\n                json!({{\n                    \"name\": \"default-project\",\n                    \"width\": 1920,\n                    \"height\": 1080,\n                    \"background\": \"transparent\"\n                }}),\n            );\n            command_response_with_details(\"project\", command_name, description, details)\n        }},\n"
        ),
        ("layer", "list") => format!(
            "        {pattern} => {{\n            let mut details = BTreeMap::new();\n            details.insert(\"layer_count\".to_string(), json!(3));\n            details.insert(\n                \"layers\".to_string(),\n                json!([\n                    {{ \"name\": \"Background\", \"visible\": true, \"blend_mode\": \"normal\" }},\n                    {{ \"name\": \"Foreground\", \"visible\": true, \"blend_mode\": \"normal\" }},\n                    {{ \"name\": \"Effects\", \"visible\": true, \"blend_mode\": \"screen\" }}\n                ]),\n            );\n            command_response_with_details(\"layer\", command_name, description, details)\n        }},\n"
        ),
        ("canvas", "info") => format!(
            "        {pattern} => {{\n            let mut details = BTreeMap::new();\n            details.insert(\n                \"canvas\".to_string(),\n                json!({{\n                    \"width\": 1920,\n                    \"height\": 1080,\n                    \"units\": \"px\",\n                    \"resolution\": 300\n                }}),\n            );\n            command_response_with_details(\"canvas\", command_name, description, details)\n        }},\n"
        ),
        ("canvas", "resize") => format!(
            "        {pattern} => {{\n            let mut details = BTreeMap::new();\n            details.insert(\n                \"canvas\".to_string(),\n                json!({{\n                    \"width\": width,\n                    \"height\": height,\n                    \"units\": \"px\",\n                    \"anchor\": \"center\"\n                }}),\n            );\n            command_response_with_details(\"canvas\", command_name, description, details)\n        }},\n"
        ),
        ("filter", "list") => format!(
            "        {pattern} => {{\n            let mut details = BTreeMap::new();\n            details.insert(\"filter_count\".to_string(), json!(4));\n            details.insert(\n                \"filters\".to_string(),\n                json!([\n                    {{ \"name\": \"brightness\", \"category\": \"color\" }},\n                    {{ \"name\": \"contrast\", \"category\": \"color\" }},\n                    {{ \"name\": \"gaussian-blur\", \"category\": \"blur\" }},\n                    {{ \"name\": \"unsharp-mask\", \"category\": \"sharpen\" }}\n                ]),\n            );\n            command_response_with_details(\"filter\", command_name, description, details)\n        }},\n"
        ),
        ("media", "import") => format!(
            "        {pattern} => {{\n            let mut details = BTreeMap::new();\n            details.insert(\n                \"asset\".to_string(),\n                json!({{\n                    \"path\": path,\n                    \"slot\": slot,\n                    \"kind\": \"image\",\n                    \"status\": \"queued\"\n                }}),\n            );\n            command_response_with_details(\"media\", command_name, description, details)\n        }},\n"
        ),
        ("media", "list") => format!(
            "        {pattern} => {{\n            let mut details = BTreeMap::new();\n            details.insert(\"asset_count\".to_string(), json!(2));\n            details.insert(\n                \"assets\".to_string(),\n                json!([\n                    {{ \"path\": \"fixtures/reference.png\", \"slot\": \"reference\", \"kind\": \"image\" }},\n                    {{ \"path\": \"fixtures/texture.png\", \"slot\": \"texture\", \"kind\": \"image\" }}\n                ]),\n            );\n            command_response_with_details(\"media\", command_name, description, details)\n        }},\n"
        ),
        ("export", "image") => format!(
            "        {pattern} => {{\n            let invocation = BackendInvocation::new(\n                BACKEND_CMD,\n                vec![\n                    \"-i\".to_string(),\n                    \"-b\".to_string(),\n                    \"(generated-export-command)\".to_string(),\n                ],\n                \"export-image\",\n            );\n            let outcome = backend.execute(invocation).unwrap_or_else(|err| BackendOutcome {{\n                invocation: BackendInvocation::new(BACKEND_CMD, Vec::new(), \"export-image\"),\n                status: BackendStatus::Failed,\n                stdout: String::new(),\n                stderr: err.to_string(),\n                exit_code: None,\n            }});\n            let mut details = BTreeMap::new();\n            details.insert(\"invocation\".to_string(), outcome_to_json(&outcome));\n            command_response_with_details(\"export\", command_name, description, details)\n        }},\n"
        ),
        ("export", "presets") => format!(
            "        {pattern} => {{\n            let mut details = BTreeMap::new();\n            details.insert(\"preset_count\".to_string(), json!(3));\n            details.insert(\n                \"presets\".to_string(),\n                json!([\n                    {{ \"name\": \"web-png\", \"format\": \"png\" }},\n                    {{ \"name\": \"print-jpeg\", \"format\": \"jpeg\" }},\n                    {{ \"name\": \"archive-tiff\", \"format\": \"tiff\" }}\n                ]),\n            );\n            command_response_with_details(\"export\", command_name, description, details)\n        }},\n"
        ),
        ("draw", "line") => format!(
            "        {pattern} => {{\n            let mut details = BTreeMap::new();\n            details.insert(\n                \"stroke\".to_string(),\n                json!({{\n                    \"tool\": \"paintbrush\",\n                    \"start\": {{ \"x\": x1, \"y\": y1 }},\n                    \"end\": {{ \"x\": x2, \"y\": y2 }}\n                }}),\n            );\n            command_response_with_details(\"draw\", command_name, description, details)\n        }},\n"
        ),
        ("draw", "rectangle") => format!(
            "        {pattern} => {{\n            let mut details = BTreeMap::new();\n            details.insert(\n                \"shape\".to_string(),\n                json!({{\n                    \"x\": x,\n                    \"y\": y,\n                    \"width\": width,\n                    \"height\": height,\n                    \"fill\": \"none\"\n                }}),\n            );\n            command_response_with_details(\"draw\", command_name, description, details)\n        }},\n"
        ),
        _ => format!(
            "        {pattern} => command_response({}, command_name, description),\n",
            rust_string(&group.name),
            pattern = qualified_command_pattern(group, command)
        ),
    }
}

fn render_session_helpers() -> String {
    "fn session_response(command: SessionCommand, state: &mut ProjectState) -> CommandResponse {\n    let command_name = session_command_name(&command);\n    let description = session_command_description(&command);\n\n    match command {\n        SessionCommand::Status => {\n            let mut details = BTreeMap::new();\n            details.insert(\n                \"session\".to_string(),\n                json!({\n                    \"dirty\": state.dirty,\n                    \"active_project\": state.active_project,\n                    \"history_depth\": state.history.len(),\n                    \"redo_depth\": state.redo_stack.len(),\n                }),\n            );\n            command_response_with_details(\"session\", command_name, description, details)\n        }\n        SessionCommand::Undo => {\n            let mut details = BTreeMap::new();\n            match state.undo() {\n                Some(undone) => {\n                    details.insert(\"status\".to_string(), json!(\"undone\"));\n                    details.insert(\"undone_action\".to_string(), action_to_json(&undone));\n                    details.insert(\"history_depth\".to_string(), json!(state.history.len()));\n                }\n                None => {\n                    details.insert(\"status\".to_string(), json!(\"nothing-to-undo\"));\n                    details.insert(\"history_depth\".to_string(), json!(state.history.len()));\n                }\n            }\n            command_response_with_details(\"session\", command_name, description, details)\n        }\n        SessionCommand::Redo => {\n            let mut details = BTreeMap::new();\n            match state.redo() {\n                Some(redone) => {\n                    details.insert(\"status\".to_string(), json!(\"redone\"));\n                    details.insert(\"redone_action\".to_string(), action_to_json(&redone));\n                    details.insert(\"history_depth\".to_string(), json!(state.history.len()));\n                }\n                None => {\n                    details.insert(\"status\".to_string(), json!(\"nothing-to-redo\"));\n                    details.insert(\"history_depth\".to_string(), json!(state.history.len()));\n                }\n            }\n            command_response_with_details(\"session\", command_name, description, details)\n        }\n        SessionCommand::History => {\n            let mut details = BTreeMap::new();\n            let history: Vec<Value> = state\n                .history\n                .iter()\n                .enumerate()\n                .map(|(index, record)| {\n                    json!({\n                        \"index\": index,\n                        \"group\": record.group,\n                        \"command\": record.command,\n                        \"description\": record.description,\n                    })\n                })\n                .collect();\n            details.insert(\"history_depth\".to_string(), json!(history.len()));\n            details.insert(\"history\".to_string(), Value::Array(history));\n            command_response_with_details(\"session\", command_name, description, details)\n        }\n        SessionCommand::Save => {\n            state.mark_clean();\n            let mut details = BTreeMap::new();\n            details.insert(\"status\".to_string(), json!(\"saved\"));\n            details.insert(\"history_depth\".to_string(), json!(state.history.len()));\n            command_response_with_details(\"session\", command_name, description, details)\n        }\n    }\n}\n\nfn action_to_json(action: &ActionRecord) -> Value {\n    json!({\n        \"group\": action.group,\n        \"command\": action.command,\n        \"description\": action.description,\n        \"payload\": action.payload,\n    })\n}"
        .to_string()
}

fn render_command_variant(group: &CommandGroup, command: &CommandSpec) -> String {
    let variant = to_pascal_case(&command.name);
    match command_fields(&group.name, &command.name) {
        Some(fields) => format!("    {variant} {{\n{fields}    }},\n"),
        None => format!("    {variant},\n"),
    }
}

fn render_command_pattern(group: &CommandGroup, command: &CommandSpec) -> String {
    let variant = to_pascal_case(&command.name);
    if command_fields(&group.name, &command.name).is_some() {
        format!("{variant} {{ .. }}")
    } else {
        variant
    }
}

fn qualified_command_pattern(group: &CommandGroup, command: &CommandSpec) -> String {
    let group_type = to_pascal_case(&group.name);
    format!("{group_type}Command::{}", render_command_pattern(group, command))
}

fn response_command_pattern(group: &CommandGroup, command: &CommandSpec) -> String {
    let group_type = to_pascal_case(&group.name);
    match (group.name.as_str(), command.name.as_str()) {
        ("project", "new") => {
            format!("{group_type}Command::New {{ name, width, height, color_mode }}")
        }
        ("canvas", "resize") => format!("{group_type}Command::Resize {{ width, height }}"),
        ("media", "import") => format!("{group_type}Command::Import {{ path, slot }}"),
        ("draw", "line") => format!("{group_type}Command::Line {{ x1, y1, x2, y2 }}"),
        ("draw", "rectangle") => {
            format!("{group_type}Command::Rectangle {{ x, y, width, height }}")
        }
        _ => qualified_command_pattern(group, command),
    }
}

fn command_fields(group_name: &str, command_name: &str) -> Option<&'static str> {
    match (group_name, command_name) {
        ("project", "new") => Some(
            "        #[arg(long, default_value = \"untitled\")]\n        name: String,\n        #[arg(long, default_value_t = 1920)]\n        width: u32,\n        #[arg(long, default_value_t = 1080)]\n        height: u32,\n        #[arg(long, default_value = \"RGB\")]\n        color_mode: String,\n",
        ),
        ("canvas", "resize") => Some(
            "        #[arg(long, default_value_t = 1920)]\n        width: u32,\n        #[arg(long, default_value_t = 1080)]\n        height: u32,\n",
        ),
        ("media", "import") => Some(
            "        #[arg(long)]\n        path: String,\n        #[arg(long, default_value = \"reference\")]\n        slot: String,\n",
        ),
        ("draw", "line") => Some(
            "        #[arg(long)]\n        x1: u32,\n        #[arg(long)]\n        y1: u32,\n        #[arg(long)]\n        x2: u32,\n        #[arg(long)]\n        y2: u32,\n",
        ),
        ("draw", "rectangle") => Some(
            "        #[arg(long)]\n        x: u32,\n        #[arg(long)]\n        y: u32,\n        #[arg(long)]\n        width: u32,\n        #[arg(long)]\n        height: u32,\n",
        ),
        _ => None,
    }
}

fn has_command(manifest: &CliAnythingManifest, group_name: &str, command_name: &str) -> bool {
    manifest.command_groups.iter().any(|group| {
        group.name == group_name
            && group
                .commands
                .iter()
                .any(|command| command.name == command_name)
    })
}

fn rust_string(value: &str) -> String {
    format!("{value:?}")
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
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    fn unique_test_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        std::env::temp_dir().join(format!("cli-anything-rs-generator-{prefix}-{nanos}"))
    }

    #[cfg(unix)]
    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root should resolve")
    }

    #[cfg(unix)]
    fn make_temp_workspace_with_shared_crates(prefix: &str) -> PathBuf {
        let workspace = unique_test_dir(prefix);
        fs::create_dir_all(workspace.join("packages")).expect("packages dir should exist");
        fs::write(
            workspace.join("Cargo.toml"),
            "[workspace]\nmembers = [\n    \"crates/*\",\n    \"packages/*\",\n]\nresolver = \"3\"\n\n[workspace.package]\nedition = \"2024\"\nversion = \"0.1.0\"\nlicense = \"MIT\"\nauthors = [\"cli-anything contributors\"]\n\n[workspace.dependencies]\nanyhow = \"1.0\"\nclap = { version = \"4.5\", features = [\"derive\"] }\nserde = { version = \"1.0\", features = [\"derive\"] }\nserde_json = \"1.0\"\ntoml = \"0.8\"\n",
        )
        .expect("workspace Cargo.toml should be written");
        symlink(repo_root().join("crates"), workspace.join("crates"))
            .expect("shared crates symlink should be created");
        workspace
    }

    #[cfg(unix)]
    fn assert_generated_package_smoke_tests_run(software: &str) {
        let workspace = make_temp_workspace_with_shared_crates(&format!("{software}-generated"));
        let result = generate_package(&workspace, software, false).expect("build should succeed");

        let output = Command::new("cargo")
            .arg("test")
            .arg("--manifest-path")
            .arg(&result.layout.cargo_toml)
            .arg("--test")
            .arg("smoke")
            .output()
            .expect("generated package smoke tests should run");

        assert!(
            output.status.success(),
            "generated smoke tests failed for {software}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        fs::remove_dir_all(&workspace).expect("workspace should be removed");
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

    #[cfg(unix)]
    #[test]
    fn generated_gimp_package_smoke_tests_run_in_temp_workspace() {
        assert_generated_package_smoke_tests_run("gimp");
    }

    #[cfg(unix)]
    #[test]
    fn generated_blender_package_smoke_tests_run_in_temp_workspace() {
        assert_generated_package_smoke_tests_run("blender");
    }

    #[cfg(unix)]
    #[test]
    fn generated_drawio_package_smoke_tests_run_in_temp_workspace() {
        assert_generated_package_smoke_tests_run("drawio");
    }

    #[test]
    fn generated_main_includes_summary_and_command_descriptions() {
        let manifest = scaffold_manifest("gimp");
        let source = render_package_main_rs(&manifest);

        assert!(source.contains("PackageSummary::new("));
        assert!(source.contains("PROJECT_FORMAT"));
        assert!(source.contains("Action::Project"));
        assert!(source.contains("Create a new image project"));
    }

    #[test]
    fn generated_main_includes_runtime_contract_for_curated_gimp() {
        let manifest = scaffold_manifest("gimp");
        let source = render_package_main_rs(&manifest);

        assert!(source.contains(
            "use cli_anything_core::{CommandResponse, PackageSummary, ResponseDetails};"
        ));
        assert!(source.contains("backend_from_env()"));
        assert!(source.contains("load_or_seed_state"));
        assert!(source.contains("save_state"));
        assert!(source.contains("fn run_repl("));
        assert!(source.contains("fn stamp_backend("));
        assert!(source.contains("enum SessionCommand"));
        assert!(source.contains("SessionCommand::Status"));
    }

    #[test]
    fn generated_package_cargo_toml_includes_shared_workspace_crates() {
        let manifest = scaffold_manifest("gimp");
        let cargo_toml = render_package_cargo_toml(&manifest);

        assert!(cargo_toml.contains("cli-anything-core"));
        assert!(cargo_toml.contains("cli-anything-project"));
        assert!(cargo_toml.contains("cli-anything-repl"));
    }

    #[test]
    fn generated_smoke_test_covers_json_summary_and_first_command() {
        let manifest = scaffold_manifest("drawio");
        let source = render_smoke_test(&manifest);

        assert!(source.contains("json_summary_reports_package_metadata"));
        assert!(source.contains("json_subcommand_response_includes_description"));
        assert!(source.contains("cli-anything-drawio"));
    }

    #[test]
    fn generated_smoke_test_covers_summary_state_session_and_backend() {
        let manifest = scaffold_manifest("gimp");
        let source = render_smoke_test(&manifest);

        assert!(source.contains("json_summary_reports_package_metadata"));
        assert!(source.contains("project_new_persists_state_for_session_status"));
        assert!(source.contains("session_undo_redo_and_save_round_trip"));
        assert!(source.contains("export_image_reports_dry_run_backend_invocation"));
    }
}
