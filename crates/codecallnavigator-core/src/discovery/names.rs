//! Best-effort display metadata, never evaluated as build code or used for identity.
use roxmltree::{Document, Node};
use std::{fs, iter::Peekable, path::Path, str::Chars, sync::OnceLock};

#[derive(Clone, Copy)]
pub(super) enum Kind {
    Maven,
    Gradle,
    Project,
    Solution,
    TypeScript,
}

pub(super) fn csharp_label(path: &Path) -> String {
    let kind = if path
        .extension()
        .is_some_and(|extension| extension == "csproj")
    {
        Kind::Project
    } else {
        Kind::Solution
    };
    label(path, kind)
}

pub(super) fn label(path: &Path, kind: Kind) -> String {
    let directory_target = matches!(kind, Kind::Maven | Kind::Gradle);
    let folder = if directory_target {
        path
    } else {
        path.parent().unwrap_or(Path::new("."))
    };
    let fallback = if directory_target {
        basename(path)
    } else {
        path.file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    };
    let (name, suffix) = match kind {
        Kind::Maven => (maven_name(&path.join("pom.xml")), " (Maven)"),
        Kind::Gradle => (gradle_name(path), " (Gradle)"),
        Kind::Project => (csharp_name(path), " (C# project)"),
        Kind::Solution => (None, " (Solution)"),
        Kind::TypeScript => return format!("{} / {}", basename(folder), basename(path)),
    };
    format!(
        "{} / {}{suffix}",
        basename(folder),
        name.unwrap_or(fallback)
    )
}

fn basename(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

fn literal(value: &str) -> Option<String> {
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    (!value.is_empty()
        && !value.contains(['$', '`'])
        && !value.contains("@(")
        && !value.contains("%("))
    .then_some(value)
}

fn text(node: Node<'_, '_>) -> Option<String> {
    if node.children().any(|child| child.is_element()) {
        return None;
    }
    let value: String = node
        .children()
        .filter(|child| child.is_text())
        .filter_map(|child| child.text())
        .collect();
    literal(&value)
}

fn maven_name(path: &Path) -> Option<String> {
    let source = fs::read_to_string(path).ok()?;
    let document = Document::parse(&source).ok()?;
    let root = document.root_element();
    if !root.has_tag_name("project") {
        return None;
    }
    ["name", "artifactId"].into_iter().find_map(|tag| {
        root.children()
            .find(|node| node.has_tag_name(tag))
            .and_then(text)
    })
}

fn csharp_name(path: &Path) -> Option<String> {
    let source = fs::read_to_string(path).ok()?;
    let document = Document::parse(&source).ok()?;
    let root = document.root_element();
    if !root.has_tag_name("Project") {
        return None;
    }
    ["ProjectName", "AssemblyName"].into_iter().find_map(|tag| {
        root.children()
            .filter(|group| {
                group.has_tag_name("PropertyGroup") && group.attribute("Condition").is_none()
            })
            .flat_map(|group| group.children())
            .rfind(|node| node.has_tag_name(tag) && node.attribute("Condition").is_none())
            .and_then(text)
    })
}

fn gradle_name(root: &Path) -> Option<String> {
    ["settings.gradle", "settings.gradle.kts"]
        .into_iter()
        .find_map(|file| {
            let source = fs::read_to_string(root.join(file)).ok()?;
            // Multiline literals require language parsing; never treat their examples as facts.
            if source.contains("\"\"\"") || source.contains("'''") {
                return None;
            }
            gradle_literal(&without_comments(&source))
        })
}

fn gradle_literal(source: &str) -> Option<String> {
    static ASSIGNMENT: OnceLock<regex::Regex> = OnceLock::new();
    let assignment = ASSIGNMENT.get_or_init(|| {
        regex::Regex::new(r"^\s*rootProject\s*\.\s*name\s*=\s*(.*)$")
            .expect("constant Gradle assignment pattern")
    });
    let mut depth = 0usize;
    let mut name = None;
    for line in source.lines() {
        if depth == 0
            && let Some(captures) = assignment.captures(line)
        {
            // A later computed assignment must not leave an earlier literal presented as fact.
            name = quoted_literal(&captures[1]);
        }
        depth = scope_depth(line, depth);
    }
    name
}

fn quoted_literal(value: &str) -> Option<String> {
    static QUOTED: OnceLock<regex::Regex> = OnceLock::new();
    let pattern = QUOTED.get_or_init(|| {
        regex::Regex::new(r#"^\s*(?:'([^'\\]*)'|"([^"\\]*)")\s*;?\s*$"#)
            .expect("constant Gradle name pattern")
    });
    let captures = pattern.captures(value)?;
    captures
        .get(1)
        .or_else(|| captures.get(2))
        .and_then(|capture| literal(capture.as_str()))
}

/// Ignore braces inside strings when rejecting assignments nested in conditional blocks.
fn scope_depth(line: &str, mut depth: usize) -> usize {
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' | '"' => {
                quoted(&mut chars, c);
            }
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth
}

/// Preserve quoted strings and newlines so commented assignments cannot become metadata.
fn without_comments(source: &str) -> String {
    let mut result = String::new();
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, chars.peek()) {
            ('/', Some('/')) => {
                chars.by_ref().find(|next| *next == '\n');
                result.push('\n');
            }
            ('/', Some('*')) => {
                chars.next();
                result.push(' ');
                result.push_str(&block_comment(&mut chars));
            }
            ('\'' | '"', _) => {
                result.push(c);
                result.push_str(&quoted(&mut chars, c));
            }
            _ => result.push(c),
        }
    }
    result
}

fn quoted(chars: &mut Peekable<Chars<'_>>, quote: char) -> String {
    let mut value = String::new();
    while let Some(c) = chars.next() {
        value.push(c);
        if c == quote {
            break;
        }
        if c == '\\'
            && let Some(escaped) = chars.next()
        {
            value.push(escaped);
        }
    }
    value
}

fn block_comment(chars: &mut Peekable<Chars<'_>>) -> String {
    let mut newlines = String::new();
    while let Some(c) = chars.next() {
        if c == '*' && chars.peek() == Some(&'/') {
            chars.next();
            break;
        }
        if c == '\n' {
            newlines.push('\n');
        }
    }
    newlines
}
