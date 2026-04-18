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
        "cli-anything-rs-drawio-{label}-{nanos}-{bump}.json"
    ))
}

fn run_json(state_file: &PathBuf, args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_cli-anything-drawio"))
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

    assert_eq!(payload["name"], "drawio");
    assert_eq!(payload["binary"], "cli-anything-drawio");
    assert_eq!(payload["version"], "1.0.0");
    assert_eq!(payload["project_format"], "drawio");
    assert_eq!(payload["command_groups"].as_array().map(Vec::len), Some(7));
}

#[test]
fn diagram_new_reports_template() {
    let state_file = unique_state_file("diagram-new");
    let payload = run_json(
        &state_file,
        &[
            "--json",
            "diagram",
            "new",
            "--name",
            "system",
            "--template",
            "architecture",
        ],
    );

    assert_eq!(payload["group"], "diagram");
    assert_eq!(payload["command"], "new");
    assert_eq!(payload["diagram"]["name"], "system");
    assert_eq!(payload["diagram"]["template"], "architecture");
}

#[test]
fn shape_add_reports_coordinates() {
    let state_file = unique_state_file("shape-add");
    let payload = run_json(
        &state_file,
        &[
            "--json",
            "shape",
            "add",
            "--kind",
            "rectangle",
            "--x",
            "100",
            "--y",
            "200",
        ],
    );

    assert_eq!(payload["shape"]["kind"], "rectangle");
    assert_eq!(payload["shape"]["x"], 100);
    assert_eq!(payload["shape"]["y"], 200);
}

#[test]
fn connection_add_reports_endpoints() {
    let state_file = unique_state_file("connection-add");
    let payload = run_json(
        &state_file,
        &["--json", "connection", "add", "--from", "A", "--to", "B"],
    );

    assert_eq!(payload["connection"]["from"], "A");
    assert_eq!(payload["connection"]["to"], "B");
    assert_eq!(payload["connection"]["style"], "orthogonal");
}

#[test]
fn export_png_reports_dpi_and_output() {
    let state_file = unique_state_file("export-png");
    let payload = run_json(
        &state_file,
        &[
            "--json",
            "export",
            "png",
            "--output",
            "diagram.png",
            "--dpi",
            "600",
        ],
    );

    assert_eq!(payload["export"]["format"], "png");
    assert_eq!(payload["export"]["output"], "diagram.png");
    assert_eq!(payload["export"]["dpi"], 600);
}

#[test]
fn session_status_reports_state_after_mutation() {
    let state_file = unique_state_file("session-status");

    let fresh = run_json(&state_file, &["--json", "session", "status"]);
    assert_eq!(fresh["session"]["dirty"], false);
    assert_eq!(fresh["session"]["history_depth"], 0);

    run_json(
        &state_file,
        &["--json", "diagram", "new", "--name", "system"],
    );

    let after = run_json(&state_file, &["--json", "session", "status"]);
    assert_eq!(after["session"]["dirty"], true);
    assert_eq!(after["session"]["history_depth"], 1);
    assert_eq!(after["session"]["active_project"], "system");
}

#[test]
fn session_undo_redo_roundtrip() {
    let state_file = unique_state_file("session-undo-redo");

    run_json(
        &state_file,
        &["--json", "diagram", "new", "--name", "system"],
    );
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

    run_json(
        &state_file,
        &["--json", "diagram", "new", "--name", "system"],
    );
    run_json(
        &state_file,
        &[
            "--json",
            "shape",
            "add",
            "--kind",
            "rectangle",
            "--x",
            "0",
            "--y",
            "0",
        ],
    );

    let history = run_json(&state_file, &["--json", "session", "history"]);
    assert_eq!(history["history_depth"], 2);
    assert_eq!(history["history"][0]["group"], "diagram");
    assert_eq!(history["history"][1]["group"], "shape");
}

#[test]
fn session_save_marks_clean() {
    let state_file = unique_state_file("session-save");

    run_json(
        &state_file,
        &["--json", "diagram", "new", "--name", "system"],
    );

    let saved = run_json(&state_file, &["--json", "session", "save"]);
    assert_eq!(saved["status"], "saved");

    let status = run_json(&state_file, &["--json", "session", "status"]);
    assert_eq!(status["session"]["dirty"], false);
}
