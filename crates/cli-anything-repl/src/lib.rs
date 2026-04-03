#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skin {
    software: String,
    version: String,
}

impl Skin {
    pub fn new(software: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            software: software.into(),
            version: version.into(),
        }
    }

    pub fn display_name(&self) -> String {
        self.software.replace(['_', '-'], " ")
    }

    pub fn banner_title(&self) -> String {
        format!("cli-anything · {} · v{}", self.display_name(), self.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_display_name_for_prompt_and_banner() {
        let skin = Skin::new("obs_studio", "0.1.0");

        assert_eq!(skin.display_name(), "obs studio");
    }

    #[test]
    fn builds_banner_title() {
        let skin = Skin::new("shotcut", "0.1.0");

        assert_eq!(skin.banner_title(), "cli-anything · shotcut · v0.1.0");
    }
}
