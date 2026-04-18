use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use cli_anything_core::{CommandResponse, PackageSummary, ResponseDetails};
use cli_anything_project::{
    ActionRecord, ProjectState, load_or_seed_state, resolve_state_file, save_state,
};
use cli_anything_repl::{DispatchOutcome, Repl, Skin};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{self, IsTerminal};

const SOFTWARE: &str = "drawio";
const BINARY: &str = "cli-anything-drawio";
const VERSION: &str = "1.0.0";
const PROJECT_FORMAT: &str = "drawio";

#[derive(Debug, Parser)]
#[command(name = "cli-anything-drawio")]
#[command(about = "Diagram authoring and export via draw.io desktop CLI (-x --format)")]
struct App {
    #[arg(long)]
    json: bool,
    #[command(subcommand)]
    action: Option<Action>,
}

#[derive(Debug, Subcommand)]
enum Action {
    Diagram {
        #[command(subcommand)]
        command: DiagramCommand,
    },
    Page {
        #[command(subcommand)]
        command: PageCommand,
    },
    Shape {
        #[command(subcommand)]
        command: ShapeCommand,
    },
    Connection {
        #[command(subcommand)]
        command: ConnectionCommand,
    },
    Style {
        #[command(subcommand)]
        command: StyleCommand,
    },
    Export {
        #[command(subcommand)]
        command: ExportCommand,
    },
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
}

#[derive(Debug, Subcommand)]
enum DiagramCommand {
    New {
        #[arg(long, default_value = "untitled")]
        name: String,
        #[arg(long, default_value = "flowchart")]
        template: String,
    },
    Info,
}

#[derive(Debug, Subcommand)]
enum PageCommand {
    Add {
        #[arg(long, default_value = "Page")]
        name: String,
    },
    List,
}

#[derive(Debug, Subcommand)]
enum ShapeCommand {
    Add {
        #[arg(long)]
        kind: String,
        #[arg(long, default_value_t = 0)]
        x: i32,
        #[arg(long, default_value_t = 0)]
        y: i32,
    },
    List,
}

#[derive(Debug, Subcommand)]
enum ConnectionCommand {
    Add {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
    },
    List,
}

#[derive(Debug, Subcommand)]
enum StyleCommand {
    Apply {
        #[arg(long)]
        target: String,
        #[arg(long)]
        style: String,
    },
    List,
}

#[derive(Debug, Subcommand)]
enum ExportCommand {
    Svg {
        #[arg(long, default_value = "out.svg")]
        output: String,
    },
    Png {
        #[arg(long, default_value = "out.png")]
        output: String,
        #[arg(long, default_value_t = 300)]
        dpi: u32,
    },
    Pdf {
        #[arg(long, default_value = "out.pdf")]
        output: String,
    },
}

#[derive(Debug, Subcommand)]
enum SessionCommand {
    Status,
    Undo,
    Redo,
    History,
    Save,
}

fn main() -> Result<()> {
    let app = App::parse();
    let state_path = resolve_state_file(SOFTWARE);
    let mut state = load_or_seed_state(&state_path, SOFTWARE, BINARY, PROJECT_FORMAT)
        .with_context(|| format!("failed to load state from {}", state_path.display()))?;
    let skin = Skin::new(SOFTWARE, VERSION).with_skill_path("skills/SKILL.md");

    match app.action {
        Some(action) => {
            let response = execute(action, &mut state);
            save_state(&state_path, &state)
                .with_context(|| format!("failed to save state to {}", state_path.display()))?;
            print_response(&skin, &response, app.json);
        }
        None if app.json => {
            let summary = package_summary();
            println!(
                "{}",
                serde_json::to_string_pretty(&summary).expect("package summary should serialize")
            );
        }
        None if io::stdin().is_terminal() => {
            run_repl(skin, state, state_path)?;
        }
        None => {
            let summary = package_summary();
            for line in skin.banner_lines() {
                println!("{line}");
            }
            println!("{}", skin.status("binary", BINARY));
            println!("{}", skin.status("format", PROJECT_FORMAT));
            println!(
                "{}",
                skin.status("groups", &summary.command_groups.join(", "))
            );
        }
    }

    Ok(())
}

