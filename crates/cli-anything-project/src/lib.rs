//! Project state, layout helpers, and artifact management.
//!
//! The manifest declares a `state_file` (for example `.gimp-cli.json`)
//! that generated packages use to persist session history, the active
//! project, and a dirty marker. This crate owns the on-disk format for
//! those files plus the minimal in-memory machinery to push, undo, and
//! redo actions.
//!
//! Backends are not yet wired through – only the scaffolding is provided
//! so that package authors and higher-level commands (refine / test /
//! validate) can start consuming a stable shape.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cli_anything_manifest::CliAnythingManifest;
use serde::{Deserialize, Serialize};

/// Current on-disk format version for the state file. Bumped whenever a
/// breaking change lands so older files can be migrated instead of
/// crashing the reader.
pub const STATE_FILE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectState {
    #[serde(default = "default_state_version")]
    pub version: u32,
    pub software: String,
    pub binary: String,
    pub project_format: String,
    #[serde(default)]
    pub active_project: Option<String>,
    #[serde(default)]
    pub dirty: bool,
    #[serde(default)]
    pub history: Vec<ActionRecord>,
    #[serde(default)]
    pub redo_stack: Vec<ActionRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionRecord {
    pub group: String,
    pub command: String,
    pub description: String,
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
}

impl ProjectState {
    /// Build a fresh state object populated from `manifest`.
    pub fn from_manifest(manifest: &CliAnythingManifest) -> Self {
        Self {
            version: STATE_FILE_VERSION,
            software: manifest.name.clone(),
            binary: manifest.binary.clone(),
            project_format: manifest.project.format.clone(),
            active_project: None,
            dirty: false,
            history: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Record a new action and mark the session as dirty. The redo stack
    /// is cleared because the timeline now diverges from what was
    /// previously undone.
    pub fn push_action(&mut self, action: ActionRecord) {
        self.history.push(action);
        self.redo_stack.clear();
        self.dirty = true;
    }

    /// Move the most recent action from history to the redo stack.
    /// Returns the undone action so callers can surface it to the user.
    pub fn undo(&mut self) -> Option<ActionRecord> {
        let action = self.history.pop()?;
        self.redo_stack.push(action.clone());
        self.dirty = true;
        Some(action)
    }

    /// Restore the most recently undone action.
    pub fn redo(&mut self) -> Option<ActionRecord> {
        let action = self.redo_stack.pop()?;
        self.history.push(action.clone());
        self.dirty = true;
        Some(action)
    }

    /// Mark the project as saved. Callers are expected to invoke this
    /// after a successful `save_state`.
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
}

/// Resolve `manifest.project.state_file` against a workspace directory.
pub fn state_path(workspace_root: &Path, manifest: &CliAnythingManifest) -> PathBuf {
    workspace_root.join(&manifest.project.state_file)
}

pub fn load_state(path: &Path) -> Result<ProjectState> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read state file at {}", path.display()))?;
    serde_json::from_str::<ProjectState>(&content)
        .with_context(|| format!("failed to parse state file at {}", path.display()))
}

pub fn load_or_init_state(path: &Path, manifest: &CliAnythingManifest) -> Result<ProjectState> {
    if path.exists() {
        load_state(path)
    } else {
        Ok(ProjectState::from_manifest(manifest))
    }
}

pub fn save_state(path: &Path, state: &ProjectState) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let payload =
        serde_json::to_string_pretty(state).context("failed to encode project state as JSON")?;
    fs::write(path, payload)
        .with_context(|| format!("failed to write state file at {}", path.display()))?;
    Ok(())
}

fn default_state_version() -> u32 {
    STATE_FILE_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;
    use cli_anything_manifest::parse_manifest;
    use std::time::{SystemTime, UNIX_EPOCH};

    const SAMPLE_MANIFEST: &str = r#"
name = "shotcut"
version = "1.0.0"
binary = "cli-anything-shotcut"
description = "Rust CLI harness for Shotcut"
repl_default = true
supports_json = true

[backend]
command = "melt"
system_package = "melt ffmpeg"
hard_dependency = true

[project]
format = "mlt"
state_file = ".shotcut-cli.json"

[skill]
output = "packages/shotcut/skills/SKILL.md"
template = "templates/skill/SKILL.md.template"
"#;

    fn unique_test_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        std::env::temp_dir().join(format!("cli-anything-rs-project-{prefix}-{nanos}"))
    }

    #[test]
    fn from_manifest_seeds_empty_history_and_clean_flag() {
        let manifest = parse_manifest(SAMPLE_MANIFEST).expect("manifest should parse");
        let state = ProjectState::from_manifest(&manifest);

        assert_eq!(state.software, "shotcut");
        assert_eq!(state.binary, "cli-anything-shotcut");
        assert_eq!(state.project_format, "mlt");
        assert!(state.history.is_empty());
        assert!(!state.dirty);
    }

    #[test]
    fn push_then_undo_then_redo_restores_action() {
        let manifest = parse_manifest(SAMPLE_MANIFEST).expect("manifest should parse");
        let mut state = ProjectState::from_manifest(&manifest);

        state.push_action(ActionRecord {
            group: "project".to_string(),
            command: "new".to_string(),
            description: "Create a new project".to_string(),
            payload: None,
        });

        let undone = state.undo().expect("undo should return the last action");
        assert_eq!(undone.command, "new");
        assert!(state.history.is_empty());

        let redone = state.redo().expect("redo should return the last action");
        assert_eq!(redone.command, "new");
        assert_eq!(state.history.len(), 1);
        assert!(state.redo_stack.is_empty());
    }

    #[test]
    fn push_action_clears_redo_stack() {
        let manifest = parse_manifest(SAMPLE_MANIFEST).expect("manifest should parse");
        let mut state = ProjectState::from_manifest(&manifest);

        state.push_action(ActionRecord {
            group: "project".to_string(),
            command: "new".to_string(),
            description: "Create a new project".to_string(),
            payload: None,
        });
        state.undo();
        state.push_action(ActionRecord {
            group: "project".to_string(),
            command: "info".to_string(),
            description: "Show info".to_string(),
            payload: None,
        });

        assert!(state.redo_stack.is_empty());
        assert_eq!(state.history.len(), 1);
        assert_eq!(state.history[0].command, "info");
    }

    #[test]
    fn save_and_load_round_trip_preserves_state() {
        let manifest = parse_manifest(SAMPLE_MANIFEST).expect("manifest should parse");
        let workspace = unique_test_dir("round-trip");
        fs::create_dir_all(&workspace).expect("workspace should exist");
        let path = state_path(&workspace, &manifest);

        let mut state = ProjectState::from_manifest(&manifest);
        state.push_action(ActionRecord {
            group: "project".to_string(),
            command: "new".to_string(),
            description: "Create a new project".to_string(),
            payload: Some(serde_json::json!({ "name": "demo" })),
        });
        state.mark_clean();

        save_state(&path, &state).expect("state should be written");
        let loaded = load_state(&path).expect("state should load");

        assert_eq!(loaded, state);

        fs::remove_dir_all(&workspace).expect("workspace should be removed");
    }

    #[test]
    fn load_or_init_state_returns_fresh_state_when_file_missing() {
        let manifest = parse_manifest(SAMPLE_MANIFEST).expect("manifest should parse");
        let workspace = unique_test_dir("missing");
        fs::create_dir_all(&workspace).expect("workspace should exist");
        let path = state_path(&workspace, &manifest);

        let state =
            load_or_init_state(&path, &manifest).expect("init should not fail for missing file");

        assert!(state.history.is_empty());
        assert!(!state.dirty);

        fs::remove_dir_all(&workspace).expect("workspace should be removed");
    }
}
