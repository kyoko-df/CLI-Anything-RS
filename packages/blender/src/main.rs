use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use cli_anything_core::{CommandResponse, PackageSummary, ResponseDetails};
use cli_anything_project::backend::{
    Backend, BackendInvocation, BackendOutcome, BackendStatus, backend_from_env,
};
use cli_anything_project::{
    ActionRecord, ProjectState, load_or_seed_state, resolve_state_file, save_state,
};
use cli_anything_repl::{DispatchOutcome, Repl, Skin};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{self, IsTerminal};
use std::sync::Arc;

const SOFTWARE: &str = "blender";
const BINARY: &str = "cli-anything-blender";
const VERSION: &str = "1.0.0";
const PROJECT_FORMAT: &str = "blend";
const BACKEND_CMD: &str = "blender";

#[derive(Debug, Parser)]
#[command(name = "cli-anything-blender")]
#[command(about = "3D modeling, animation, and rendering via blender --background --python")]
struct App {
    #[arg(long)]
    json: bool,
    #[command(subcommand)]
    action: Option<Action>,
}

#[derive(Debug, Subcommand)]
enum Action {
    Scene {
        #[command(subcommand)]
        command: SceneCommand,
    },
    Object {
        #[command(subcommand)]
        command: ObjectCommand,
    },
    Material {
        #[command(subcommand)]
        command: MaterialCommand,
    },
    Modifier {
        #[command(subcommand)]
        command: ModifierCommand,
    },
    Camera {
        #[command(subcommand)]
        command: CameraCommand,
    },
    Light {
        #[command(subcommand)]
        command: LightCommand,
    },
    Animation {
        #[command(subcommand)]
        command: AnimationCommand,
    },
    Render {
        #[command(subcommand)]
        command: RenderCommand,
    },
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
}

#[derive(Debug, Subcommand)]
enum SceneCommand {
    New {
        #[arg(long, default_value = "untitled")]
        name: String,
        #[arg(long, default_value_t = 1)]
        start_frame: u32,
        #[arg(long, default_value_t = 250)]
        end_frame: u32,
    },
    Info,
}

#[derive(Debug, Subcommand)]
enum ObjectCommand {
    Add {
        #[arg(long)]
        kind: String,
        #[arg(long, default_value = "default")]
        name: String,
    },
    List,
}

#[derive(Debug, Subcommand)]
enum MaterialCommand {
    Assign {
        #[arg(long)]
        object: String,
        #[arg(long)]
        material: String,
    },
    List,
}

#[derive(Debug, Subcommand)]
enum ModifierCommand {
    Add {
        #[arg(long)]
        object: String,
        #[arg(long)]
        kind: String,
    },
    Apply {
        #[arg(long)]
        object: String,
        #[arg(long)]
        modifier: String,
    },
}

#[derive(Debug, Subcommand)]
enum CameraCommand {
    Add {
        #[arg(long, default_value = "Camera")]
        name: String,
        #[arg(long, default_value_t = 50.0)]
        focal_length: f32,
    },
    List,
}

#[derive(Debug, Subcommand)]
enum LightCommand {
    Add {
        #[arg(long)]
        kind: String,
        #[arg(long, default_value_t = 1000.0)]
        energy: f32,
    },
    List,
}

#[derive(Debug, Subcommand)]
enum AnimationCommand {
    Keyframe {
        #[arg(long)]
        object: String,
        #[arg(long, default_value_t = 1)]
        frame: u32,
    },
    Playblast {
        #[arg(long, default_value_t = 1)]
        start_frame: u32,
        #[arg(long, default_value_t = 250)]
        end_frame: u32,
    },
}

#[derive(Debug, Subcommand)]
enum RenderCommand {
    Frame {
        #[arg(long, default_value_t = 1)]
        frame: u32,
        #[arg(long, default_value = "png")]
        format: String,
    },
    Info,
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
    let backend = backend_from_env();
    let skin = Skin::new(SOFTWARE, VERSION).with_skill_path("skills/SKILL.md");

