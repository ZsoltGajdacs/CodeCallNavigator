//! Present Maven reactor failures from the captured build streams.
use crate::analyzers::process::{LogTails, NativeExitFailure};
use anyhow::Error;
use std::collections::HashSet;

const LOG_READ_LIMIT: u64 = 64 * 1024;
const ERROR_PREFIX: &str = "[ERROR]";
const FAILED_GOAL: &str = "Failed to execute goal ";

pub(super) fn enrich_exit(error: Error) -> Error {
    match error.downcast::<NativeExitFailure>() {
        Ok(failure) if failure.exited() => {
            let summary = summarize(failure.log_tails(LOG_READ_LIMIT));
            failure.with_log_summary(summary).into()
        }
        Ok(failure) => failure.into(),
        Err(error) => error,
    }
}

fn summarize(logs: LogTails) -> String {
    let stdout = without_terminal_formatting(&logs.stdout);
    let stderr = without_terminal_formatting(&logs.stderr);
    let errors = stdout
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix(ERROR_PREFIX))
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let goals = excerpt(
        errors.clone().filter(|line| line.starts_with(FAILED_GOAL)),
        1600,
    );
    if goals.is_empty() {
        return render_sections([
            ("Maven stdout", tail(&stdout, 2000)),
            ("Maven stderr", tail(&stderr, 1000)),
        ]);
    }

    let details = errors.filter(|line| {
        !line.starts_with(FAILED_GOAL)
            && *line != "BUILD FAILURE"
            && !line.starts_with("-> [Help")
            && !line.starts_with("[Help ")
    });
    render_sections([
        ("Maven failures", goals),
        ("Build errors", excerpt(details, 1450)),
        ("Maven stderr", tail(&stderr, 750)),
    ])
}

fn render_sections<const N: usize>(sections: [(&str, String); N]) -> String {
    sections
        .into_iter()
        .filter(|(_, text)| !text.is_empty())
        .map(|(label, text)| format!("\n{label}:\n{text}"))
        .collect()
}

fn excerpt<'a>(lines: impl Iterator<Item = &'a str>, limit: usize) -> String {
    let mut seen = HashSet::new();
    let lines = lines
        .filter(|line| seen.insert(*line))
        .map(|line| cap(line, 400))
        .collect::<Vec<_>>();
    cap(&lines.join("\n"), limit)
}

fn cap(text: &str, limit: usize) -> String {
    text.char_indices()
        .nth(limit)
        .map_or_else(|| text.to_owned(), |(end, _)| format!("{}…", &text[..end]))
}

fn tail(text: &str, limit: usize) -> String {
    let text = text.trim();
    text.char_indices().rev().nth(limit).map_or_else(
        || text.to_owned(),
        |(start, character)| format!("…{}", &text[start + character.len_utf8()..]),
    )
}

fn without_terminal_formatting(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(character) = chars.next() {
        match character {
            '\u{1b}' => skip_escape(&mut chars),
            '\n' | '\r' | '\t' => result.push(character),
            printable if !printable.is_control() => result.push(printable),
            _ => {}
        }
    }
    result
}

fn skip_escape(chars: &mut std::str::Chars<'_>) {
    match chars.next() {
        Some('[') => {
            chars.find(|item| ('@'..='~').contains(item));
        }
        Some(']') => {
            // OSC strings end at BEL or the two-character string terminator.
            while let Some(item) = chars.find(|item| matches!(item, '\u{7}' | '\u{1b}')) {
                if item == '\u{7}' || chars.next() == Some('\\') {
                    break;
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{LogTails, summarize};

    #[test]
    fn prioritizes_reactor_errors_over_stderr_warnings() {
        let compiler = "[ERROR] LabelFilterTest.java:[19,10] cannot find symbol\n[ERROR] symbol: method getSlug()\n";
        let stdout = format!(
            "{}\u{1b}[31m[ERROR]\u{1b}[m Failed to execute goal compiler:testCompile on project recurring-dao: Compilation failure\n[ERROR] Failed to execute goal antrun:run on project recurring-ui: exec returned 1",
            compiler.repeat(250)
        );
        let summary = summarize(LogTails {
            stdout,
            stderr: "WARNING: sun.misc.Unsafe\n".into(),
        });
        let dao = summary.find("recurring-dao").unwrap();
        let ui = summary.find("recurring-ui").unwrap();
        let error = summary.find("getSlug()").unwrap();
        let warning = summary.find("WARNING").unwrap();
        assert!(dao < ui && ui < error && error < warning, "{summary}");
        assert_eq!(summary.matches("getSlug()").count(), 1);
        assert!(!summary.contains('\u{1b}'));
        assert!(summary.chars().count() < 4096);
    }

    #[test]
    fn falls_back_to_bounded_unicode_tails() {
        let stdout = format!("{}end", "界".repeat(4500));
        let summary = summarize(LogTails {
            stdout,
            stderr: "\u{1b}]8;;https://example.invalid\u{7}stderr".into(),
        });
        assert!(summary.contains("Maven stdout:\n…界"));
        assert!(summary.contains("end\nMaven stderr:\nstderr"));
        assert!(summary.chars().count() < 4096);
        assert!(
            summarize(LogTails {
                stdout: " \n".into(),
                stderr: String::new(),
            })
            .is_empty()
        );
    }
}
