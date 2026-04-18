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

const SOFTWARE: &str = "gimp";
const BINARY: &str = "cli-anything-gimp";
const VERSION: &str = "1.0.0";
const PROJECT_FORMAT: &str = "xcf";

#[derive(Debug, Parser)]
#[command(name = "cli-anything-gimp")]
#[command(about = "Raster image processing via gimp -i -b (batch mode)")]
struct App {
    #[arg(long)]
    json: bool,
    #[command(subcommand)]
    action: Option<Action>,
}

#[derive(Debug, Subcommand)]
enum Action {
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    Layer {
        #[command(subcommand)]
        command: LayerCommand,
    },
    Canvas {
        #[command(subcommand)]
        command: CanvasCommand,
    },
    Filter {
        #[command(subcommand)]
        command: FilterCommand,
    },
    Media {
        #[command(subcommand)]
        command: MediaCommand,
    },
    Export {
        #[command(subcommand)]
        command: ExportCommand,
    },
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
    Draw {
        #[command(subcommand)]
        command: DrawCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ProjectCommand {
    New {
        #[arg(long, default_value = "untitled")]
        name: String,
        #[arg(long, default_value_t = 1920)]
        width: u32,
        #[arg(long, default_value_t = 1080)]
        height: u32,
        #[arg(long, default_value = "RGB")]
        color_mode: String,
    },
    Info,
}

#[derive(Debug, Subcommand)]
enum LayerCommand {
    New,
    List,
}

#[derive(Debug, Subcommand)]
enum CanvasCommand {
    Info,
    Resize {
        #[arg(long, default_value_t = 1920)]
        width: u32,
        #[arg(long, default_value_t = 1080)]
        height: u32,
    },
}

#[derive(Debug, Subcommand)]
enum FilterCommand {
    Add,
    List,
}

#[derive(Debug, Subcommand)]
enum MediaCommand {
    Import {
        #[arg(long)]
        path: String,
        #[arg(long, default_value = "reference")]
        slot: String,
    },
    List,
}

#[derive(Debug, Subcommand)]
enum ExportCommand {
    Image,
    Presets,
}

#[derive(Debug, Subcommand)]
enum SessionCommand {
    Status,
    Undo,
    Redo,
    History,
    Save,
}

#[derive(Debug, Subcommand)]
enum DrawCommand {
    Line {
        #[arg(long)]
        x1: u32,
        #[arg(long)]
        y1: u32,
        #[arg(long)]
        x2: u32,
        #[arg(long)]
        y2: u32,
    },
    Rectangle {
        #[arg(long)]
        x: u32,
        #[arg(long)]
        y: u32,
        #[arg(long)]
        width: u32,
        #[arg(long)]
        height: u32,
    },
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
                    "enter a subcommand (project/layer/canvas/...); type 'help' for builtins"
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
        Action::Project { command } => record(project_response(command), state),
        Action::Layer { command } => record(layer_response(command), state),
        Action::Canvas { command } => record(canvas_response(command), state),
        Action::Filter { command } => record(filter_response(command), state),
        Action::Media { command } => record(media_response(command), state),
        Action::Export { command } => record(export_response(command), state),
        Action::Draw { command } => record(draw_response(command), state),
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
    if response.group == "project"
        && response.command == "new"
        && let Some(Value::Object(project)) = response.details.get("project")
        && let Some(Value::String(name)) = project.get("name")
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
        description: "Raster image processing via gimp -i -b (batch mode)".to_string(),
        project_format: PROJECT_FORMAT.to_string(),
        skill_path: "packages/gimp/skills/SKILL.md".to_string(),
        command_groups: [
            "project", "layer", "canvas", "filter", "media", "export", "session", "draw",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        supports_json: true,
        repl_default: true,
    }
}

fn command_response(
    group: &'static str,
    command: &'static str,
    description: &'static str,
) -> CommandResponse {
    command_response_with_details(group, command, description, ResponseDetails::new())
}

fn command_response_with_details(
    group: &'static str,
    command: &'static str,
    description: &'static str,
    details: ResponseDetails,
) -> CommandResponse {
    CommandResponse::new(SOFTWARE, BINARY, group, command, description).with_details(details)
}

fn project_response(command: ProjectCommand) -> CommandResponse {
    let command_name = project_command_name(&command);
    let description = project_command_description(&command);

    match command {
        ProjectCommand::New {
            name,
            width,
            height,
            color_mode,
        } => {
            let mut details = BTreeMap::new();
            details.insert(
                "project".to_string(),
                json!({
                    "name": name,
                    "width": width,
                    "height": height,
                    "color_mode": color_mode,
                    "background": "transparent",
                    "dpi": 300,
                    "layer_count": 1
                }),
            );
            command_response_with_details("project", command_name, description, details)
        }
        ProjectCommand::Info => {
            let mut details = BTreeMap::new();
            details.insert("project_format".to_string(), json!("xcf"));
            details.insert(
                "default_template".to_string(),
                json!({
                    "name": "default-rgb",
                    "width": 1920,
                    "height": 1080,
                    "color_mode": "RGB",
                    "background": "white",
                    "dpi": 300
                }),
            );
            command_response_with_details("project", command_name, description, details)
        }
    }
}

fn layer_response(command: LayerCommand) -> CommandResponse {
    let command_name = layer_command_name(&command);
    let description = layer_command_description(&command);

    match command {
        LayerCommand::New => command_response("layer", command_name, description),
        LayerCommand::List => {
            let mut details = BTreeMap::new();
            details.insert("layer_count".to_string(), json!(3));
            details.insert(
                "layers".to_string(),
                json!([
                    {
                        "name": "Background",
                        "visible": true,
                        "blend_mode": "normal",
                        "opacity": 100
                    },
                    {
                        "name": "Subject",
                        "visible": true,
                        "blend_mode": "normal",
                        "opacity": 100
                    },
                    {
                        "name": "Highlights",
                        "visible": true,
                        "blend_mode": "screen",
                        "opacity": 72
                    }
                ]),
            );
            command_response_with_details("layer", command_name, description, details)
        }
    }
}

fn canvas_response(command: CanvasCommand) -> CommandResponse {
    let command_name = canvas_command_name(&command);
    let description = canvas_command_description(&command);

    match command {
        CanvasCommand::Info => {
            let mut details = BTreeMap::new();
            details.insert(
                "canvas".to_string(),
                json!({
                    "width": 1920,
                    "height": 1080,
                    "units": "px",
                    "resolution": 300,
                    "background": "white"
                }),
            );
            command_response_with_details("canvas", command_name, description, details)
        }
        CanvasCommand::Resize { width, height } => {
            let mut details = BTreeMap::new();
            details.insert(
                "canvas".to_string(),
                json!({
                    "width": width,
                    "height": height,
                    "units": "px",
                    "anchor": "center",
                    "resize_layers": false
                }),
            );
            command_response_with_details("canvas", command_name, description, details)
        }
    }
}

fn filter_response(command: FilterCommand) -> CommandResponse {
    let command_name = filter_command_name(&command);
    let description = filter_command_description(&command);

    match command {
        FilterCommand::Add => command_response("filter", command_name, description),
        FilterCommand::List => {
            let filters = json!([
                {
                    "name": "brightness",
                    "category": "color",
                    "summary": "Adjust overall image brightness"
                },
                {
                    "name": "contrast",
                    "category": "color",
                    "summary": "Increase or decrease contrast"
                },
                {
                    "name": "gaussian-blur",
                    "category": "blur",
                    "summary": "Soften pixels with gaussian blur"
                },
                {
                    "name": "unsharp-mask",
                    "category": "sharpen",
                    "summary": "Sharpen edges with unsharp masking"
                }
            ]);
            let mut details = BTreeMap::new();
            details.insert("filter_count".to_string(), json!(4));
            details.insert("filters".to_string(), filters);
            command_response_with_details("filter", command_name, description, details)
        }
    }
}

fn export_response(command: ExportCommand) -> CommandResponse {
    let command_name = export_command_name(&command);
    let description = export_command_description(&command);

    match command {
        ExportCommand::Image => command_response("export", command_name, description),
        ExportCommand::Presets => {
            let mut details = BTreeMap::new();
            details.insert("preset_count".to_string(), json!(3));
            details.insert(
                "presets".to_string(),
                json!([
                    {
                        "name": "web-png",
                        "format": "png",
                        "color_profile": "sRGB"
                    },
                    {
                        "name": "print-jpeg",
                        "format": "jpeg",
                        "color_profile": "AdobeRGB"
                    },
                    {
                        "name": "archive-tiff",
                        "format": "tiff",
                        "color_profile": "ProPhotoRGB"
                    }
                ]),
            );
            command_response_with_details("export", command_name, description, details)
        }
    }
}

fn media_response(command: MediaCommand) -> CommandResponse {
    let command_name = media_command_name(&command);
    let description = media_command_description(&command);

    match command {
        MediaCommand::Import { path, slot } => {
            let mut details = BTreeMap::new();
            details.insert(
                "asset".to_string(),
                json!({
                    "path": path,
                    "slot": slot,
                    "kind": "image",
                    "status": "queued"
                }),
            );
            command_response_with_details("media", command_name, description, details)
        }
        MediaCommand::List => {
            let mut details = BTreeMap::new();
            details.insert("asset_count".to_string(), json!(3));
            details.insert(
                "assets".to_string(),
                json!([
                    {
                        "path": "fixtures/reference.png",
                        "slot": "reference",
                        "kind": "image"
                    },
                    {
                        "path": "fixtures/texture.jpg",
                        "slot": "texture",
                        "kind": "image"
                    },
                    {
                        "path": "fixtures/mask.png",
                        "slot": "mask",
                        "kind": "image"
                    }
                ]),
            );
            command_response_with_details("media", command_name, description, details)
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

fn draw_response(command: DrawCommand) -> CommandResponse {
    let command_name = draw_command_name(&command);
    let description = draw_command_description(&command);

    match command {
        DrawCommand::Line { x1, y1, x2, y2 } => {
            let mut details = BTreeMap::new();
            details.insert(
                "stroke".to_string(),
                json!({
                    "tool": "paintbrush",
                    "start": { "x": x1, "y": y1 },
                    "end": { "x": x2, "y": y2 },
                    "color": "#000000"
                }),
            );
            command_response_with_details("draw", command_name, description, details)
        }
        DrawCommand::Rectangle {
            x,
            y,
            width,
            height,
        } => {
            let mut details = BTreeMap::new();
            details.insert(
                "shape".to_string(),
                json!({
                    "x": x,
                    "y": y,
                    "width": width,
                    "height": height,
                    "fill": "none",
                    "stroke": "#000000"
                }),
            );
            command_response_with_details("draw", command_name, description, details)
        }
    }
}

fn project_command_name(command: &ProjectCommand) -> &'static str {
    match command {
        ProjectCommand::New { .. } => "new",
        ProjectCommand::Info => "info",
    }
}

fn layer_command_name(command: &LayerCommand) -> &'static str {
    match command {
        LayerCommand::New => "new",
        LayerCommand::List => "list",
    }
}

fn canvas_command_name(command: &CanvasCommand) -> &'static str {
    match command {
        CanvasCommand::Info => "info",
        CanvasCommand::Resize { .. } => "resize",
    }
}

fn filter_command_name(command: &FilterCommand) -> &'static str {
    match command {
        FilterCommand::Add => "add",
        FilterCommand::List => "list",
    }
}

fn media_command_name(command: &MediaCommand) -> &'static str {
    match command {
        MediaCommand::Import { .. } => "import",
        MediaCommand::List => "list",
    }
}

