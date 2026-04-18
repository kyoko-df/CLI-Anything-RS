//! Regenerate the curated packages' SKILL.md files from their manifests.
//!
//! Run from the workspace root:
//!
//! ```bash
//! cargo run --example regen_skills -p cli-anything-skillgen
//! ```
//!
//! Used whenever the skillgen output format changes so the curated
//! packages (`gimp`, `blender`, `drawio`) stay in sync with the
//! templates.

use std::path::PathBuf;

use cli_anything_core::load_manifest_from_path;
use cli_anything_skillgen::generate_skill_file;

fn main() -> anyhow::Result<()> {
    let workspace_root = std::env::current_dir()?;
    for software in ["gimp", "blender", "drawio"] {
        let manifest_path = workspace_root
            .join("packages")
            .join(software)
            .join("cli-anything.toml");
        let skill_path: PathBuf = workspace_root
            .join("packages")
            .join(software)
            .join("skills")
            .join("SKILL.md");
        let manifest = load_manifest_from_path(&manifest_path)?;
        let written = generate_skill_file(&manifest, Some(&skill_path))?;
        println!("regenerated {}", written.display());
    }
    Ok(())
}
