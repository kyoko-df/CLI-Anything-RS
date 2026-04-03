use clap::{Parser, Subcommand};
use cli_anything_repl::Skin;

#[derive(Debug, Parser)]
#[command(name = "cli-anything")]
#[command(about = "Rust-first harness generator bootstrap")]
struct App {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Status,
}

fn main() {
    let app = App::parse();

    match app.command.unwrap_or(Command::Status) {
        Command::Status => {
            let skin = Skin::new("cli-anything", env!("CARGO_PKG_VERSION"));
            println!("{}", skin.banner_title());
            println!("workspace bootstrap ready");
        }
    }
}
