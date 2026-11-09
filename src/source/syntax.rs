//! Small lexical source scanners; coloring never changes or interprets graph semantics.
use rune_kit::syntax::{self, Highlight, Paint, Syntax};
mod jsx;
mod strings;
use std::{ops::Range, path::Path};
use strings::{string_end, string_prefix, typescript_regex};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Language {
    Java,
    Csharp,
    TypeScript,
    Tsx,
}

pub(super) fn highlight(path: &Path, text: &str) -> Vec<Highlight> {
    let Some(language) = language(path) else {
        return Vec::new();
    };
    let mut scanner = Scanner {
        text,
        language,
        offset: 0,
        highlights: Vec::new(),
        jsx: jsx::Jsx::default(),
    };
    scanner.scan();
    scanner.highlights
}

fn language(path: &Path) -> Option<Language> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "java" => Some(Language::Java),
        "cs" => Some(Language::Csharp),
        "ts" | "mts" | "cts" => Some(Language::TypeScript),
        "tsx" => Some(Language::Tsx),
        _ => None,
    }
}

struct Scanner<'a> {
    text: &'a str,
    language: Language,
    offset: usize,
    highlights: Vec<Highlight>,
    jsx: jsx::Jsx,
}

impl Scanner<'_> {
    fn scan(&mut self) {
        let mut code_start = 0;
        while self.offset < self.text.len() {
            let start = self.offset;
            if let Some(skip) = self.jsx_skip() {
                self.code(code_start..start);
                if skip.tag {
                    self.tag(start..skip.end);
                }
                self.offset = skip.end;
                code_start = skip.end;
                continue;
            }
            if let Some(kind) = self.literal() {
                self.code(code_start..start);
                self.highlights
                    .push((start..self.offset, Paint::Kind(kind)));
                code_start = self.offset;
            } else {
                self.offset += self.text[self.offset..].chars().next().unwrap().len_utf8();
            }
        }
        self.code(code_start..self.text.len());
    }

    fn jsx_skip(&mut self) -> Option<jsx::Skip> {
        if self.language != Language::Tsx {
            return None;
        }
        self.jsx.step(self.text, self.offset)
    }

    fn tag(&mut self, range: Range<usize>) {
        // A tag has code/attribute strings, but no JSX prose. The normal TS
        // scanner preserves long/multiline attributes and Kit colors its code.
        let mut scanner = Scanner {
            text: &self.text[range.clone()],
            language: Language::TypeScript,
            offset: 0,
            highlights: Vec::new(),
            jsx: jsx::Jsx::default(),
        };
        scanner.scan();
        self.highlights.extend(
            scanner
                .highlights
                .into_iter()
                .map(|(r, p)| (r.start + range.start..r.end + range.start, p)),
        );
    }

    fn literal(&mut self) -> Option<Syntax> {
        let rest = &self.text[self.offset..];
        if rest.starts_with("//") {
            self.offset += rest.find('\n').unwrap_or(rest.len());
            return Some(Syntax::Comment);
        }
        if let Some(comment) = rest.strip_prefix("/*") {
            self.offset += comment.find("*/").map_or(rest.len(), |end| end + 4);
            return Some(Syntax::Comment);
        }
        if let Some(end) = typescript_regex(self.text, self.offset, self.language) {
            self.offset = end;
            return Some(Syntax::String);
        }
        let prefix = string_prefix(rest, self.language)?;
        self.offset = string_end(self.text, self.offset, prefix);
        Some(Syntax::String)
    }

    fn code(&mut self, range: Range<usize>) {
        if matches!(self.language, Language::TypeScript | Language::Tsx) {
            self.typescript(range);
        } else {
            self.highlights
                .extend(clike_code(self.text, range, self.language));
        }
    }

    // The Kit scanner caps each invocation at 32 KiB/8192 tokens. These chunks stay
    // below both limits. Multiline strings/comments were already consumed whole.
    fn typescript(&mut self, range: Range<usize>) {
        // JSX tags/prose have already been separated by the context tracker.
        // Use TS here so generic arrow declarations are not re-read as JSX.
        let language = "ts";
        let mut start = range.start;
        while start < range.end {
            let end = chunk_end(self.text, start, range.end);
            self.highlights.extend(
                syntax::builtin(Some(language), &self.text[start..end])
                    .into_iter()
                    .map(|(r, kind)| (r.start + start..r.end + start, kind)),
            );
            start = end;
        }
    }
}

