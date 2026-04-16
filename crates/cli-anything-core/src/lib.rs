//! Shared domain model for the CLI-Anything-RS workspace.
//!
//! The manifest schema itself lives in `cli-anything-manifest`; this
//! crate re-exports those types so existing consumers keep compiling,
//! and owns the rest of the cross-cutting surface: built-in command
//! documents, source-target parsing, package layout helpers, the
//! validation report shape, and the shared `PackageSummary` /
//! `CommandResponse` payloads used by every generated binary.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use cli_anything_manifest::{
    BackendConfig, CliAnythingManifest, CommandGroup, CommandSpec, ExampleSpec, KnownPackageSpec,
    ProjectConfig, SkillConfig, builtin_package_spec, builtin_package_specs,
    load_manifest_from_path, parse_manifest,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub description: String,
    pub author: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuiltinCommandId {
    Build,
    Refine,
    Test,
    Validate,
    List,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandDocument {
    pub id: BuiltinCommandId,
    pub title: String,
    pub usage: String,
    pub summary: String,
    pub phases: Vec<WorkflowPhase>,
    pub success_criteria: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowPhase {
    pub title: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceTargetKind {
    LocalPath(PathBuf),
    GitHubRepo(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceTarget {
    pub raw: String,
    pub software_name: String,
    pub kind: SourceTargetKind,
}

impl SourceTarget {
    pub fn is_remote(&self) -> bool {
        matches!(self.kind, SourceTargetKind::GitHubRepo(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageLayout {
    pub software_name: String,
    pub root: PathBuf,
    pub cargo_toml: PathBuf,
    pub manifest: PathBuf,
    pub src_dir: PathBuf,
    pub src_main: PathBuf,
    pub skills_dir: PathBuf,
    pub skill_file: PathBuf,
    pub tests_dir: PathBuf,
    pub fixtures_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub software_name: String,
    pub package_root: PathBuf,
    pub categories: Vec<ValidationCategory>,
}

impl ValidationReport {
    pub fn total_checks(&self) -> usize {
        self.categories
            .iter()
            .map(|category| category.checks.len())
            .sum()
    }

    pub fn passed_checks(&self) -> usize {
        self.categories
            .iter()
            .map(|category| category.checks.iter().filter(|check| check.passed).count())
            .sum()
    }

    pub fn is_pass(&self) -> bool {
        self.total_checks() == self.passed_checks()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationCategory {
    pub name: String,
    pub checks: Vec<ValidationCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationCheck {
    pub label: String,
    pub passed: bool,
    pub detail: String,
}

pub type ResponseDetails = BTreeMap<String, Value>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageSummary {
    pub name: String,
    pub binary: String,
    pub version: String,
    pub description: String,
    pub project_format: String,
    pub skill_path: String,
    pub command_groups: Vec<String>,
    pub supports_json: bool,
    pub repl_default: bool,
}

impl PackageSummary {
    pub fn new(
        name: impl Into<String>,
        binary: impl Into<String>,
        version: impl Into<String>,
        description: impl Into<String>,
        project_format: impl Into<String>,
        skill_path: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            binary: binary.into(),
            version: version.into(),
            description: description.into(),
            project_format: project_format.into(),
            skill_path: skill_path.into(),
            command_groups: Vec::new(),
            supports_json: false,
            repl_default: false,
        }
    }

    pub fn with_command_groups(
        mut self,
        command_groups: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.command_groups = command_groups.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_modes(mut self, supports_json: bool, repl_default: bool) -> Self {
        self.supports_json = supports_json;
        self.repl_default = repl_default;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandResponse {
    pub software: String,
    pub binary: String,
    pub group: String,
    pub command: String,
    pub description: String,
    #[serde(flatten)]
    pub details: ResponseDetails,
}

impl CommandResponse {
    pub fn new(
        software: impl Into<String>,
        binary: impl Into<String>,
        group: impl Into<String>,
        command: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            software: software.into(),
            binary: binary.into(),
            group: group.into(),
            command: command.into(),
            description: description.into(),
            details: ResponseDetails::new(),
        }
    }

    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }

    pub fn with_details(mut self, details: ResponseDetails) -> Self {
        self.details.extend(details);
        self
    }
}

pub fn plugin_metadata() -> PluginMetadata {
    PluginMetadata {
        name: "cli-anything".to_string(),
        description:
            "Build powerful, stateful CLI interfaces for any GUI application using the cli-anything harness methodology.".to_string(),
        author: "cli-anything contributors".to_string(),
    }
}

pub fn builtin_command_documents() -> Vec<CommandDocument> {
    vec![
        command_document(BuiltinCommandId::Build),
        command_document(BuiltinCommandId::Refine),
        command_document(BuiltinCommandId::Test),
        command_document(BuiltinCommandId::Validate),
        command_document(BuiltinCommandId::List),
    ]
}

pub fn command_document(id: BuiltinCommandId) -> CommandDocument {
    match id {
        BuiltinCommandId::Build => CommandDocument {
            id,
            title: "cli-anything".to_string(),
            usage: "cli-anything <software-path-or-repo>".to_string(),
            summary: "Build a complete Rust CLI package for any software target.".to_string(),
            phases: vec![
                phase(
                    "Phase 0: Source Acquisition",
                    &[
                        "Accept a local source path or a GitHub repository URL.",
                        "Derive the software name from the local directory or repository slug.",
                        "Prepare the Rust package destination under packages/<software>/.",
                    ],
                ),
                phase(
                    "Phase 1: Codebase Analysis",
                    &[
                        "Analyze the source tree, backend engine, and data model.",
                        "Map GUI actions to APIs and existing command-line tools.",
                    ],
                ),
                phase(
                    "Phase 2: CLI Architecture Design",
                    &[
                        "Design command groups, state handling, and JSON output.",
                        "Create the software-specific SOP and package manifest.",
                    ],
                ),
                phase(
                    "Phase 3: Rust Implementation",
                    &[
                        "Create packages/<software>/ with src/, tests/, skills/, and fixtures/.",
                        "Generate a Rust entry point, package manifest, and skill skeleton.",
                    ],
                ),
            ],
            success_criteria: vec![
                "A Rust package scaffold exists under packages/<software>/.".to_string(),
                "The generated package includes cli-anything.toml and src/main.rs.".to_string(),
                "The package is ready for skill generation, validation, and testing.".to_string(),
            ],
        },
        BuiltinCommandId::Refine => CommandDocument {
            id,
            title: "cli-anything refine".to_string(),
            usage: "cli-anything refine <software-path> [focus]".to_string(),
            summary: "Refine an existing Rust package to improve command coverage.".to_string(),
            phases: vec![
                phase(
                    "Step 1: Inventory Current Coverage",
                    &[
                        "Inspect the package manifest, command groups, and tests.",
                        "Build a coverage map for the current Rust package.",
                    ],
                ),
                phase(
                    "Step 2: Analyze Software Capabilities",
                    &[
                        "Re-scan the source code for missing APIs and workflows.",
                        "Narrow the scope when a focus string is provided.",
                    ],
                ),
                phase(
                    "Step 3: Gap Analysis and Expansion",
                    &[
                        "Prioritize missing commands by impact and composability.",
                        "Plan new command groups, tests, and documentation updates.",
                    ],
                ),
            ],
            success_criteria: vec![
                "The refine plan identifies concrete capability gaps.".to_string(),
                "Focused refinement narrows the analysis to the requested area.".to_string(),
            ],
        },
        BuiltinCommandId::Test => CommandDocument {
            id,
            title: "cli-anything test".to_string(),
            usage: "cli-anything test <software-path-or-repo>".to_string(),
            summary: "Prepare and run the Rust test workflow for a generated package.".to_string(),
            phases: vec![phase(
                "Test Workflow",
                &[
                    "Locate the Rust package in packages/<software>/.",
                    "Run cargo test for the workspace or specific package.",
                    "Capture results for TEST.md and subprocess verification.",
                ],
            )],
            success_criteria: vec![
                "All tests pass for the target package.".to_string(),
                "The test output is available for appending to TEST.md.".to_string(),
            ],
        },
        BuiltinCommandId::Validate => CommandDocument {
            id,
            title: "cli-anything validate".to_string(),
            usage: "cli-anything validate <software-path-or-repo>".to_string(),
            summary: "Validate a Rust package against the CLI-Anything-RS package layout."
                .to_string(),
            phases: vec![phase(
                "Validation Checks",
                &[
                    "Verify packages/<software>/ exists.",
                    "Check Cargo.toml, cli-anything.toml, src/main.rs, skills/, tests/, and fixtures/.",
                    "Summarize results as a structured validation report.",
                ],
            )],
            success_criteria: vec![
                "Every required Rust package file and directory is present.".to_string(),
                "The package passes all validation checks.".to_string(),
            ],
        },
        BuiltinCommandId::List => CommandDocument {
            id,
            title: "cli-anything list".to_string(),
            usage: "cli-anything list [--path <directory>] [--depth <n>] [--json]".to_string(),
            summary: "List locally generated Rust packages and their metadata.".to_string(),
            phases: vec![phase(
                "Discovery",
                &[
                    "Scan the selected directory for packages/<software>/cli-anything.toml.",
                    "Read each package manifest and summarize status, version, and source path.",
                ],
            )],
            success_criteria: vec![
                "The command returns all matching Rust packages.".to_string(),
                "Table and JSON output remain stable and machine-readable.".to_string(),
            ],
        },
    }
}

pub fn parse_source_target(input: &str) -> Result<SourceTarget> {
    let raw = input.trim();
    if raw.is_empty() {
        anyhow::bail!("source path or repository URL cannot be empty");
    }

    if raw.starts_with("https://github.com/") || raw.starts_with("github.com/") {
        return Ok(SourceTarget {
            raw: raw.to_string(),
            software_name: derive_software_name(raw)?,
            kind: SourceTargetKind::GitHubRepo(raw.to_string()),
        });
    }

    let path = PathBuf::from(raw);
    Ok(SourceTarget {
        raw: raw.to_string(),
        software_name: derive_software_name(raw)?,
        kind: SourceTargetKind::LocalPath(path),
    })
}

pub fn derive_software_name(input: &str) -> Result<String> {
    let trimmed = input.trim().trim_end_matches('/');
    let candidate = trimmed
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.trim_end_matches(".git"))
        .filter(|segment| !segment.is_empty())
        .ok_or_else(|| anyhow::anyhow!("unable to derive software name from {input}"))?;

    Ok(candidate.to_lowercase())
}

pub fn package_layout(workspace_root: &Path, software_name: &str) -> PackageLayout {
    let root = workspace_root.join("packages").join(software_name);
    let src_dir = root.join("src");
    let skills_dir = root.join("skills");
    PackageLayout {
        software_name: software_name.to_string(),
        cargo_toml: root.join("Cargo.toml"),
        manifest: root.join("cli-anything.toml"),
        src_main: src_dir.join("main.rs"),
        skill_file: skills_dir.join("SKILL.md"),
        tests_dir: root.join("tests"),
        fixtures_dir: root.join("fixtures"),
        root,
        src_dir,
        skills_dir,
    }
}

fn phase(title: &str, items: &[&str]) -> WorkflowPhase {
    WorkflowPhase {
        title: title.to_string(),
        items: items.iter().map(|item| item.to_string()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_github_source_target() {
        let target =
            parse_source_target("https://github.com/blender/blender").expect("target should parse");

        assert!(target.is_remote());
        assert_eq!(target.software_name, "blender");
    }

    #[test]
    fn parses_local_source_target() {
        let target = parse_source_target("./obs-studio").expect("target should parse");

        assert!(!target.is_remote());
        assert_eq!(target.software_name, "obs-studio");
    }

    #[test]
    fn package_layout_is_nested_under_packages_directory() {
        let layout = package_layout(Path::new("/tmp/workspace"), "gimp");

        assert_eq!(layout.root, Path::new("/tmp/workspace/packages/gimp"));
        assert_eq!(
            layout.manifest,
            Path::new("/tmp/workspace/packages/gimp/cli-anything.toml")
        );
        assert_eq!(
            layout.skill_file,
            Path::new("/tmp/workspace/packages/gimp/skills/SKILL.md")
        );
    }

    #[test]
    fn exposes_all_builtin_command_documents() {
        let docs = builtin_command_documents();

        assert_eq!(docs.len(), 5);
        assert_eq!(docs[0].id, BuiltinCommandId::Build);
        assert_eq!(docs[4].id, BuiltinCommandId::List);
    }

    #[test]
    fn build_document_mentions_packages_directory() {
        let document = command_document(BuiltinCommandId::Build);

        assert!(document.usage.contains("cli-anything"));
        assert!(
            document
                .phases
                .iter()
                .flat_map(|phase| phase.items.iter())
                .any(|item| item.contains("packages/<software>/"))
        );
    }

    #[test]
    fn package_summary_builder_collects_owned_metadata() {
        let summary = PackageSummary::new(
            "gimp",
            "cli-anything-gimp",
            "1.0.0",
            "Raster image processing via gimp -i -b (batch mode)",
            "xcf",
            "packages/gimp/skills/SKILL.md",
        )
        .with_command_groups(["project", "layer", "filter"])
        .with_modes(true, true);

        assert_eq!(summary.name, "gimp");
        assert_eq!(summary.project_format, "xcf");
        assert_eq!(summary.command_groups, ["project", "layer", "filter"]);
        assert!(summary.supports_json);
    }

    #[test]
    fn command_response_builder_flattens_details_when_serialized() {
        let response = CommandResponse::new(
            "gimp",
            "cli-anything-gimp",
            "project",
            "new",
            "Create a new image project",
        )
        .with_detail("project", json!({ "name": "poster", "width": 2048 }))
        .with_details(std::iter::once(("status".to_string(), json!("queued"))).collect());

        let payload = serde_json::to_value(&response).expect("response should serialize");

        assert_eq!(payload["group"], "project");
        assert_eq!(payload["project"]["name"], "poster");
        assert_eq!(payload["status"], "queued");
    }
}
