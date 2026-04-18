use std::io::{self, BufRead, Write};

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

/// Outcome reported by a `Repl` dispatcher for a parsed command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchOutcome {
    /// Command succeeded; render `message` to the user and keep looping.
    Rendered(String),
    /// Command failed; render `message` as an error and keep looping.
    Failed(String),
    /// User asked to exit the REPL.
    Exit,
}

/// Shell-like whitespace tokenizer with quoted-string support.
/// Unknown escapes are preserved verbatim. Returns `Err` on an unterminated
/// quote so the REPL can surface a clear error instead of swallowing input.
pub fn tokenize(line: &str) -> Result<Vec<String>, String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_token = false;
    let mut quote: Option<char> = None;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match quote {
            Some(q) => {
                if ch == q {
                    quote = None;
                } else if ch == '\\' {
                    if let Some(&next) = chars.peek() {
                        current.push(next);
                        chars.next();
                    }
                } else {
                    current.push(ch);
                }
            }
            None => {
                if ch.is_whitespace() {
                    if in_token {
                        tokens.push(std::mem::take(&mut current));
                        in_token = false;
                    }
                } else if ch == '"' || ch == '\'' {
                    in_token = true;
                    quote = Some(ch);
                } else if ch == '\\' {
                    in_token = true;
                    if let Some(&next) = chars.peek() {
                        current.push(next);
                        chars.next();
                    }
                } else {
                    in_token = true;
                    current.push(ch);
                }
            }
        }
    }

    if quote.is_some() {
        return Err("unterminated quoted string".to_string());
    }
    if in_token {
        tokens.push(current);
    }
    Ok(tokens)
}

/// Interactive read-evaluate loop that delegates command parsing to the
/// caller. The REPL owns the prompt rendering, builtin commands (help,
/// exit, quit, clear), tokenization, and error display; everything else
/// flows through the `dispatch` closure.
#[derive(Debug, Clone)]
pub struct Repl {
    skin: Skin,
    project_name: String,
    context: String,
    modified: bool,
}

impl Repl {
    pub fn new(skin: Skin) -> Self {
        Self {
            skin,
            project_name: String::new(),
            context: String::new(),
            modified: false,
        }
    }

    pub fn with_project_name(mut self, project_name: impl Into<String>) -> Self {
        self.project_name = project_name.into();
        self
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = context.into();
        self
    }

    pub fn with_modified(mut self, modified: bool) -> Self {
        self.modified = modified;
        self
    }

    pub fn set_project_name(&mut self, project_name: impl Into<String>) {
        self.project_name = project_name.into();
    }

    pub fn set_context(&mut self, context: impl Into<String>) {
        self.context = context.into();
    }

    pub fn set_modified(&mut self, modified: bool) {
        self.modified = modified;
    }

    pub fn skin(&self) -> &Skin {
        &self.skin
    }

