use clap::{Parser, Subcommand};
use cli_anything_repl::Skin;
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(name = "cli-anything-drawio")]
#[command(about = "Diagram creation and export via draw.io CLI")]
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
    Shape {
        #[command(subcommand)]
        command: ShapeCommand,
    },
    Connect {
        #[command(subcommand)]
        command: ConnectCommand,
    },
    Page {
        #[command(subcommand)]
        command: PageCommand,
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
enum ProjectCommand {
    New,
    Info,
}

#[derive(Debug, Subcommand)]
enum ShapeCommand {
    Add,
    Types,
}

#[derive(Debug, Subcommand)]
enum ConnectCommand {
    Add,
    Styles,
}

#[derive(Debug, Subcommand)]
enum PageCommand {
    Add,
    List,
}

#[derive(Debug, Subcommand)]
enum ExportCommand {
    Diagram,
    Formats,
}

#[derive(Debug, Subcommand)]
enum SessionCommand {
    Status,
    Save,
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
    let skin = Skin::new("drawio", "1.0.0").with_skill_path("skills/SKILL.md");

    match app.action {
        Some(action) => {
            let response = match action {
                Action::Project { command } => command_response(
                    "project",
                    project_command_name(&command),
                    project_command_description(&command),
                ),
                Action::Shape { command } => command_response(
                    "shape",
                    shape_command_name(&command),
                    shape_command_description(&command),
                ),
                Action::Connect { command } => command_response(
                    "connect",
                    connect_command_name(&command),
                    connect_command_description(&command),
                ),
                Action::Page { command } => command_response(
                    "page",
                    page_command_name(&command),
                    page_command_description(&command),
                ),
                Action::Export { command } => command_response(
                    "export",
                    export_command_name(&command),
                    export_command_description(&command),
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
                println!("{}", skin.status("binary", "cli-anything-drawio"));
                println!("{}", skin.status("format", "drawio"));
                println!(
                    "{}",
                    skin.status("groups", "project, shape, connect, page, export, session")
                );
            }
        }
    }
}

fn package_summary() -> PackageSummary {
    PackageSummary {
        name: "drawio",
        binary: "cli-anything-drawio",
        version: "1.0.0",
        description: "Diagram creation and export via draw.io CLI",
        project_format: "drawio",
        skill_path: "packages/drawio/skills/SKILL.md",
        command_groups: vec!["project", "shape", "connect", "page", "export", "session"],
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
        software: "drawio",
        binary: "cli-anything-drawio",
        group,
        command,
        description,
    }
}

fn project_command_name(command: &ProjectCommand) -> &'static str {
    match command {
        ProjectCommand::New => "new",
        ProjectCommand::Info => "info",
    }
}

fn shape_command_name(command: &ShapeCommand) -> &'static str {
    match command {
        ShapeCommand::Add => "add",
        ShapeCommand::Types => "types",
    }
}

fn connect_command_name(command: &ConnectCommand) -> &'static str {
    match command {
        ConnectCommand::Add => "add",
        ConnectCommand::Styles => "styles",
    }
}

fn page_command_name(command: &PageCommand) -> &'static str {
    match command {
        PageCommand::Add => "add",
        PageCommand::List => "list",
    }
}

fn export_command_name(command: &ExportCommand) -> &'static str {
    match command {
        ExportCommand::Diagram => "diagram",
        ExportCommand::Formats => "formats",
    }
}

fn session_command_name(command: &SessionCommand) -> &'static str {
    match command {
        SessionCommand::Status => "status",
        SessionCommand::Save => "save",
    }
}

fn project_command_description(command: &ProjectCommand) -> &'static str {
    match command {
        ProjectCommand::New => "Create a new diagram",
        ProjectCommand::Info => "Show project metadata",
    }
}

fn shape_command_description(command: &ShapeCommand) -> &'static str {
    match command {
        ShapeCommand::Add => "Add a shape",
        ShapeCommand::Types => "List shape types",
    }
}

fn connect_command_description(command: &ConnectCommand) -> &'static str {
    match command {
        ConnectCommand::Add => "Create a connector",
        ConnectCommand::Styles => "List connector styles",
    }
}

fn page_command_description(command: &PageCommand) -> &'static str {
    match command {
        PageCommand::Add => "Add a page",
        PageCommand::List => "List pages",
    }
}

fn export_command_description(command: &ExportCommand) -> &'static str {
    match command {
        ExportCommand::Diagram => "Export a diagram",
        ExportCommand::Formats => "List formats",
    }
}

fn session_command_description(command: &SessionCommand) -> &'static str {
    match command {
        SessionCommand::Status => "Show current session",
        SessionCommand::Save => "Persist session state",
    }
}