fn run_repl(skin: Skin, mut state: ProjectState, state_path: std::path::PathBuf) -> Result<()> {
    let mut repl = Repl::new(skin.clone())
        .with_project_name(
            state
                .active_project
                .clone()
                .unwrap_or_else(|| "(unsaved)".to_string()),
        )
        .with_modified(state.dirty);

    let stdin = io::stdin();
    let stdout = io::stdout();
    repl.run(stdin.lock(), stdout.lock(), |tokens| {
        let mut args: Vec<String> = Vec::with_capacity(tokens.len() + 1);
        args.push(BINARY.to_string());
        args.extend(tokens.iter().cloned());
        match App::try_parse_from(args) {
            Ok(parsed) => match parsed.action {
                Some(action) => {
                    let response = execute(action, &mut state);
                    let rendered = serde_json::to_string_pretty(&response)
                        .unwrap_or_else(|err| format!("{{\"error\":\"{err}\"}}"));
                    if let Err(err) = save_state(&state_path, &state) {
                        return DispatchOutcome::Failed(format!(
                            "command ran but state save failed: {err}"
                        ));
                    }
                    DispatchOutcome::Rendered(rendered)
                }
                None => DispatchOutcome::Rendered(
                    "enter a subcommand (diagram/page/shape/...); type 'help' for builtins"
                        .to_string(),
                ),
            },
            Err(err) => DispatchOutcome::Failed(err.to_string().trim().to_string()),
        }
    })?;
    Ok(())
}

fn print_response(skin: &Skin, response: &CommandResponse, as_json: bool) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(response).expect("command response should serialize")
        );
    } else {
        println!(
            "{}",
            skin.info(&format!("{} -> {}", response.group, response.command))
        );
        println!("{}", skin.status("detail", &response.description));
        if !response.details.is_empty() {
            println!(
                "{}",
                serde_json::to_string_pretty(&response.details)
                    .expect("response details should serialize")
            );
        }
    }
}

fn execute(action: Action, state: &mut ProjectState) -> CommandResponse {
    match action {
        Action::Diagram { command } => record(diagram_response(command), state),
        Action::Page { command } => record(page_response(command), state),
        Action::Shape { command } => record(shape_response(command), state),
        Action::Connection { command } => record(connection_response(command), state),
        Action::Style { command } => record(style_response(command), state),
        Action::Export { command } => record(export_response(command), state),
        Action::Session { command } => session_response(command, state),
    }
}

fn record(response: CommandResponse, state: &mut ProjectState) -> CommandResponse {
    if let Some(name) = active_project_from_response(&response) {
        state.active_project = Some(name);
    }
    state.push_action(ActionRecord {
        group: response.group.to_string(),
        command: response.command.to_string(),
        description: response.description.to_string(),
        payload: if response.details.is_empty() {
            None
        } else {
            Some(serde_json::to_value(&response.details).unwrap_or(Value::Null))
        },
    });
    response
}

fn active_project_from_response(response: &CommandResponse) -> Option<String> {
    if response.group == "diagram"
        && response.command == "new"
        && let Some(Value::Object(diagram)) = response.details.get("diagram")
        && let Some(Value::String(name)) = diagram.get("name")
    {
        return Some(name.clone());
    }
    None
}