fn clike_code(text: &str, range: Range<usize>, language: Language) -> Vec<Highlight> {
    let mut result = Vec::new();
    let mut offset = range.start;
    while offset < range.end {
        let start = offset;
        let ch = text[offset..].chars().next().unwrap();
        offset += ch.len_utf8();
        let kind = if word_start(ch) {
            offset = token_end(text, offset, range.end, word_character);
            word_kind(&text[start..offset], language)
        } else if ch.is_ascii_digit() {
            offset = token_end(text, offset, range.end, number_character);
            Some(Syntax::Number)
        } else if ch.is_ascii_punctuation() {
            Some(Syntax::Punctuation)
        } else {
            None
        };
        if let Some(kind) = kind {
            result.push((start..offset, Paint::Kind(kind)));
        }
    }
    result
}

fn word_start(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_'
}
fn word_character(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}
fn number_character(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.')
}

fn chunk_end(text: &str, start: usize, limit: usize) -> usize {
    let mut end = (start + 4096).min(limit);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    if end == limit {
        return end;
    }
    match text[start..end].rfind(char::is_whitespace) {
        Some(boundary) if boundary > 0 => start + boundary,
        _ => end,
    }
}

fn token_end(text: &str, start: usize, end: usize, accepts: impl Fn(char) -> bool) -> usize {
    start
        + text[start..end]
            .chars()
            .take_while(|&c| accepts(c))
            .map(char::len_utf8)
            .sum::<usize>()
}

