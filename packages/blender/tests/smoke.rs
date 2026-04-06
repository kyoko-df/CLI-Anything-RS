use std::process::Command;

use serde_json::Value;

fn run_binary(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cli-anything-blender"))
        .args(args)
        .output()
        .expect("generated binary should run")
}

#[test]
fn binary_name_is_stable() {
    assert_eq!("cli-anything-blender", "cli-anything-blender");
}

#[test]
fn json_summary_reports_package_metadata() {
    let output = run_binary(&["--json"]);

    assert!(output.status.success());

    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("summary output should be valid json");

    assert_eq!(payload["name"], "blender");
    assert_eq!(payload["binary"], "cli-anything-blender");
    assert_eq!(payload["version"], "1.0.0");
    assert_eq!(
        payload["description"],
        "3D modeling, animation, and rendering via blender --background --python"
    );
    assert_eq!(payload["project_format"], "blend");
    assert_eq!(payload["command_groups"].as_array().map(Vec::len), Some(9));
}

#[test]
fn json_subcommand_response_includes_description() {
    let output = run_binary(&["--json", "scene", "new"]);

    assert!(output.status.success());

    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("command output should be valid json");

    assert_eq!(payload["software"], "blender");
    assert_eq!(payload["group"], "scene");
    assert_eq!(payload["command"], "new");
    assert_eq!(payload["description"], "Create a new scene");
}
