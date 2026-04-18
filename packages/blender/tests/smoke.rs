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
    std::env::temp_dir().join(format!(
        "cli-anything-rs-blender-{label}-{nanos}-{bump}.json"
    ))
}

fn run_json(state_file: &PathBuf, args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_cli-anything-blender"))
        .env("CLI_ANYTHING_STATE_FILE", state_file)
        .args(args)
        .output()
        .expect("generated binary should run");
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
fn json_summary_reports_package_metadata() {
    let state_file = unique_state_file("summary");
    let payload = run_json(&state_file, &["--json"]);

    assert_eq!(payload["name"], "blender");
    assert_eq!(payload["binary"], "cli-anything-blender");
    assert_eq!(payload["version"], "1.0.0");
    assert_eq!(payload["project_format"], "blend");
    assert_eq!(payload["command_groups"].as_array().map(Vec::len), Some(9));
}

#[test]
fn scene_new_reports_requested_frame_range() {
    let state_file = unique_state_file("scene-new");
    let payload = run_json(
        &state_file,
        &[
            "--json",
            "scene",
            "new",
            "--name",
            "intro",
            "--start-frame",
            "10",
            "--end-frame",
            "150",
        ],
    );

    assert_eq!(payload["group"], "scene");
    assert_eq!(payload["command"], "new");
    assert_eq!(payload["scene"]["name"], "intro");
    assert_eq!(payload["scene"]["start_frame"], 10);
    assert_eq!(payload["scene"]["end_frame"], 150);
}

#[test]
fn object_add_reports_location() {
    let state_file = unique_state_file("object-add");
    let payload = run_json(
        &state_file,
        &[
            "--json", "object", "add", "--kind", "mesh", "--name", "Hero",
        ],
    );

    assert_eq!(payload["group"], "object");
    assert_eq!(payload["command"], "add");
    assert_eq!(payload["object"]["kind"], "mesh");
    assert_eq!(payload["object"]["name"], "Hero");
    assert_eq!(payload["object"]["location"]["x"], 0);
}

#[test]
fn object_list_reports_default_objects() {
    let state_file = unique_state_file("object-list");
    let payload = run_json(&state_file, &["--json", "object", "list"]);

    assert_eq!(payload["object_count"], 3);
    assert_eq!(payload["objects"][0]["name"], "Cube");
}

#[test]
fn material_assign_reports_slot() {
    let state_file = unique_state_file("material-assign");
    let payload = run_json(
        &state_file,
        &[
            "--json",
            "material",
            "assign",
            "--object",
            "Cube",
            "--material",
            "Metal",
        ],
    );

    assert_eq!(payload["assignment"]["object"], "Cube");
    assert_eq!(payload["assignment"]["material"], "Metal");
    assert_eq!(payload["assignment"]["slot"], 0);
}

#[test]
fn render_frame_reports_format_and_engine() {
    let state_file = unique_state_file("render-frame");
    let payload = run_json(
        &state_file,
        &[
            "--json", "render", "frame", "--frame", "42", "--format", "exr",
        ],
    );

    assert_eq!(payload["render"]["frame"], 42);
    assert_eq!(payload["render"]["format"], "exr");
    assert_eq!(payload["render"]["engine"], "CYCLES");
}

#[test]
fn session_status_reports_state_after_mutation() {
    let state_file = unique_state_file("session-status");

    let fresh = run_json(&state_file, &["--json", "session", "status"]);
    assert_eq!(fresh["session"]["dirty"], false);
    assert_eq!(fresh["session"]["history_depth"], 0);
    assert_eq!(fresh["session"]["active_project"], Value::Null);

    run_json(&state_file, &["--json", "scene", "new", "--name", "intro"]);

    let after = run_json(&state_file, &["--json", "session", "status"]);
    assert_eq!(after["session"]["dirty"], true);
    assert_eq!(after["session"]["history_depth"], 1);
    assert_eq!(after["session"]["active_project"], "intro");
}

#[test]
fn session_undo_redo_roundtrip() {
    let state_file = unique_state_file("session-undo-redo");

    run_json(&state_file, &["--json", "scene", "new", "--name", "intro"]);
    let undone = run_json(&state_file, &["--json", "session", "undo"]);
    assert_eq!(undone["status"], "undone");
    assert_eq!(undone["history_depth"], 0);

    let redone = run_json(&state_file, &["--json", "session", "redo"]);
    assert_eq!(redone["status"], "redone");
    assert_eq!(redone["history_depth"], 1);
}

#[test]
fn session_history_lists_recorded_actions() {
    let state_file = unique_state_file("session-history");

    run_json(&state_file, &["--json", "scene", "new", "--name", "intro"]);
    run_json(
        &state_file,
        &[
            "--json", "object", "add", "--kind", "mesh", "--name", "Hero",
        ],
    );

    let history = run_json(&state_file, &["--json", "session", "history"]);
    assert_eq!(history["history_depth"], 2);
    assert_eq!(history["history"][0]["group"], "scene");
    assert_eq!(history["history"][1]["group"], "object");
}

#[test]
fn session_save_marks_clean() {
    let state_file = unique_state_file("session-save");

    run_json(&state_file, &["--json", "scene", "new", "--name", "intro"]);

    let saved = run_json(&state_file, &["--json", "session", "save"]);
    assert_eq!(saved["status"], "saved");

    let status = run_json(&state_file, &["--json", "session", "status"]);
    assert_eq!(status["session"]["dirty"], false);
}