fn package_summary() -> PackageSummary {
    PackageSummary {
        name: SOFTWARE.to_string(),
        binary: BINARY.to_string(),
        version: VERSION.to_string(),
        description: "Diagram authoring and export via draw.io desktop CLI (-x --format)"
            .to_string(),
        project_format: PROJECT_FORMAT.to_string(),
        skill_path: "packages/drawio/skills/SKILL.md".to_string(),
        command_groups: [
            "diagram",
            "page",
            "shape",
            "connection",
            "style",
            "export",
            "session",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        supports_json: true,
        repl_default: true,
    }
}

fn command_response_with_details(
    group: &'static str,
    command: &'static str,
    description: &'static str,
    details: ResponseDetails,
) -> CommandResponse {
    CommandResponse::new(SOFTWARE, BINARY, group, command, description).with_details(details)
}

fn diagram_response(command: DiagramCommand) -> CommandResponse {
    let command_name = diagram_command_name(&command);
    let description = diagram_command_description(&command);

    match command {
        DiagramCommand::New { name, template } => {
            let mut details = BTreeMap::new();
            details.insert(
                "diagram".to_string(),
                json!({
                    "name": name,
                    "template": template,
                    "page_count": 1
                }),
            );
            command_response_with_details("diagram", command_name, description, details)
        }
        DiagramCommand::Info => {
            let mut details = BTreeMap::new();
            details.insert(
                "diagram".to_string(),
                json!({
                    "name": "Untitled",
                    "template": "flowchart",
                    "page_count": 1,
                    "shape_count": 0
                }),
            );
            command_response_with_details("diagram", command_name, description, details)
        }
    }
}

fn page_response(command: PageCommand) -> CommandResponse {
    let command_name = page_command_name(&command);
    let description = page_command_description(&command);

    match command {
        PageCommand::Add { name } => {
            let mut details = BTreeMap::new();
            details.insert(
                "page".to_string(),
                json!({
                    "name": name,
                    "orientation": "landscape",
                    "size": "A4"
                }),
            );
            command_response_with_details("page", command_name, description, details)
        }
        PageCommand::List => {
            let mut details = BTreeMap::new();
            details.insert("page_count".to_string(), json!(1));
            details.insert(
                "pages".to_string(),
                json!([{ "name": "Page-1", "orientation": "landscape" }]),
            );
            command_response_with_details("page", command_name, description, details)
        }
    }
}

fn shape_response(command: ShapeCommand) -> CommandResponse {
    let command_name = shape_command_name(&command);
    let description = shape_command_description(&command);

    match command {
        ShapeCommand::Add { kind, x, y } => {
            let mut details = BTreeMap::new();
            details.insert(
                "shape".to_string(),
                json!({
                    "kind": kind,
                    "x": x,
                    "y": y,
                    "width": 120,
                    "height": 60
                }),
            );
            command_response_with_details("shape", command_name, description, details)
        }
        ShapeCommand::List => {
            let mut details = BTreeMap::new();
            details.insert("shape_count".to_string(), json!(0));
            details.insert("shapes".to_string(), json!([]));
            command_response_with_details("shape", command_name, description, details)
        }
    }
}

fn connection_response(command: ConnectionCommand) -> CommandResponse {
    let command_name = connection_command_name(&command);
    let description = connection_command_description(&command);

    match command {
        ConnectionCommand::Add { from, to } => {
            let mut details = BTreeMap::new();
            details.insert(
                "connection".to_string(),
                json!({
                    "from": from,
                    "to": to,
                    "style": "orthogonal"
                }),
            );
            command_response_with_details("connection", command_name, description, details)
        }
        ConnectionCommand::List => {
            let mut details = BTreeMap::new();
            details.insert("connection_count".to_string(), json!(0));
            details.insert("connections".to_string(), json!([]));
            command_response_with_details("connection", command_name, description, details)
        }
    }
}

fn style_response(command: StyleCommand) -> CommandResponse {
    let command_name = style_command_name(&command);
    let description = style_command_description(&command);

    match command {
        StyleCommand::Apply { target, style } => {
            let mut details = BTreeMap::new();
            details.insert(
                "application".to_string(),
                json!({
                    "target": target,
                    "style": style,
                    "status": "applied"
                }),
            );
            command_response_with_details("style", command_name, description, details)
        }
        StyleCommand::List => {
            let mut details = BTreeMap::new();
            details.insert("style_count".to_string(), json!(3));
            details.insert(
                "styles".to_string(),
                json!([
                    { "name": "default", "family": "shape" },
                    { "name": "accent", "family": "shape" },
                    { "name": "dashed", "family": "edge" }
                ]),
            );
            command_response_with_details("style", command_name, description, details)
        }
    }
}

fn export_response(command: ExportCommand) -> CommandResponse {
    let command_name = export_command_name(&command);
    let description = export_command_description(&command);

    match command {
        ExportCommand::Svg { output } => {
            let mut details = BTreeMap::new();
            details.insert(
                "export".to_string(),
                json!({
                    "format": "svg",
                    "output": output,
                    "status": "queued"
                }),
            );
            command_response_with_details("export", command_name, description, details)
        }
        ExportCommand::Png { output, dpi } => {
            let mut details = BTreeMap::new();
            details.insert(
                "export".to_string(),
                json!({
                    "format": "png",
                    "output": output,
                    "dpi": dpi,
                    "status": "queued"
                }),
            );
            command_response_with_details("export", command_name, description, details)
        }
        ExportCommand::Pdf { output } => {
            let mut details = BTreeMap::new();
            details.insert(
                "export".to_string(),
                json!({
                    "format": "pdf",
                    "output": output,
                    "status": "queued"
                }),
            );
            command_response_with_details("export", command_name, description, details)
        }
    }
}

fn session_response(command: SessionCommand, state: &mut ProjectState) -> CommandResponse {
    let command_name = session_command_name(&command);
    let description = session_command_description(&command);

    match command {
        SessionCommand::Status => {
            let mut details = BTreeMap::new();
            details.insert(
                "session".to_string(),
                json!({
                    "dirty": state.dirty,
                    "active_project": state.active_project,
                    "history_depth": state.history.len(),
                    "redo_depth": state.redo_stack.len(),
                    "autosave": "enabled"
                }),
            );
            command_response_with_details("session", command_name, description, details)
        }
        SessionCommand::Undo => {
            let mut details = BTreeMap::new();
            match state.undo() {
                Some(undone) => {
                    details.insert("status".to_string(), json!("undone"));
                    details.insert("undone_action".to_string(), action_to_json(&undone));
                    details.insert("history_depth".to_string(), json!(state.history.len()));
                }
                None => {
                    details.insert("status".to_string(), json!("nothing-to-undo"));
                    details.insert("history_depth".to_string(), json!(0));
                }
            }
            command_response_with_details("session", command_name, description, details)
        }
        SessionCommand::Redo => {
            let mut details = BTreeMap::new();
            match state.redo() {
                Some(redone) => {
                    details.insert("status".to_string(), json!("redone"));
                    details.insert("redone_action".to_string(), action_to_json(&redone));
                    details.insert("history_depth".to_string(), json!(state.history.len()));
                }
                None => {
                    details.insert("status".to_string(), json!("nothing-to-redo"));
                    details.insert("history_depth".to_string(), json!(state.history.len()));
                }
            }
            command_response_with_details("session", command_name, description, details)
        }
        SessionCommand::History => {
            let mut details = BTreeMap::new();
            let history: Vec<Value> = state
                .history
                .iter()
                .enumerate()
                .map(|(index, record)| {
                    json!({
                        "index": index,
                        "group": record.group,
                        "command": record.command,
                        "description": record.description,
                    })
                })
                .collect();
            details.insert("history_depth".to_string(), json!(history.len()));
            details.insert("history".to_string(), Value::Array(history));
            command_response_with_details("session", command_name, description, details)
        }
        SessionCommand::Save => {
            state.mark_clean();
            let mut details = BTreeMap::new();
            details.insert("status".to_string(), json!("saved"));
            details.insert("history_depth".to_string(), json!(state.history.len()));
            command_response_with_details("session", command_name, description, details)
        }
    }
}

fn action_to_json(action: &ActionRecord) -> Value {
    json!({
        "group": action.group,
        "command": action.command,
        "description": action.description,
        "payload": action.payload,
    })
}

fn diagram_command_name(command: &DiagramCommand) -> &'static str {
    match command {
        DiagramCommand::New { .. } => "new",
        DiagramCommand::Info => "info",
    }
}

