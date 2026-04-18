use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_state_file(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    let bump = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("cli-anything-rs-gimp-{label}-{nanos}-{bump}.json"))
}

fn run_binary(state_file: &PathBuf, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cli-anything-gimp"))
        .env("CLI_ANYTHING_STATE_FILE", state_file)
        .args(args)
        .output()
        .expect("generated binary should run")
}

fn run_json(state_file: &PathBuf, args: &[&str]) -> Value {
    let output = run_binary(state_file, args);
    assert!(
        output.status.success(),
        "binary exited with {:?} for args {:?}\nstderr: {}",
        output.status,
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("command output should be valid json")
}

#[test]
fn binary_name_is_stable() {
    assert_eq!("cli-anything-gimp", "cli-anything-gimp");
}

#[test]
fn json_summary_reports_package_metadata() {
    let state_file = unique_state_file("summary");
    let payload = run_json(&state_file, &["--json"]);

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
    let state_file = unique_state_file("subcommand");
    let payload = run_json(&state_file, &["--json", "project", "new"]);

    assert_eq!(payload["software"], "gimp");
    assert_eq!(payload["group"], "project");
    assert_eq!(payload["command"], "new");
    assert_eq!(payload["description"], "Create a new image project");
}

#[test]
fn project_new_json_includes_requested_dimensions() {
    let state_file = unique_state_file("project-new");
    let payload = run_json(
        &state_file,
        &[
            "--json", "project", "new", "--name", "poster", "--width", "2048", "--height", "1024",
        ],
    );

    assert_eq!(payload["group"], "project");
    assert_eq!(payload["command"], "new");
    assert_eq!(payload["project"]["name"], "poster");
    assert_eq!(payload["project"]["width"], 2048);
    assert_eq!(payload["project"]["height"], 1024);
    assert_eq!(payload["project"]["color_mode"], "RGB");
}

#[test]
fn project_info_json_reports_default_template() {
    let state_file = unique_state_file("project-info");
    let payload = run_json(&state_file, &["--json", "project", "info"]);

    assert_eq!(payload["group"], "project");
    assert_eq!(payload["command"], "info");
    assert_eq!(payload["project_format"], "xcf");
    assert_eq!(payload["default_template"]["width"], 1920);
    assert_eq!(payload["default_template"]["height"], 1080);
    assert_eq!(payload["default_template"]["color_mode"], "RGB");
}

#[test]
fn filter_list_json_reports_known_filters() {
    let state_file = unique_state_file("filter-list");
    let payload = run_json(&state_file, &["--json", "filter", "list"]);

    assert_eq!(payload["group"], "filter");
    assert_eq!(payload["command"], "list");
    assert_eq!(payload["filter_count"], 4);
    assert_eq!(payload["filters"][0]["name"], "brightness");
    assert_eq!(payload["filters"][1]["name"], "contrast");
}

#[test]
fn layer_list_json_reports_default_layers() {
    let state_file = unique_state_file("layer-list");
    let payload = run_json(&state_file, &["--json", "layer", "list"]);

    assert_eq!(payload["group"], "layer");
    assert_eq!(payload["command"], "list");
    assert_eq!(payload["layer_count"], 3);
    assert_eq!(payload["layers"][0]["name"], "Background");
    assert_eq!(payload["layers"][1]["blend_mode"], "normal");
}

#[test]
fn canvas_info_json_reports_current_canvas_state() {
    let state_file = unique_state_file("canvas-info");
    let payload = run_json(&state_file, &["--json", "canvas", "info"]);

    assert_eq!(payload["group"], "canvas");
    assert_eq!(payload["command"], "info");
    assert_eq!(payload["canvas"]["width"], 1920);
    assert_eq!(payload["canvas"]["height"], 1080);
    assert_eq!(payload["canvas"]["units"], "px");
}

#[test]
fn canvas_resize_json_reports_requested_dimensions() {
    let state_file = unique_state_file("canvas-resize");
    let payload = run_json(
        &state_file,
        &[
            "--json", "canvas", "resize", "--width", "4096", "--height", "2160",
        ],
    );

    assert_eq!(payload["group"], "canvas");
    assert_eq!(payload["command"], "resize");
    assert_eq!(payload["canvas"]["width"], 4096);
    assert_eq!(payload["canvas"]["height"], 2160);
    assert_eq!(payload["canvas"]["anchor"], "center");
}

#[test]
fn export_presets_json_reports_available_formats() {
    let state_file = unique_state_file("export-presets");
    let payload = run_json(&state_file, &["--json", "export", "presets"]);

    assert_eq!(payload["group"], "export");
    assert_eq!(payload["command"], "presets");
    assert_eq!(payload["preset_count"], 3);
    assert_eq!(payload["presets"][0]["name"], "web-png");
    assert_eq!(payload["presets"][2]["format"], "tiff");
}

#[test]
fn media_import_json_reports_asset_metadata() {
    let state_file = unique_state_file("media-import");
    let payload = run_json(
        &state_file,
        &[
            "--json",
            "media",
            "import",
            "--path",
            "fixtures/reference.png",
            "--slot",
            "reference",
        ],
    );

    assert_eq!(payload["group"], "media");
    assert_eq!(payload["command"], "import");
    assert_eq!(payload["asset"]["path"], "fixtures/reference.png");
    assert_eq!(payload["asset"]["slot"], "reference");
    assert_eq!(payload["asset"]["status"], "queued");
}

#[test]
fn media_list_json_reports_imported_assets() {
    let state_file = unique_state_file("media-list");
    let payload = run_json(&state_file, &["--json", "media", "list"]);

    assert_eq!(payload["group"], "media");
    assert_eq!(payload["command"], "list");
    assert_eq!(payload["asset_count"], 3);
    assert_eq!(payload["assets"][0]["kind"], "image");
    assert_eq!(payload["assets"][2]["slot"], "mask");
}

#[test]
fn session_status_reports_state_after_mutation() {
    let state_file = unique_state_file("session-status");

    let fresh = run_json(&state_file, &["--json", "session", "status"]);
    assert_eq!(fresh["group"], "session");
    assert_eq!(fresh["command"], "status");
    assert_eq!(fresh["session"]["dirty"], false);
    assert_eq!(fresh["session"]["history_depth"], 0);
    assert_eq!(fresh["session"]["active_project"], Value::Null);

    run_json(&state_file, &["--json", "project", "new", "--name", "demo"]);

    let after = run_json(&state_file, &["--json", "session", "status"]);
    assert_eq!(after["session"]["dirty"], true);
    assert_eq!(after["session"]["history_depth"], 1);
    assert_eq!(after["session"]["active_project"], "demo");
}

#[test]
fn session_undo_returns_last_action_then_empties_history() {
    let state_file = unique_state_file("session-undo");

    run_json(
        &state_file,
        &["--json", "project", "new", "--name", "poster"],
    );

    let undone = run_json(&state_file, &["--json", "session", "undo"]);
    assert_eq!(undone["command"], "undo");
    assert_eq!(undone["status"], "undone");
    assert_eq!(undone["undone_action"]["command"], "new");
    assert_eq!(undone["history_depth"], 0);

    let nothing = run_json(&state_file, &["--json", "session", "undo"]);
    assert_eq!(nothing["status"], "nothing-to-undo");
    assert_eq!(nothing["history_depth"], 0);
}

#[test]
fn session_redo_restores_undone_action() {
    let state_file = unique_state_file("session-redo");

    run_json(
        &state_file,
        &["--json", "project", "new", "--name", "poster"],
    );
    run_json(&state_file, &["--json", "session", "undo"]);

    let redone = run_json(&state_file, &["--json", "session", "redo"]);
    assert_eq!(redone["status"], "redone");
    assert_eq!(redone["redone_action"]["command"], "new");
    assert_eq!(redone["history_depth"], 1);
}

#[test]
fn session_history_lists_recorded_actions() {
    let state_file = unique_state_file("session-history");

    run_json(
        &state_file,
        &["--json", "project", "new", "--name", "poster"],
    );
    run_json(
        &state_file,
        &[
            "--json", "canvas", "resize", "--width", "2048", "--height", "1024",
        ],
    );

    let history = run_json(&state_file, &["--json", "session", "history"]);
    assert_eq!(history["history_depth"], 2);
    assert_eq!(history["history"][0]["group"], "project");
    assert_eq!(history["history"][0]["command"], "new");
    assert_eq!(history["history"][1]["group"], "canvas");
    assert_eq!(history["history"][1]["command"], "resize");
}

#[test]
fn session_save_marks_clean() {
    let state_file = unique_state_file("session-save");

    run_json(&state_file, &["--json", "project", "new", "--name", "demo"]);

    let saved = run_json(&state_file, &["--json", "session", "save"]);
    assert_eq!(saved["status"], "saved");
    assert_eq!(saved["history_depth"], 1);

    let status = run_json(&state_file, &["--json", "session", "status"]);
    // Session commands do not mutate history; save only flips the dirty flag.
    assert_eq!(status["session"]["history_depth"], 1);
    assert_eq!(status["session"]["dirty"], false);
}

#[test]
fn draw_line_json_reports_stroke_geometry() {
    let state_file = unique_state_file("draw-line");
    let payload = run_json(
        &state_file,
        &[
            "--json", "draw", "line", "--x1", "10", "--y1", "20", "--x2", "320", "--y2", "240",
        ],
    );

    assert_eq!(payload["group"], "draw");
    assert_eq!(payload["command"], "line");
    assert_eq!(payload["stroke"]["start"]["x"], 10);
    assert_eq!(payload["stroke"]["end"]["y"], 240);
    assert_eq!(payload["stroke"]["tool"], "paintbrush");
}

#[test]
fn draw_rectangle_json_reports_bounds() {
    let state_file = unique_state_file("draw-rect");
    let payload = run_json(
        &state_file,
        &[
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
        ],
    );

    assert_eq!(payload["group"], "draw");
    assert_eq!(payload["command"], "rectangle");
    assert_eq!(payload["shape"]["x"], 64);
    assert_eq!(payload["shape"]["height"], 256);
    assert_eq!(payload["shape"]["fill"], "none");
}
