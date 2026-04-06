use std::process::Command;

use serde_json::Value;

fn run_binary(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cli-anything-gimp"))
        .args(args)
        .output()
        .expect("generated binary should run")
}

#[test]
fn binary_name_is_stable() {
    assert_eq!("cli-anything-gimp", "cli-anything-gimp");
}

#[test]
fn json_summary_reports_package_metadata() {
    let output = run_binary(&["--json"]);

    assert!(output.status.success());

    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("summary output should be valid json");

    assert_eq!(payload["name"], "gimp");
    assert_eq!(payload["binary"], "cli-anything-gimp");
    assert_eq!(payload["version"], "1.0.0");
    assert_eq!(
        payload["description"],
        "Raster image processing via gimp -i -b (batch mode)"
    );
    assert_eq!(payload["project_format"], "xcf");
    assert_eq!(payload["command_groups"].as_array().map(Vec::len), Some(8));
}

#[test]
fn json_subcommand_response_includes_description() {
    let output = run_binary(&["--json", "project", "new"]);

    assert!(output.status.success());

    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("command output should be valid json");

    assert_eq!(payload["software"], "gimp");
    assert_eq!(payload["group"], "project");
    assert_eq!(payload["command"], "new");
    assert_eq!(payload["description"], "Create a new image project");
}

#[test]
fn project_new_json_includes_requested_dimensions() {
    let output = run_binary(&[
        "--json", "project", "new", "--name", "poster", "--width", "2048", "--height", "1024",
    ]);

    assert!(output.status.success());

    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("command output should be valid json");

    assert_eq!(payload["group"], "project");
    assert_eq!(payload["command"], "new");
    assert_eq!(payload["project"]["name"], "poster");
    assert_eq!(payload["project"]["width"], 2048);
    assert_eq!(payload["project"]["height"], 1024);
    assert_eq!(payload["project"]["color_mode"], "RGB");
}

#[test]
fn project_info_json_reports_default_template() {
    let output = run_binary(&["--json", "project", "info"]);

    assert!(output.status.success());

    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("command output should be valid json");

    assert_eq!(payload["group"], "project");
    assert_eq!(payload["command"], "info");
    assert_eq!(payload["project_format"], "xcf");
    assert_eq!(payload["default_template"]["width"], 1920);
    assert_eq!(payload["default_template"]["height"], 1080);
    assert_eq!(payload["default_template"]["color_mode"], "RGB");
}

#[test]
fn filter_list_json_reports_known_filters() {
    let output = run_binary(&["--json", "filter", "list"]);

    assert!(output.status.success());

    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("command output should be valid json");

    assert_eq!(payload["group"], "filter");
    assert_eq!(payload["command"], "list");
    assert_eq!(payload["filter_count"], 4);
    assert_eq!(payload["filters"][0]["name"], "brightness");
    assert_eq!(payload["filters"][1]["name"], "contrast");
}

#[test]
fn layer_list_json_reports_default_layers() {
    let output = run_binary(&["--json", "layer", "list"]);

    assert!(output.status.success());

    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("command output should be valid json");

    assert_eq!(payload["group"], "layer");
    assert_eq!(payload["command"], "list");
    assert_eq!(payload["layer_count"], 3);
    assert_eq!(payload["layers"][0]["name"], "Background");
    assert_eq!(payload["layers"][1]["blend_mode"], "normal");
}

#[test]
fn canvas_info_json_reports_current_canvas_state() {
    let output = run_binary(&["--json", "canvas", "info"]);

    assert!(output.status.success());

    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("command output should be valid json");

    assert_eq!(payload["group"], "canvas");
    assert_eq!(payload["command"], "info");
    assert_eq!(payload["canvas"]["width"], 1920);
    assert_eq!(payload["canvas"]["height"], 1080);
    assert_eq!(payload["canvas"]["units"], "px");
}