    match app.action {
        Some(action) => {
            let response = execute(action, &mut state, backend.as_ref());
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
            run_repl(skin, state, state_path, backend)?;
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

fn run_repl(
    skin: Skin,
    mut state: ProjectState,
    state_path: std::path::PathBuf,
    backend: Arc<dyn Backend>,
) -> Result<()> {
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
                    let response = execute(action, &mut state, backend.as_ref());
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
                    "enter a subcommand (scene/object/render/...); type 'help' for builtins"
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

fn execute(action: Action, state: &mut ProjectState, backend: &dyn Backend) -> CommandResponse {
    let response = match action {
        Action::Scene { command } => record(scene_response(command), state),
        Action::Object { command } => record(object_response(command), state),
        Action::Material { command } => record(material_response(command), state),
        Action::Modifier { command } => record(modifier_response(command), state),
        Action::Camera { command } => record(camera_response(command), state),
        Action::Light { command } => record(light_response(command), state),
        Action::Animation { command } => record(animation_response(command), state),
        Action::Render { command } => record(render_response(command, backend), state),
        Action::Session { command } => session_response(command, state),
    };
    stamp_backend(response, backend)
}

fn stamp_backend(mut response: CommandResponse, backend: &dyn Backend) -> CommandResponse {
    response
        .details
        .insert("backend".to_string(), json!(backend.name()));
    response
}

fn outcome_to_json(outcome: &BackendOutcome) -> Value {
    let status = match outcome.status {
        BackendStatus::DryRun => "dry-run",
        BackendStatus::Success => "success",
        BackendStatus::Failed => "failed",
    };
    json!({
        "program": outcome.invocation.program,
        "args": outcome.invocation.args,
        "label": outcome.invocation.label,
        "status": status,
        "exit_code": outcome.exit_code,
    })
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
    if response.group == "scene"
        && response.command == "new"
        && let Some(Value::Object(scene)) = response.details.get("scene")
        && let Some(Value::String(name)) = scene.get("name")
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
        description: "3D modeling, animation, and rendering via blender --background --python"
            .to_string(),
        project_format: PROJECT_FORMAT.to_string(),
        skill_path: "packages/blender/skills/SKILL.md".to_string(),
        command_groups: [
            "scene",
            "object",
            "material",
            "modifier",
            "camera",
            "light",
            "animation",
            "render",
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

fn scene_response(command: SceneCommand) -> CommandResponse {
    let command_name = scene_command_name(&command);
    let description = scene_command_description(&command);

    match command {
        SceneCommand::New {
            name,
            start_frame,
            end_frame,
        } => {
            let mut details = BTreeMap::new();
            details.insert(
                "scene".to_string(),
                json!({
                    "name": name,
                    "start_frame": start_frame,
                    "end_frame": end_frame,
                    "engine": "CYCLES",
                    "unit_system": "METRIC"
                }),
            );
            command_response_with_details("scene", command_name, description, details)
        }
        SceneCommand::Info => {
            let mut details = BTreeMap::new();
            details.insert(
                "scene".to_string(),
                json!({
                    "name": "Scene",
                    "engine": "CYCLES",
                    "frame_rate": 24,
                    "active_camera": "Camera"
                }),
            );
            command_response_with_details("scene", command_name, description, details)
        }
    }
}

fn object_response(command: ObjectCommand) -> CommandResponse {
    let command_name = object_command_name(&command);
    let description = object_command_description(&command);

    match command {
        ObjectCommand::Add { kind, name } => {
            let mut details = BTreeMap::new();
            details.insert(
                "object".to_string(),
                json!({
                    "kind": kind,
                    "name": name,
                    "location": { "x": 0, "y": 0, "z": 0 }
                }),
            );
            command_response_with_details("object", command_name, description, details)
        }
        ObjectCommand::List => {
            let mut details = BTreeMap::new();
            details.insert("object_count".to_string(), json!(3));
            details.insert(
                "objects".to_string(),
                json!([
                    { "name": "Cube", "kind": "mesh" },
                    { "name": "Camera", "kind": "camera" },
                    { "name": "Light", "kind": "light" }
                ]),
            );
            command_response_with_details("object", command_name, description, details)
        }
    }
}

fn material_response(command: MaterialCommand) -> CommandResponse {
    let command_name = material_command_name(&command);
    let description = material_command_description(&command);

    match command {
        MaterialCommand::Assign { object, material } => {
            let mut details = BTreeMap::new();
            details.insert(
                "assignment".to_string(),
                json!({
                    "object": object,
                    "material": material,
                    "slot": 0
                }),
            );
            command_response_with_details("material", command_name, description, details)
        }
        MaterialCommand::List => {
            let mut details = BTreeMap::new();
            details.insert("material_count".to_string(), json!(3));
            details.insert(
                "materials".to_string(),
                json!([
                    { "name": "Default", "shader": "Principled BSDF" },
                    { "name": "Metal", "shader": "Principled BSDF" },
                    { "name": "Glass", "shader": "Glass BSDF" }
                ]),
            );
            command_response_with_details("material", command_name, description, details)
        }
    }
}

fn modifier_response(command: ModifierCommand) -> CommandResponse {
    let command_name = modifier_command_name(&command);
    let description = modifier_command_description(&command);

    match command {
        ModifierCommand::Add { object, kind } => {
            let mut details = BTreeMap::new();
            details.insert(
                "modifier".to_string(),
                json!({
                    "object": object,
                    "kind": kind,
                    "status": "queued"
                }),
            );
            command_response_with_details("modifier", command_name, description, details)
        }
        ModifierCommand::Apply { object, modifier } => {
            let mut details = BTreeMap::new();
            details.insert(
                "application".to_string(),
                json!({
                    "object": object,
                    "modifier": modifier,
                    "status": "applied"
                }),
            );
            command_response_with_details("modifier", command_name, description, details)
        }
    }
}

fn camera_response(command: CameraCommand) -> CommandResponse {
    let command_name = camera_command_name(&command);
    let description = camera_command_description(&command);

    match command {
        CameraCommand::Add { name, focal_length } => {
            let mut details = BTreeMap::new();
            details.insert(
                "camera".to_string(),
                json!({
                    "name": name,
                    "focal_length": focal_length,
                    "sensor": "35mm"
                }),
            );
            command_response_with_details("camera", command_name, description, details)
        }
        CameraCommand::List => {
            let mut details = BTreeMap::new();
            details.insert("camera_count".to_string(), json!(1));
            details.insert(
                "cameras".to_string(),
                json!([{ "name": "Camera", "focal_length": 50.0, "active": true }]),
            );
            command_response_with_details("camera", command_name, description, details)
        }
    }
}

fn light_response(command: LightCommand) -> CommandResponse {
    let command_name = light_command_name(&command);
    let description = light_command_description(&command);

    match command {
        LightCommand::Add { kind, energy } => {
            let mut details = BTreeMap::new();
            details.insert(
                "light".to_string(),
                json!({
                    "kind": kind,
                    "energy": energy,
                    "color": "#ffffff"
                }),
            );
            command_response_with_details("light", command_name, description, details)
        }
        LightCommand::List => {
            let mut details = BTreeMap::new();
            details.insert("light_count".to_string(), json!(2));
            details.insert(
                "lights".to_string(),
                json!([
                    { "name": "Key", "kind": "area", "energy": 1000.0 },
                    { "name": "Fill", "kind": "point", "energy": 200.0 }
                ]),
            );
            command_response_with_details("light", command_name, description, details)
        }
    }
}

fn animation_response(command: AnimationCommand) -> CommandResponse {
    let command_name = animation_command_name(&command);
    let description = animation_command_description(&command);

    match command {
        AnimationCommand::Keyframe { object, frame } => {
            let mut details = BTreeMap::new();
            details.insert(
                "keyframe".to_string(),
                json!({
                    "object": object,
                    "frame": frame,
                    "channels": ["location", "rotation", "scale"]
                }),
            );
            command_response_with_details("animation", command_name, description, details)
        }
        AnimationCommand::Playblast {
            start_frame,
            end_frame,
        } => {
            let mut details = BTreeMap::new();
            details.insert(
                "playblast".to_string(),
                json!({
                    "start_frame": start_frame,
                    "end_frame": end_frame,
                    "format": "mp4",
                    "status": "queued"
                }),
            );
            command_response_with_details("animation", command_name, description, details)
        }
    }
}

fn render_response(command: RenderCommand, backend: &dyn Backend) -> CommandResponse {
    let command_name = render_command_name(&command);
    let description = render_command_description(&command);

    match command {
        RenderCommand::Frame { frame, format } => {
            let invocation = BackendInvocation::new(
                BACKEND_CMD,
                vec![
                    "--background".to_string(),
                    "--render-frame".to_string(),
                    frame.to_string(),
                    "--render-format".to_string(),
                    format.clone(),
                ],
                "render-frame",
            );
            let outcome = backend
                .execute(invocation)
                .unwrap_or_else(|err| BackendOutcome {
                    invocation: BackendInvocation::new(BACKEND_CMD, Vec::new(), "render-frame"),
                    status: BackendStatus::Failed,
                    stdout: String::new(),
                    stderr: err.to_string(),
                    exit_code: None,
                });
            let mut details = BTreeMap::new();
            details.insert(
                "render".to_string(),
                json!({
                    "frame": frame,
                    "format": format,
                    "engine": "CYCLES",
                    "status": "queued"
                }),
            );
            details.insert("invocation".to_string(), outcome_to_json(&outcome));
            command_response_with_details("render", command_name, description, details)
        }
        RenderCommand::Info => {
            let mut details = BTreeMap::new();
            details.insert(
                "settings".to_string(),
                json!({
                    "engine": "CYCLES",
                    "resolution": { "width": 1920, "height": 1080 },
                    "samples": 64,
                    "device": "GPU"
                }),
            );
            command_response_with_details("render", command_name, description, details)
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

fn scene_command_name(command: &SceneCommand) -> &'static str {
    match command {
        SceneCommand::New { .. } => "new",
        SceneCommand::Info => "info",
    }
}

fn object_command_name(command: &ObjectCommand) -> &'static str {
    match command {
        ObjectCommand::Add { .. } => "add",
        ObjectCommand::List => "list",
    }
}

fn material_command_name(command: &MaterialCommand) -> &'static str {
    match command {
        MaterialCommand::Assign { .. } => "assign",
        MaterialCommand::List => "list",
    }
}

fn modifier_command_name(command: &ModifierCommand) -> &'static str {
    match command {
        ModifierCommand::Add { .. } => "add",
        ModifierCommand::Apply { .. } => "apply",
    }
}

fn camera_command_name(command: &CameraCommand) -> &'static str {
    match command {
        CameraCommand::Add { .. } => "add",
        CameraCommand::List => "list",
    }
}

fn light_command_name(command: &LightCommand) -> &'static str {
    match command {
        LightCommand::Add { .. } => "add",
        LightCommand::List => "list",
    }
}

fn animation_command_name(command: &AnimationCommand) -> &'static str {
    match command {
        AnimationCommand::Keyframe { .. } => "keyframe",
        AnimationCommand::Playblast { .. } => "playblast",
    }
}

fn render_command_name(command: &RenderCommand) -> &'static str {
    match command {
        RenderCommand::Frame { .. } => "frame",
        RenderCommand::Info => "info",
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

fn scene_command_description(command: &SceneCommand) -> &'static str {
    match command {
        SceneCommand::New { .. } => "Create a new scene",
        SceneCommand::Info => "Inspect the active scene",
    }
}

fn object_command_description(command: &ObjectCommand) -> &'static str {
    match command {
        ObjectCommand::Add { .. } => "Add a new object",
        ObjectCommand::List => "List scene objects",
    }
}

fn material_command_description(command: &MaterialCommand) -> &'static str {
    match command {
        MaterialCommand::Assign { .. } => "Assign a material",
        MaterialCommand::List => "List materials",
    }
}

fn modifier_command_description(command: &ModifierCommand) -> &'static str {
    match command {
        ModifierCommand::Add { .. } => "Add a modifier",
        ModifierCommand::Apply { .. } => "Apply a modifier",
    }
}

fn camera_command_description(command: &CameraCommand) -> &'static str {
    match command {
        CameraCommand::Add { .. } => "Add a camera",
        CameraCommand::List => "List cameras",
    }
}

fn light_command_description(command: &LightCommand) -> &'static str {
    match command {
        LightCommand::Add { .. } => "Add a light",
        LightCommand::List => "List lights",
    }
}

fn animation_command_description(command: &AnimationCommand) -> &'static str {
    match command {
        AnimationCommand::Keyframe { .. } => "Insert a keyframe",
        AnimationCommand::Playblast { .. } => "Preview the animation",
    }
}

fn render_command_description(command: &RenderCommand) -> &'static str {
    match command {
        RenderCommand::Frame { .. } => "Render a frame",
        RenderCommand::Info => "Inspect render settings",
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
