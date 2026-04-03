use cli_anything_core::CliAnythingManifest;

pub fn render_skill_markdown(manifest: &CliAnythingManifest) -> String {
    format!(
        "---\nname: {}\ndescription: {}\n---\n\n# {}\n\n{}\n",
        manifest.binary, manifest.description, manifest.binary, manifest.description
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cli_anything_core::parse_manifest;

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
output = "crates/shotcut-cli/skills/SKILL.md"
template = "templates/SKILL.md.template"
"#;

    #[test]
    fn renders_frontmatter_from_manifest() {
        let manifest = parse_manifest(SAMPLE_MANIFEST).expect("manifest should parse");
        let markdown = render_skill_markdown(&manifest);

        assert!(markdown.contains("name: cli-anything-shotcut"));
        assert!(markdown.contains("# cli-anything-shotcut"));
    }
}