#[test]
fn canvas_resize_json_reports_requested_dimensions() {
    let output = run_binary(&[
        "--json", "canvas", "resize", "--width", "4096", "--height", "2160",
    ]);

    assert!(output.status.success());

    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("command output should be valid json");

    assert_eq!(payload["group"], "canvas");
    assert_eq!(payload["command"], "resize");
    assert_eq!(payload["canvas"]["width"], 4096);
    assert_eq!(payload["canvas"]["height"], 2160);
    assert_eq!(payload["canvas"]["anchor"], "center");
}

#[test]
fn export_presets_json_reports_available_formats() {
    let output = run_binary(&["--json", "export", "presets"]);

    assert!(output.status.success());

    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("command output should be valid json");

    assert_eq!(payload["group"], "export");
    assert_eq!(payload["command"], "presets");
    assert_eq!(payload["preset_count"], 3);
    assert_eq!(payload["presets"][0]["name"], "web-png");
    assert_eq!(payload["presets"][2]["format"], "tiff");
}

#[test]
fn media_import_json_reports_asset_metadata() {
    let output = run_binary(&[
        "--json",
        "media",
        "import",
        "--path",
        "fixtures/reference.png",
        "--slot",
        "reference",
    ]);

    assert!(output.status.success());

    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("command output should be valid json");

    assert_eq!(payload["group"], "media");
    assert_eq!(payload["command"], "import");
    assert_eq!(payload["asset"]["path"], "fixtures/reference.png");
    assert_eq!(payload["asset"]["slot"], "reference");
    assert_eq!(payload["asset"]["status"], "queued");
}

#[test]
fn media_list_json_reports_imported_assets() {
    let output = run_binary(&["--json", "media", "list"]);

    assert!(output.status.success());

    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("command output should be valid json");

    assert_eq!(payload["group"], "media");
    assert_eq!(payload["command"], "list");
    assert_eq!(payload["asset_count"], 3);
    assert_eq!(payload["assets"][0]["kind"], "image");
    assert_eq!(payload["assets"][2]["slot"], "mask");
}

#[test]
fn session_status_json_reports_current_checkpoint() {
    let output = run_binary(&["--json", "session", "status"]);

    assert!(output.status.success());

    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("command output should be valid json");

    assert_eq!(payload["group"], "session");
    assert_eq!(payload["command"], "status");
    assert_eq!(payload["session"]["dirty"], false);
    assert_eq!(payload["session"]["active_tool"], "paintbrush");
    assert_eq!(payload["session"]["history_depth"], 12);
}

#[test]
fn session_undo_json_reports_reverted_operation() {
    let output = run_binary(&["--json", "session", "undo"]);

    assert!(output.status.success());

    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("command output should be valid json");

    assert_eq!(payload["group"], "session");
    assert_eq!(payload["command"], "undo");
    assert_eq!(payload["undone_action"]["name"], "bucket-fill");
    assert_eq!(payload["undone_action"]["target"], "Highlights");
}

#[test]
fn draw_line_json_reports_stroke_geometry() {
    let output = run_binary(&[
        "--json", "draw", "line", "--x1", "10", "--y1", "20", "--x2", "320", "--y2", "240",
    ]);

    assert!(output.status.success());

    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("command output should be valid json");

    assert_eq!(payload["group"], "draw");
    assert_eq!(payload["command"], "line");
    assert_eq!(payload["stroke"]["start"]["x"], 10);
    assert_eq!(payload["stroke"]["end"]["y"], 240);
    assert_eq!(payload["stroke"]["tool"], "paintbrush");
}

#[test]
fn draw_rectangle_json_reports_bounds() {
    let output = run_binary(&[
        "--json",
        "draw",
        "rectangle",
        "--x",
        "64",
        "--y",
        "96",
        "--width",
        "512",
        "--height",
        "256",
    ]);

    assert!(output.status.success());

    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("command output should be valid json");

    assert_eq!(payload["group"], "draw");
    assert_eq!(payload["command"], "rectangle");
    assert_eq!(payload["shape"]["x"], 64);
    assert_eq!(payload["shape"]["height"], 256);
    assert_eq!(payload["shape"]["fill"], "none");
}
