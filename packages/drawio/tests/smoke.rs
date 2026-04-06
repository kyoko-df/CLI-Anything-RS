use std::process::Command;

use serde_json::Value;

fn run_binary(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cli-anything-drawio"))
        .args(args)
        .output()
        .expect("generated binary should run")
}

#[test]
fn binary_name_is_stable() {
    assert_eq!("cli-anything-drawio", "cli-anything-drawio");
}

#[test]
fn json_summary_reports_package_metadata() {
    let output = run_binary(&["--json"]);

    assert!(output.status.success());

    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("summary output should be valid json");

    assert_eq!(payload["name"], "drawio");
    assert_eq!(payload["binary"], "cli-anything-drawio");
    assert_eq!(payload["version"], "1.0.0");
    assert_eq!(
        payload["description"],
        "Diagram creation and export via draw.io CLI"
    );
    assert_eq!(payload["project_format"], "drawio");
    assert_eq!(payload["command_groups"].as_array().map(Vec::len), Some(6));
}

#[test]
fn json_subcommand_response_includes_description() {
    let output = run_binary(&["--json", "project", "new"]);

    assert!(output.status.success());

    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("command output should be valid json");

    assert_eq!(payload["software"], "drawio");
    assert_eq!(payload["group"], "project");
    assert_eq!(payload["command"], "new");
    assert_eq!(payload["description"], "Create a new diagram");
}
