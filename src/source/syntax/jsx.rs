//! Tracks JSX text separately from JavaScript expressions so prose quotes are plain text.
use super::{
    Language,
    strings::{string_end, string_prefix},
};

#[derive(Default)]
pub(super) struct Jsx {
    contexts: Vec<Context>,
}

enum Context {
    Text,
    Expression(usize),
}

pub(super) struct Skip {
    pub end: usize,
    pub tag: bool,
}

impl Jsx {
    pub fn step(&mut self, text: &str, start: usize) -> Option<Skip> {
        let cursor = Cursor {
            text,
            offset: start,
        };
        let rest = cursor.rest();
        let in_text = matches!(self.contexts.last(), Some(Context::Text));
        let tag_start = rest.starts_with('<');
        let jsx_allowed = in_text || expression_start(&text[..start]);
        let candidate = (tag_start && jsx_allowed).then(|| tag(cursor)).flatten();
        if let Some(tag) = candidate {
            self.apply_tag(&tag);
            return Some(Skip {
                end: tag.end,
                tag: true,
            });
        }
        if in_text {
            return self.text_step(cursor);
        }
        self.expression_step(rest.as_bytes()[0]);
        None
    }

    fn apply_tag(&mut self, tag: &Tag) {
        if tag.closing {
            self.contexts.pop();
        } else if !tag.self_closing {
            self.contexts.push(Context::Text);
        }
    }

    fn text_step(&mut self, cursor: Cursor<'_>) -> Option<Skip> {
        let Cursor {
            text,
            offset: start,
        } = cursor;
        if cursor.rest().starts_with('{') {
            self.contexts.push(Context::Expression(1));
            return None;
        }
        let first = text[start..].chars().next().unwrap().len_utf8();
        let end = text[start + first..]
            .find(['<', '{'])
            .map_or(text.len(), |next| start + first + next);
        Some(Skip { end, tag: false })
    }

    fn expression_step(&mut self, byte: u8) {
        let Some(Context::Expression(braces)) = self.contexts.last_mut() else {
            return;
        };
        if byte == b'{' {
            *braces += 1;
        }
        if byte == b'}' {
            *braces -= 1;
            if *braces == 0 {
                self.contexts.pop();
            }
        }
    }
}

struct Tag {
    end: usize,
    closing: bool,
    self_closing: bool,
}

#[derive(Clone, Copy)]
struct Cursor<'a> {
    text: &'a str,
    offset: usize,
}
impl<'a> Cursor<'a> {
    fn rest(self) -> &'a str {
        &self.text[self.offset..]
    }
}

fn tag(cursor: Cursor<'_>) -> Option<Tag> {
    let rest = &cursor.rest()[1..];
    let closing = rest.starts_with('/');
    let name = rest.strip_prefix('/').unwrap_or(rest);
    if !tag_name(name) {
        return None;
    }
    let end = delimited_end(
        Cursor {
            offset: cursor.offset + 1,
            ..cursor
        },
        Delimiters::TAG,
    )?;
    if generic_arrow(cursor, end) {
        return None;
    }
    Some(Tag {
        end,
        closing,
        self_closing: cursor.text[..end - 1].ends_with('/'),
    })
}

fn tag_name(name: &str) -> bool {
    if name.starts_with('>') {
        return true;
    }
    if !name.chars().next().is_some_and(char::is_alphabetic) {
        return false;
    }
    let after_name = name.trim_start_matches(|c: char| c.is_alphanumeric() || "_.:-".contains(c));
    after_name.starts_with(['>', '/']) || after_name.starts_with(char::is_whitespace)
}

enum TagStep {
    Advance(usize),
    Close(usize),
}

fn delimited_end(mut cursor: Cursor<'_>, delimiters: Delimiters) -> Option<usize> {
    let mut braces = 0usize;
    while cursor.offset < cursor.text.len() {
        match tag_step(cursor, &mut braces, delimiters) {
            TagStep::Advance(next) => cursor.offset = next,
            TagStep::Close(end) => return Some(end),
        }
    }
    None
}

fn tag_step(cursor: Cursor<'_>, braces: &mut usize, delimiters: Delimiters) -> TagStep {
    let rest = cursor.rest();
    if let Some(prefix) = string_prefix(rest, Language::TypeScript) {
        return TagStep::Advance(string_end(cursor.text, cursor.offset, prefix));
    }
    let byte = rest.as_bytes()[0];
    if byte == delimiters.end && *braces == 0 {
        return TagStep::Close(cursor.offset + 1);
    }
    delimiters.advance(byte, braces);
    TagStep::Advance(cursor.offset + rest.chars().next().unwrap().len_utf8())
}

#[derive(Clone, Copy)]
struct Delimiters {
    open: u8,
    close: u8,
    end: u8,
}
impl Delimiters {
    const TAG: Self = Self {
        open: b'{',
        close: b'}',
        end: b'>',
    };
    const PARAMETERS: Self = Self {
        open: b'(',
        close: b')',
        end: b')',
    };
    fn advance(self, byte: u8, depth: &mut usize) {
        if byte == self.open {
            *depth += 1;
        }
        if byte == self.close {
            *depth = depth.saturating_sub(1);
        }
    }
}

// In TSX a constrained/defaulted generic arrow resembles an opening element.
// Its complete parameter list followed by => identifies code, not JSX prose.
fn generic_arrow(cursor: Cursor<'_>, head_end: usize) -> bool {
    let tail = cursor.text[head_end..].trim_start();
    if !tail.starts_with('(') {
        return false;
    }
    let parameters = Cursor {
        text: cursor.text,
        offset: cursor.text.len() - tail.len() + 1,
    };
    delimited_end(parameters, Delimiters::PARAMETERS)
        .is_some_and(|end| cursor.text[end..].trim_start().starts_with("=>"))
}

fn expression_start(before: &str) -> bool {
    let before = before.trim_end();
    let Some(last) = before.chars().last() else {
        return true;
    };
    if "=>([,?:;{!&|".contains(last) {
        return true;
    }
    let word = before
        .rsplit(|c: char| !c.is_alphabetic())
        .next()
        .unwrap_or("");
    matches!(word, "return" | "yield")
}