    /// Drive the REPL against arbitrary readers/writers. Tests inject
    /// `Cursor::new(...)` / `Vec<u8>`; production code passes stdin/stdout.
    pub fn run<R, W, F>(&mut self, reader: R, mut writer: W, mut dispatch: F) -> io::Result<()>
    where
        R: BufRead,
        W: Write,
        F: FnMut(&[String]) -> DispatchOutcome,
    {
        for line in self.skin.banner_lines() {
            writeln!(writer, "{line}")?;
        }

        let mut lines = reader.lines();
        loop {
            write!(
                writer,
                "{}",
                self.skin
                    .prompt(&self.project_name, self.modified, &self.context)
            )?;
            writer.flush()?;

            let Some(next) = lines.next() else {
                writeln!(writer)?;
                writeln!(writer, "{}", self.skin.goodbye())?;
                break;
            };
            let raw = next?;
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            if matches!(trimmed, "exit" | "quit" | ":q") {
                writeln!(writer, "{}", self.skin.goodbye())?;
                break;
            }
            if matches!(trimmed, "help" | "?") {
                writeln!(
                    writer,
                    "{}",
                    self.skin.info("Builtin: help, exit, quit, clear")
                )?;
                writeln!(
                    writer,
                    "{}",
                    self.skin
                        .info("All other input is dispatched to the package CLI")
                )?;
                continue;
            }
            if matches!(trimmed, "clear") {
                write!(writer, "\x1b[2J\x1b[H")?;
                writer.flush()?;
                continue;
            }

            let tokens = match tokenize(trimmed) {
                Ok(tokens) => tokens,
                Err(err) => {
                    writeln!(writer, "{}", self.skin.error(&err))?;
                    continue;
                }
            };
            if tokens.is_empty() {
                continue;
            }

            match dispatch(&tokens) {
                DispatchOutcome::Rendered(message) => {
                    if !message.is_empty() {
                        writeln!(writer, "{message}")?;
                    }
                }
                DispatchOutcome::Failed(message) => {
                    writeln!(writer, "{}", self.skin.error(&message))?;
                }
                DispatchOutcome::Exit => {
                    writeln!(writer, "{}", self.skin.goodbye())?;
                    break;
                }
            }
        }

        Ok(())
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
    fn tokenize_handles_quoted_segments_and_escapes() {
        let tokens = tokenize(r#"project new --name "Weekly Report" --label can\'t"#)
            .expect("tokenize should succeed");

        assert_eq!(
            tokens,
            vec![
                "project".to_string(),
                "new".to_string(),
                "--name".to_string(),
                "Weekly Report".to_string(),
                "--label".to_string(),
                "can't".to_string(),
            ]
        );
    }

    #[test]
    fn tokenize_rejects_unterminated_quotes() {
        let err = tokenize(r#"project new --name "broken"#).expect_err("expected parse error");
        assert!(err.contains("unterminated"));
    }

    #[test]
    fn repl_loop_dispatches_and_exits_on_exit() {
        use std::io::Cursor;

        let skin = Skin::new("gimp", "1.0.0");
        let mut repl = Repl::new(skin).with_project_name("demo.xcf");
        let reader = Cursor::new(b"project new --name demo\nexit\n" as &[u8]);
        let mut writer: Vec<u8> = Vec::new();
        let mut calls: Vec<Vec<String>> = Vec::new();

        repl.run(reader, &mut writer, |tokens| {
            calls.push(tokens.to_vec());
            DispatchOutcome::Rendered(format!("ok({})", tokens.join(" ")))
        })
        .expect("repl should run");

        let out = String::from_utf8(writer).unwrap();
        assert!(
            out.contains("cli-anything · Gimp · v1.0.0"),
            "banner missing from:\n{out}"
        );
        assert!(
            out.contains("gimp:demo.xcf> "),
            "prompt missing from:\n{out}"
        );
        assert!(out.contains("ok(project new --name demo)"));
        assert!(out.contains("Session closed for Gimp."));
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0],
            vec!["project", "new", "--name", "demo"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn repl_loop_renders_failures_without_exiting() {
        use std::io::Cursor;

        let skin = Skin::new("blender", "1.0.0");
        let mut repl = Repl::new(skin);
        let reader = Cursor::new(b"boom\nexit\n" as &[u8]);
        let mut writer: Vec<u8> = Vec::new();

        repl.run(reader, &mut writer, |_| {
            DispatchOutcome::Failed("unknown command".to_string())
        })
        .expect("repl should run");

        let out = String::from_utf8(writer).unwrap();
        assert!(out.contains("✘ unknown command"), "error missing:\n{out}");
        assert!(out.contains("Session closed"), "goodbye missing:\n{out}");
    }

    #[test]
    fn repl_loop_builtins_show_help_and_skip_empty() {
        use std::io::Cursor;

        let skin = Skin::new("drawio", "1.0.0");
        let mut repl = Repl::new(skin);
        let reader = Cursor::new(b"\nhelp\nexit\n" as &[u8]);
        let mut writer: Vec<u8> = Vec::new();
        let mut calls = 0;

        repl.run(reader, &mut writer, |_| {
            calls += 1;
            DispatchOutcome::Rendered(String::new())
        })
        .expect("repl should run");

        assert_eq!(calls, 0, "dispatcher should never be invoked for builtins");
        let out = String::from_utf8(writer).unwrap();
        assert!(out.contains("Builtin: help"), "help missing:\n{out}");
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
