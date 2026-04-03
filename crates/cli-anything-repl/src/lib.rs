#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skin {
    software: String,
    version: String,
    skill_path: Option<String>,
}

impl Skin {
    pub fn new(software: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            software: normalize_software_name(&software.into()),
            version: version.into(),
            skill_path: None,
        }
    }

    pub fn with_skill_path(mut self, skill_path: impl Into<String>) -> Self {
        self.skill_path = Some(skill_path.into());
        self
    }

    pub fn software(&self) -> &str {
        &self.software
    }

    pub fn display_name(&self) -> String {
        self.software
            .split(['_', '-'])
            .filter(|segment| !segment.is_empty())
            .map(title_case)
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn banner_title(&self) -> String {
        format!("cli-anything · {} · v{}", self.display_name(), self.version)
    }

    pub fn banner_lines(&self) -> Vec<String> {
        let mut lines = vec![
            self.banner_title(),
            format!("Software: {}", self.software),
            "Type help for commands, exit to leave the session.".to_string(),
        ];

        if let Some(skill_path) = &self.skill_path {
            lines.push(format!("Skill: {}", skill_path));
        }

        lines
    }

    pub fn prompt(&self, project_name: &str, modified: bool, context: &str) -> String {
        let mut scope = if project_name.trim().is_empty() {
            self.software.clone()
        } else {
            format!("{}:{}", self.software, project_name.trim())
        };

        if !context.trim().is_empty() {
            scope.push('[');
            scope.push_str(context.trim());
            scope.push(']');
        }

        if modified {
            scope.push('*');
        }

        format!("{scope}> ")
    }

    pub fn success(&self, message: &str) -> String {
        format!("✔ {}", message.trim())
    }

    pub fn error(&self, message: &str) -> String {
        format!("✘ {}", message.trim())
    }

    pub fn warning(&self, message: &str) -> String {
        format!("! {}", message.trim())
    }

    pub fn info(&self, message: &str) -> String {
        format!("• {}", message.trim())
    }

    pub fn status(&self, label: &str, value: &str) -> String {
        format!("{:<12} {}", label.trim(), value.trim())
    }

    pub fn progress(&self, label: &str, current: usize, total: usize) -> String {
        format!("[{current}/{total}] {}", label.trim())
    }

    pub fn goodbye(&self) -> String {
        format!("Session closed for {}.", self.display_name())
    }

    pub fn format_table(&self, headers: &[&str], rows: &[Vec<String>]) -> String {
        if headers.is_empty() {
            return String::new();
        }

        let mut widths = headers
            .iter()
            .map(|header| header.len())
            .collect::<Vec<_>>();
        for row in rows {
            for (index, value) in row.iter().enumerate() {
                if index < widths.len() {
                    widths[index] = widths[index].max(value.len());
                }
            }
        }

        let mut lines = Vec::with_capacity(rows.len() + 2);
        lines.push(render_row(
            headers.iter().map(|header| header.to_string()).collect(),
            &widths,
        ));
        lines.push(
            widths
                .iter()
                .map(|width| "-".repeat(*width))
                .collect::<Vec<_>>()
                .join("-+-"),
        );
        for row in rows {
            lines.push(render_row(row.clone(), &widths));
        }

        lines.join("\n")
    }
}

fn normalize_software_name(input: &str) -> String {
    input.trim().to_lowercase().replace(' ', "-")
}

fn title_case(segment: &str) -> String {
    let mut characters = segment.chars();
    match characters.next() {
        Some(first) => {
            let mut title = first.to_uppercase().collect::<String>();
            title.push_str(&characters.as_str().to_lowercase());
            title
        }
        None => String::new(),
    }
}

fn render_row(values: Vec<String>, widths: &[usize]) -> String {
    widths
        .iter()
        .enumerate()
        .map(|(index, width)| {
            let value = values.get(index).cloned().unwrap_or_default();
            format!("{value:<width$}")
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_display_name_for_prompt_and_banner() {
        let skin = Skin::new("obs_studio", "0.1.0");

        assert_eq!(skin.display_name(), "Obs Studio");
        assert_eq!(skin.software(), "obs_studio");
    }

    #[test]
    fn builds_banner_title() {
        let skin = Skin::new("shotcut", "0.1.0");

        assert_eq!(skin.banner_title(), "cli-anything · Shotcut · v0.1.0");
    }

    #[test]
    fn includes_skill_path_in_banner_lines() {
        let skin = Skin::new("drawio", "1.2.0").with_skill_path("skills/SKILL.md");
        let banner = skin.banner_lines().join("\n");

        assert!(banner.contains("drawio"));
        assert!(banner.contains("skills/SKILL.md"));
    }

    #[test]
    fn builds_prompt_with_project_context_and_modified_marker() {
        let skin = Skin::new("blender", "0.1.0");

        assert_eq!(
            skin.prompt("demo.blend", true, "object-mode"),
            "blender:demo.blend[object-mode]*> "
        );
    }

    #[test]
    fn formats_table_with_headers_and_rows() {
        let skin = Skin::new("gimp", "0.1.0");
        let table = skin.format_table(
            &["name", "status"],
            &[vec!["gimp".to_string(), "ready".to_string()]],
        );

        assert!(table.contains("name"));
        assert!(table.contains("status"));
        assert!(table.contains("gimp"));
        assert!(table.contains("ready"));
    }
}