fn page_command_name(command: &PageCommand) -> &'static str {
    match command {
        PageCommand::Add { .. } => "add",
        PageCommand::List => "list",
    }
}

fn shape_command_name(command: &ShapeCommand) -> &'static str {
    match command {
        ShapeCommand::Add { .. } => "add",
        ShapeCommand::List => "list",
    }
}

fn connection_command_name(command: &ConnectionCommand) -> &'static str {
    match command {
        ConnectionCommand::Add { .. } => "add",
        ConnectionCommand::List => "list",
    }
}

fn style_command_name(command: &StyleCommand) -> &'static str {
    match command {
        StyleCommand::Apply { .. } => "apply",
        StyleCommand::List => "list",
    }
}

fn export_command_name(command: &ExportCommand) -> &'static str {
    match command {
        ExportCommand::Svg { .. } => "svg",
        ExportCommand::Png { .. } => "png",
        ExportCommand::Pdf { .. } => "pdf",
    }
}

fn session_command_name(command: &SessionCommand) -> &'static str {
    match command {
        SessionCommand::Status => "status",
        SessionCommand::Undo => "undo",
        SessionCommand::Redo => "redo",
        SessionCommand::History => "history",
        SessionCommand::Save => "save",
    }
}

fn diagram_command_description(command: &DiagramCommand) -> &'static str {
    match command {
        DiagramCommand::New { .. } => "Create a new diagram",
        DiagramCommand::Info => "Inspect the active diagram",
    }
}

fn page_command_description(command: &PageCommand) -> &'static str {
    match command {
        PageCommand::Add { .. } => "Add a page",
        PageCommand::List => "List pages",
    }
}

fn shape_command_description(command: &ShapeCommand) -> &'static str {
    match command {
        ShapeCommand::Add { .. } => "Add a shape",
        ShapeCommand::List => "List shapes",
    }
}

fn connection_command_description(command: &ConnectionCommand) -> &'static str {
    match command {
        ConnectionCommand::Add { .. } => "Connect two shapes",
        ConnectionCommand::List => "List connections",
    }
}

fn style_command_description(command: &StyleCommand) -> &'static str {
    match command {
        StyleCommand::Apply { .. } => "Apply a style",
        StyleCommand::List => "List available styles",
    }
}

fn export_command_description(command: &ExportCommand) -> &'static str {
    match command {
        ExportCommand::Svg { .. } => "Export diagram as SVG",
        ExportCommand::Png { .. } => "Export diagram as PNG",
        ExportCommand::Pdf { .. } => "Export diagram as PDF",
    }
}

fn session_command_description(command: &SessionCommand) -> &'static str {
    match command {
        SessionCommand::Status => "Show session state",
        SessionCommand::Undo => "Undo the last action",
        SessionCommand::Redo => "Redo the last undone action",
        SessionCommand::History => "List recorded actions",
        SessionCommand::Save => "Mark the session clean",
    }
}