fn word_kind(word: &str, language: Language) -> Option<Syntax> {
    const COMMON: &str = " abstract assert boolean break byte case catch char class const continue default do double else enum extends final finally float for if implements import instanceof int interface long native new package private protected public return short static strictfp super switch synchronized this throw throws transient try void volatile while true false null record sealed permits var yield ";
    const CSHARP: &str = " as async await base bool checked decimal delegate event explicit extern fixed foreach implicit in internal is lock namespace object operator out override params readonly ref required sbyte sizeof stackalloc string struct uint ulong unchecked unsafe ushort using virtual when where with get set init value partial global file not and or dynamic nint nuint managed unmanaged scoped ";
    let keyword = format!(" {word} ");
    let extra = if language == Language::Csharp {
        CSHARP
    } else {
        ""
    };
    if COMMON.contains(&keyword) || extra.contains(&keyword) {
        Some(Syntax::Keyword)
    } else if word.chars().next().is_some_and(char::is_uppercase) {
        Some(Syntax::Type)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn found(path: &str, text: &str, expected: &str, kind: Syntax) -> bool {
        highlight(Path::new(path), text)
            .iter()
            .any(|(range, paint)| &text[range.clone()] == expected && *paint == Paint::Kind(kind))
    }

    #[test]
    fn java_multiline_comments_text_blocks_and_unicode_keep_ranges() {
        let text = "class Café { /* two\nlines */ String s = \"\"\"\nhello \\\" world\n\"\"\"; int n = 42; }";
        assert!(found("Demo.java", text, "class", Syntax::Keyword));
        assert!(found("Demo.java", text, "Café", Syntax::Type));
        assert!(found(
            "Demo.java",
            text,
            "/* two\nlines */",
            Syntax::Comment
        ));
        assert!(found(
            "Demo.java",
            text,
            "\"\"\"\nhello \\\" world\n\"\"\"",
            Syntax::String
        ));
        assert!(found("Demo.java", text, "42", Syntax::Number));
    }

    #[test]
    fn csharp_verbatim_interpolated_and_raw_literals_are_whole() {
        let text = "namespace Demo; var a = @\"a \"\"quote\"\"\nnext\"; var b = $@\"x {a}\"; var c = $$\"\"\"\n{{a}}\n\"\"\";";
        assert!(found("Demo.cs", text, "namespace", Syntax::Keyword));
        assert!(found(
            "Demo.cs",
            text,
            "@\"a \"\"quote\"\"\nnext\"",
            Syntax::String
        ));
        assert!(found("Demo.cs", text, "$@\"x {a}\"", Syntax::String));
        assert!(found(
            "Demo.cs",
            text,
            "$$\"\"\"\n{{a}}\n\"\"\"",
            Syntax::String
        ));
    }

    #[test]
    fn typescript_is_colored_beyond_kit_single_block_limit() {
        let mut text = "// leading comment\n".repeat(3000);
        text.push_str(
            "export const answer = 42; /* across\nlines */ const s = `template\nvalue`;\n",
        );
        assert!(found("Demo.ts", &text, "export", Syntax::Keyword));
        assert!(found("Demo.ts", &text, "42", Syntax::Number));
        assert!(found(
            "Demo.ts",
            &text,
            "/* across\nlines */",
            Syntax::Comment
        ));
        assert!(found("Demo.ts", &text, "`template\nvalue`", Syntax::String));
    }

    #[test]
    fn typescript_regex_quotes_do_not_swallow_following_code() {
        let text = "const quoted = /[\"']/g; export const answer = 42;";
        assert!(found("Demo.ts", text, "/[\"']/g", Syntax::String));
        assert!(found("Demo.ts", text, "export", Syntax::Keyword));
        assert!(found("Demo.ts", text, "42", Syntax::Number));
    }

    #[test]
    fn tsx_text_apostrophes_preserve_following_code_even_in_large_files() {
        let component = "const C = () => <p>What's new?</p>; export const answer = 42;";
        assert!(found("Demo.tsx", component, "export", Syntax::Keyword));
        let text = format!("{}\n{}", "// preceding source\n".repeat(3000), component);
        assert!(found("Demo.tsx", &text, "export", Syntax::Keyword));
        assert!(found("Demo.tsx", &text, "42", Syntax::Number));
    }

    #[test]
    fn tsx_nested_text_and_expressions_keep_their_contexts() {
        let text = "const C = () => <div title=\"What's up?\">{flag ? <p>What's new?</p> : 'nothing'}</div>; export const answer = 42;";
        assert!(found("Demo.tsx", text, "'nothing'", Syntax::String));
        assert!(found("Demo.tsx", text, "export", Syntax::Keyword));
        assert!(found("Demo.tsx", text, "42", Syntax::Number));
        let generic = "const identity = <T,>(x: T) => x; export const answer = 42;";
        assert!(found("Demo.tsx", generic, "export", Syntax::Keyword));
    }

    #[test]
    fn tsx_constrained_and_defaulted_generic_arrows_are_code() {
        for declaration in [
            "const identity = <T extends Foo>(x: T) => x;",
            "const identity = <T = Foo>(x: T) => x;",
        ] {
            let text = format!("{declaration} export const answer = 42;");
            assert!(
                found("Demo.tsx", &text, "export", Syntax::Keyword),
                "{declaration}"
            );
            assert!(found("Demo.tsx", &text, "42", Syntax::Number));
        }
        let jsx = "const C = <T extends Foo>(What's new?)</T>; export const answer = 42;";
        assert!(found("Demo.tsx", jsx, "export", Syntax::Keyword));
    }

    #[test]
    fn java_textblock_escaped_triple_quotes_do_not_close_literal() {
        let literal = "\"\"\"\ncontains \\\"\"\" escaped delimiter\n\"\"\"";
        let text = format!("String x = {literal}; int answer = 42;");
        assert!(found("Demo.java", &text, literal, Syntax::String));
        assert!(found("Demo.java", &text, "42", Syntax::Number));
    }

    #[test]
    fn oversized_literals_are_colored_and_following_code_is_not_lost() {
        let text = format!("/*{}*/ class After {{}}", "x".repeat(50_000));
        assert!(found("Demo.java", &text, "class", Syntax::Keyword));
        let highlights = highlight(Path::new("Demo.java"), &text);
        assert_eq!(highlights[0].0, 0..50_004);
    }
}
