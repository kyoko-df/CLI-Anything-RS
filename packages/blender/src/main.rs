use clap::{Parser, Subcommand};
use cli_anything_repl::Skin;
use serde::Serialize;

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
    New,
    Info,
}

#[derive(Debug, Subcommand)]
enum ObjectCommand {
    Add,
    List,
}

#[derive(Debug, Subcommand)]
enum MaterialCommand {
    Assign,
    List,
}

#[derive(Debug, Subcommand)]
enum ModifierCommand {
    Add,
    Apply,
}

#[derive(Debug, Subcommand)]
enum CameraCommand {
    Add,
    List,
}

#[derive(Debug, Subcommand)]
enum LightCommand {
    Add,
    List,
}

#[derive(Debug, Subcommand)]
enum AnimationCommand {
    Keyframe,
    Playblast,
}

#[derive(Debug, Subcommand)]
enum RenderCommand {
    Frame,
    Info,
}

#[derive(Debug, Subcommand)]
enum SessionCommand {
    Status,
    History,
}

#[derive(Debug, Serialize)]
struct PackageSummary {
    name: &'static str,
    binary: &'static str,
    version: &'static str,
    description: &'static str,
    project_format: &'static str,
    skill_path: &'static str,
    command_groups: Vec<&'static str>,
    supports_json: bool,
    repl_default: bool,
}

#[derive(Debug, Serialize)]
struct CommandResponse {
    software: &'static str,
    binary: &'static str,
    group: &'static str,
    command: &'static str,
    description: &'static str,
}

fn main() {
    let app = App::parse();
    let skin = Skin::new("blender", "1.0.0").with_skill_path("skills/SKILL.md");

    match app.action {
        Some(action) => {
            let response = match action {
                Action::Scene { command } => command_response(
                    "scene",
                    scene_command_name(&command),
                    scene_command_description(&command),
                ),
                Action::Object { command } => command_response(
                    "object",
                    object_command_name(&command),
                    object_command_description(&command),
                ),
                Action::Material { command } => command_response(
                    "material",
                    material_command_name(&command),
                    material_command_description(&command),
                ),
                Action::Modifier { command } => command_response(
                    "modifier",
                    modifier_command_name(&command),
                    modifier_command_description(&command),
                ),
                Action::Camera { command } => command_response(
                    "camera",
                    camera_command_name(&command),
                    camera_command_description(&command),
                ),
                Action::Light { command } => command_response(
                    "light",
                    light_command_name(&command),
                    light_command_description(&command),
                ),
                Action::Animation { command } => command_response(
                    "animation",
                    animation_command_name(&command),
                    animation_command_description(&command),
                ),
                Action::Render { command } => command_response(
                    "render",
                    render_command_name(&command),
                    render_command_description(&command),
                ),
                Action::Session { command } => command_response(
                    "session",
                    session_command_name(&command),
                    session_command_description(&command),
                ),
            };
            if app.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&response)
                        .expect("command response should serialize")
                );
            } else {
                println!(
                    "{}",
                    skin.info(&format!("{} -> {}", response.group, response.command))
                );
                println!("{}", skin.status("detail", response.description));
            }
        }
        None => {
            let summary = package_summary();
            if app.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&summary)
                        .expect("package summary should serialize")
                );
            } else {
                for line in skin.banner_lines() {
                    println!("{line}");
                }
                println!("{}", skin.status("binary", "cli-anything-blender"));
                println!("{}", skin.status("format", "blend"));
                println!("{}", skin.status("groups", "scene, object, material, modifier, camera, light, animation, render, session"));
            }
        }
    }
}

fn package_summary() -> PackageSummary {
    PackageSummary {
        name: "blender",
        binary: "cli-anything-blender",
        version: "1.0.0",
        description: "3D modeling, animation, and rendering via blender --background --python",
        project_format: "blend",
        skill_path: "packages/blender/skills/SKILL.md",
        command_groups: vec![
            "scene",
            "object",
            "material",
            "modifier",
            "camera",
            "light",
            "animation",
            "render",
            "session",
        ],
        supports_json: true,
        repl_default: true,
    }
}

fn command_response(
    group: &'static str,
    command: &'static str,
    description: &'static str,
) -> CommandResponse {
    CommandResponse {
        software: "blender",
        binary: "cli-anything-blender",
        group,
        command,
        description,
    }
}

fn scene_command_name(command: &SceneCommand) -> &'static str {
    match command {
        SceneCommand::New => "new",
        SceneCommand::Info => "info",
    }
}

fn object_command_name(command: &ObjectCommand) -> &'static str {
    match command {
        ObjectCommand::Add => "add",
        ObjectCommand::List => "list",
    }
}

fn material_command_name(command: &MaterialCommand) -> &'static str {
    match command {
        MaterialCommand::Assign => "assign",
        MaterialCommand::List => "list",
    }
}

fn modifier_command_name(command: &ModifierCommand) -> &'static str {
    match command {
        ModifierCommand::Add => "add",
        ModifierCommand::Apply => "apply",
    }
}

fn camera_command_name(command: &CameraCommand) -> &'static str {
    match command {
        CameraCommand::Add => "add",
        CameraCommand::List => "list",
    }
}

fn light_command_name(command: &LightCommand) -> &'static str {
    match command {
        LightCommand::Add => "add",
        LightCommand::List => "list",
    }
}

fn animation_command_name(command: &AnimationCommand) -> &'static str {
    match command {
        AnimationCommand::Keyframe => "keyframe",
        AnimationCommand::Playblast => "playblast",
    }
}

fn render_command_name(command: &RenderCommand) -> &'static str {
    match command {
        RenderCommand::Frame => "frame",
        RenderCommand::Info => "info",
    }
}

fn session_command_name(command: &SessionCommand) -> &'static str {
    match command {
        SessionCommand::Status => "status",
        SessionCommand::History => "history",
    }
}

fn scene_command_description(command: &SceneCommand) -> &'static str {
    match command {
        SceneCommand::New => "Create a new scene",
        SceneCommand::Info => "Inspect the active scene",
    }
}

fn object_command_description(command: &ObjectCommand) -> &'static str {
    match command {
        ObjectCommand::Add => "Add a new object",
        ObjectCommand::List => "List scene objects",
    }
}

fn material_command_description(command: &MaterialCommand) -> &'static str {
    match command {
        MaterialCommand::Assign => "Assign a material",
        MaterialCommand::List => "List materials",
    }
}

fn modifier_command_description(command: &ModifierCommand) -> &'static str {
    match command {
        ModifierCommand::Add => "Add a modifier",
        ModifierCommand::Apply => "Apply a modifier",
    }
}

fn camera_command_description(command: &CameraCommand) -> &'static str {
    match command {
        CameraCommand::Add => "Add a camera",
        CameraCommand::List => "List cameras",
    }
}

fn light_command_description(command: &LightCommand) -> &'static str {
    match command {
        LightCommand::Add => "Add a light",
        LightCommand::List => "List lights",
    }
}

fn animation_command_description(command: &AnimationCommand) -> &'static str {
    match command {
        AnimationCommand::Keyframe => "Insert a keyframe",
        AnimationCommand::Playblast => "Preview the animation",
    }
}

fn render_command_description(command: &RenderCommand) -> &'static str {
    match command {
        RenderCommand::Frame => "Render a frame",
        RenderCommand::Info => "Inspect render settings",
    }
}

fn session_command_description(command: &SessionCommand) -> &'static str {
    match command {
        SessionCommand::Status => "Show session state",
        SessionCommand::History => "Inspect action history",
    }
}
