use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliAnythingManifest {
    pub name: String,
    pub version: String,
    pub binary: String,
    pub description: String,
    #[serde(default)]
    pub repl_default: bool,
    #[serde(default)]
    pub supports_json: bool,
    pub backend: BackendConfig,
    pub project: ProjectConfig,
    pub skill: SkillConfig,
    #[serde(default)]
    pub command_groups: Vec<CommandGroup>,
    #[serde(default)]
    pub examples: Vec<ExampleSpec>,
}

impl CliAnythingManifest {
    pub fn package_name(&self) -> &str {
        &self.binary
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendConfig {
    pub command: String,
    pub system_package: String,
    #[serde(default = "default_true")]
    pub hard_dependency: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub format: String,
    pub state_file: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillConfig {
    pub output: String,
    pub template: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandGroup {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub commands: Vec<CommandSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSpec {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExampleSpec {
    pub title: String,
    pub description: String,
    pub code: String,
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownPackageSpec {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub backend_command: String,
    pub system_package: String,
    pub project_format: String,
    pub state_file: String,
    pub command_groups: Vec<CommandGroup>,
    pub examples: Vec<ExampleSpec>,
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

pub fn parse_manifest(input: &str) -> Result<CliAnythingManifest> {
    toml::from_str::<CliAnythingManifest>(input).context("failed to parse cli-anything manifest")
}

pub fn load_manifest_from_path(path: &Path) -> Result<CliAnythingManifest> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read manifest at {}", path.display()))?;
    parse_manifest(&content)
}

pub fn builtin_package_specs() -> Vec<KnownPackageSpec> {
    vec![
        gimp_package_spec(),
        blender_package_spec(),
        drawio_package_spec(),
    ]
}

pub fn builtin_package_spec(name: &str) -> Option<KnownPackageSpec> {
    builtin_package_specs()
        .into_iter()
        .find(|spec| spec.name == name.trim().to_lowercase())
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

fn default_true() -> bool {
    true
}

fn gimp_package_spec() -> KnownPackageSpec {
    KnownPackageSpec {
        name: "gimp".to_string(),
        display_name: "GIMP".to_string(),
        description: "Raster image processing via gimp -i -b (batch mode)".to_string(),
        backend_command: "gimp".to_string(),
        system_package: "gimp (apt install gimp)".to_string(),
        project_format: "xcf".to_string(),
        state_file: ".gimp-cli.json".to_string(),
        command_groups: vec![
            command_group(
                "project",
                "Project management commands",
                &[
                    ("new", "Create a new image project"),
                    ("info", "Show project information"),
                ],
            ),
            command_group(
                "layer",
                "Layer management commands",
                &[
                    ("new", "Create a blank layer"),
                    ("list", "List project layers"),
                ],
            ),
            command_group(
                "canvas",
                "Canvas inspection and resize commands",
                &[
                    ("info", "Show canvas metadata"),
                    ("resize", "Resize the canvas"),
                ],
            ),
            command_group(
                "filter",
                "Filter application commands",
                &[
                    ("add", "Apply a filter to a layer"),
                    ("list", "List supported filters"),
                ],
            ),
            command_group(
                "media",
                "Media import and asset inspection commands",
                &[
                    ("import", "Import media into the project"),
                    ("list", "List project media"),
                ],
            ),
            command_group(
                "export",
                "Export commands",
                &[
                    ("image", "Export the current composition"),
                    ("presets", "List export presets"),
                ],
            ),
            command_group(
                "session",
                "Session history and persistence commands",
                &[
                    ("status", "Show session status"),
                    ("undo", "Undo the last action"),
                ],
            ),
            command_group(
                "draw",
                "Drawing primitive commands",
                &[("line", "Draw a line"), ("rectangle", "Draw a rectangle")],
            ),
        ],
        examples: vec![
            example(
                "Create poster",
                "Create a new raster project for poster work.",
                "cli-anything-gimp project new --width 1920 --height 1080 -o poster.json",
            ),
            example(
                "Add filter",
                "Apply a brightness filter to the active layer.",
                "cli-anything-gimp filter add brightness --layer 0 --param factor=1.3",
            ),
        ],
    }
}

fn blender_package_spec() -> KnownPackageSpec {
    KnownPackageSpec {
        name: "blender".to_string(),
        display_name: "Blender".to_string(),
        description: "3D modeling, animation, and rendering via blender --background --python"
            .to_string(),
        backend_command: "blender".to_string(),
        system_package: "blender".to_string(),
        project_format: "blend".to_string(),
        state_file: ".blender-cli.json".to_string(),
        command_groups: vec![
            command_group(
                "scene",
                "Scene management commands",
                &[
                    ("new", "Create a new scene"),
                    ("info", "Inspect the active scene"),
                ],
            ),
            command_group(
                "object",
                "Object creation and transformation commands",
                &[("add", "Add a new object"), ("list", "List scene objects")],
            ),
            command_group(
                "material",
                "Material authoring commands",
                &[("assign", "Assign a material"), ("list", "List materials")],
            ),
            command_group(
                "modifier",
                "Modifier stack commands",
                &[("add", "Add a modifier"), ("apply", "Apply a modifier")],
            ),
            command_group(
                "camera",
                "Camera rig commands",
                &[("add", "Add a camera"), ("list", "List cameras")],
            ),
            command_group(
                "light",
                "Lighting commands",
                &[("add", "Add a light"), ("list", "List lights")],
            ),
            command_group(
                "animation",
                "Animation timeline commands",
                &[
                    ("keyframe", "Insert a keyframe"),
                    ("playblast", "Preview the animation"),
                ],
            ),
            command_group(
                "render",
                "Rendering commands",
                &[
                    ("frame", "Render a frame"),
                    ("info", "Inspect render settings"),
                ],
            ),
            command_group(
                "session",
                "Session tracking commands",
                &[
                    ("status", "Show session state"),
                    ("history", "Inspect action history"),
                ],
            ),
        ],
        examples: vec![
            example(
                "Create scene",
                "Create a fresh Blender scene file.",
                "cli-anything-blender scene new -o demo.blend",
            ),
            example(
                "Render frame",
                "Render the active frame to a PNG file.",
                "cli-anything-blender render frame --output frame.png",
            ),
        ],
    }
}

fn drawio_package_spec() -> KnownPackageSpec {
    KnownPackageSpec {
        name: "drawio".to_string(),
        display_name: "Draw.io".to_string(),
        description: "Diagram creation and export via draw.io CLI".to_string(),
        backend_command: "draw.io".to_string(),
        system_package: "draw.io desktop app".to_string(),
        project_format: "drawio".to_string(),
        state_file: ".drawio-cli.json".to_string(),
        command_groups: vec![
            command_group(
                "project",
                "Diagram project commands",
                &[
                    ("new", "Create a new diagram"),
                    ("info", "Show project metadata"),
                ],
            ),
            command_group(
                "shape",
                "Shape creation commands",
                &[("add", "Add a shape"), ("types", "List shape types")],
            ),
            command_group(
                "connect",
                "Connector authoring commands",
                &[
                    ("add", "Create a connector"),
                    ("styles", "List connector styles"),
                ],
            ),
            command_group(
                "page",
                "Page management commands",
                &[("add", "Add a page"), ("list", "List pages")],
            ),
            command_group(
                "export",
                "Export commands",
                &[("diagram", "Export a diagram"), ("formats", "List formats")],
            ),
            command_group(
                "session",
                "Session management commands",
                &[
                    ("status", "Show current session"),
                    ("save", "Persist session state"),
                ],
            ),
        ],
        examples: vec![
            example(
                "Create diagram",
                "Create a new diagram file.",
                "cli-anything-drawio project new -o architecture.drawio",
            ),
            example(
                "Add rectangle",
                "Add a rectangle shape to the current page.",
                "cli-anything-drawio shape add rectangle --text API",
            ),
        ],
    }
}

fn command_group(name: &str, description: &str, commands: &[(&str, &str)]) -> CommandGroup {
    CommandGroup {
        name: name.to_string(),
        description: description.to_string(),
        commands: commands
            .iter()
            .map(|(name, description)| CommandSpec {
                name: (*name).to_string(),
                description: (*description).to_string(),
            })
            .collect(),
    }
}

fn example(title: &str, description: &str, code: &str) -> ExampleSpec {
    ExampleSpec {
        title: title.to_string(),
        description: description.to_string(),
        code: code.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BuiltinCommandId, CommandResponse, PackageSummary, builtin_command_documents,
        builtin_package_spec, builtin_package_specs, command_document, package_layout,
        parse_manifest, parse_source_target,
    };
    use serde_json::json;
    use std::path::Path;

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

[[command_groups]]
name = "project"
description = "Project lifecycle commands"

[[command_groups.commands]]
name = "new"
description = "Create a new project"

[[examples]]
title = "Create project"
description = "Create a new Shotcut project"
code = "cli-anything-shotcut project new -o demo.mlt"
"#;

    #[test]
    fn parses_manifest_with_nested_sections() {
        let manifest = parse_manifest(SAMPLE_MANIFEST).expect("manifest should parse");

        assert_eq!(manifest.name, "shotcut");
        assert_eq!(manifest.backend.command, "melt");
        assert_eq!(manifest.command_groups.len(), 1);
        assert_eq!(manifest.command_groups[0].commands[0].name, "new");
        assert_eq!(manifest.examples.len(), 1);
    }

    #[test]
    fn defaults_hard_dependency_to_true_when_omitted() {
        let input = r#"
name = "inkscape"
version = "1.0.0"
binary = "cli-anything-inkscape"
description = "Rust CLI harness for Inkscape"
repl_default = true
supports_json = true

[backend]
command = "inkscape"
system_package = "inkscape"

[project]
format = "svg"
state_file = ".inkscape-cli.json"

[skill]
output = "packages/inkscape/skills/SKILL.md"
template = "templates/skill/SKILL.md.template"
"#;

        let manifest = parse_manifest(input).expect("manifest should parse");

        assert!(manifest.backend.hard_dependency);
    }

    #[test]
    fn rejects_manifest_without_backend_section() {
        let input = r#"
name = "gimp"
version = "1.0.0"
binary = "cli-anything-gimp"
description = "Rust CLI harness for GIMP"
repl_default = true
supports_json = true

[project]
format = "xcf"
state_file = ".gimp-cli.json"

[skill]
output = "packages/gimp/skills/SKILL.md"
template = "templates/skill/SKILL.md.template"
"#;

        let error = parse_manifest(input).expect_err("manifest should be rejected");

        let chain = error
            .chain()
            .map(|cause| cause.to_string())
            .collect::<Vec<_>>();
        assert!(chain.iter().any(|message| message.contains("backend")));
    }

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
    fn includes_curated_package_specs_for_second_stage() {
        let specs = builtin_package_specs();

        assert_eq!(specs.len(), 3);
        assert!(specs.iter().any(|spec| spec.name == "gimp"));
        assert!(specs.iter().any(|spec| spec.name == "blender"));
        assert!(specs.iter().any(|spec| spec.name == "drawio"));
    }

    #[test]
    fn gimp_curated_package_spec_exposes_expected_command_groups() {
        let spec = builtin_package_spec("gimp").expect("gimp spec should exist");

        assert_eq!(spec.project_format, "xcf");
        assert!(spec.command_groups.iter().any(|group| group.name == "layer"
            && group.commands.iter().any(|command| command.name == "list")));
        assert!(
            spec.command_groups
                .iter()
                .any(|group| group.name == "filter")
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