fn export_command_name(command: &ExportCommand) -> &'static str {
    match command {
        ExportCommand::Image => "image",
        ExportCommand::Presets => "presets",
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

fn draw_command_name(command: &DrawCommand) -> &'static str {
    match command {
        DrawCommand::Line { .. } => "line",
        DrawCommand::Rectangle { .. } => "rectangle",
    }
}

fn project_command_description(command: &ProjectCommand) -> &'static str {
    match command {
        ProjectCommand::New { .. } => "Create a new image project",
        ProjectCommand::Info => "Show project information",
    }
}

fn layer_command_description(command: &LayerCommand) -> &'static str {
    match command {
        LayerCommand::New => "Create a blank layer",
        LayerCommand::List => "List project layers",
    }
}

fn canvas_command_description(command: &CanvasCommand) -> &'static str {
    match command {
        CanvasCommand::Info => "Show canvas metadata",
        CanvasCommand::Resize { .. } => "Resize the canvas",
    }
}

fn filter_command_description(command: &FilterCommand) -> &'static str {
    match command {
        FilterCommand::Add => "Apply a filter to a layer",
        FilterCommand::List => "List supported filters",
    }
}

fn media_command_description(command: &MediaCommand) -> &'static str {
    match command {
        MediaCommand::Import { .. } => "Import media into the project",
        MediaCommand::List => "List project media",
    }
}

fn export_command_description(command: &ExportCommand) -> &'static str {
    match command {
        ExportCommand::Image => "Export the current composition",
        ExportCommand::Presets => "List export presets",
    }
}

fn session_command_description(command: &SessionCommand) -> &'static str {
    match command {
        SessionCommand::Status => "Show session status",
        SessionCommand::Undo => "Undo the last action",
        SessionCommand::Redo => "Redo the last undone action",
        SessionCommand::History => "List recorded actions",
        SessionCommand::Save => "Mark the session clean",
    }
}

fn draw_command_description(command: &DrawCommand) -> &'static str {
    match command {
        DrawCommand::Line { .. } => "Draw a line",
        DrawCommand::Rectangle { .. } => "Draw a rectangle",
    }
}
