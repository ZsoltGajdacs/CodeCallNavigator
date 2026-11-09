//! Literal delimiters for Java, C# and TypeScript; independent of visible line windows.
use super::Language;

#[derive(Clone, Copy)]
pub(super) struct StringPrefix {
    bytes: usize,
    quotes: usize,
    quote: u8,
    verbatim: bool,
    escapes: bool,
}

pub(super) fn string_prefix(rest: &str, language: Language) -> Option<StringPrefix> {
    let bytes = rest.as_bytes();
    let prefix = if language == Language::Csharp {
        bytes
            .iter()
            .take_while(|&&c| matches!(c, b'@' | b'$'))
            .count()
    } else {
        0
    };
    let verbatim = bytes[..prefix].contains(&b'@');
    let quote = *bytes.get(prefix)?;
    if !is_quote(quote, language) {
        return None;
    }
    let run = bytes[prefix..]
        .iter()
        .take_while(|&&byte| byte == quote)
        .count();
    let quotes = if raw_delimiter(quote, run, language) {
        run
    } else {
        1
    };
    Some(StringPrefix {
        bytes: prefix + quotes,
        quotes,
        quote,
        verbatim,
        escapes: language == Language::Java || quotes == 1,
    })
}

fn is_quote(quote: u8, language: Language) -> bool {
    if matches!(quote, b'\'' | b'"') {
        return true;
    }
    quote == b'`' && matches!(language, Language::TypeScript | Language::Tsx)
}

fn raw_delimiter(quote: u8, run: usize, language: Language) -> bool {
    if quote != b'"' || run < 3 {
        return false;
    }
    matches!(language, Language::Java | Language::Csharp)
}

pub(super) fn string_end(text: &str, start: usize, prefix: StringPrefix) -> usize {
    let bytes = text.as_bytes();
    let mut offset = start + prefix.bytes;
    while offset < bytes.len() {
        let (advance, closes) = literal_step(bytes, offset, prefix);
        offset = (offset + advance).min(bytes.len());
        if closes {
            return offset;
        }
    }
    offset
}

fn literal_step(bytes: &[u8], offset: usize, prefix: StringPrefix) -> (usize, bool) {
    if bytes[offset] == prefix.quote {
        return quote_step(bytes, offset, prefix);
    }
    if escape_start(bytes[offset], prefix) {
        return (2, false);
    }
    (1, false)
}

fn escape_start(byte: u8, prefix: StringPrefix) -> bool {
    if prefix.verbatim || !prefix.escapes {
        return false;
    }
    byte == b'\\'
}

fn quote_step(bytes: &[u8], offset: usize, prefix: StringPrefix) -> (usize, bool) {
    let run = bytes[offset..]
        .iter()
        .take_while(|&&byte| byte == prefix.quote)
        .count();
    if prefix.quotes > 1 {
        return (prefix.quotes.min(run), run >= prefix.quotes);
    }
    if prefix.verbatim && run >= 2 {
        return (2, false);
    }
    (1, true)
}

/// A regex can contain quote marks and comment-looking text. Recognize expression
/// positions so its contents are not mistaken for standalone strings or comments.
pub(super) fn typescript_regex(text: &str, start: usize, language: Language) -> Option<usize> {
    if !matches!(language, Language::TypeScript | Language::Tsx) {
        return None;
    }
    if text.as_bytes()[start] != b'/' {
        return None;
    }
    if !regex_context(&text[..start]) {
        return None;
    }
    regex_end(text.as_bytes(), start + 1)
}

fn regex_context(before: &str) -> bool {
    let before = before.trim_end();
    let Some(last) = before.chars().last() else {
        return true;
    };
    if "=([{,:;!?&|>".contains(last) {
        return true;
    }
    let word = before
        .rsplit(|c: char| !c.is_alphabetic())
        .next()
        .unwrap_or("");
    matches!(word, "return" | "throw" | "case" | "yield" | "await")
}

fn regex_end(bytes: &[u8], mut offset: usize) -> Option<usize> {
    let mut class = false;
    while offset < bytes.len() {
        match bytes[offset] {
            b'\n' | b'\r' => return None,
            b'\\' => offset += 2,
            b'[' => {
                class = true;
                offset += 1;
            }
            b']' => {
                class = false;
                offset += 1;
            }
            b'/' if !class => return Some(regex_flags_end(bytes, offset + 1)),
            _ => offset += 1,
        }
    }
    None
}

fn regex_flags_end(bytes: &[u8], mut offset: usize) -> usize {
    while bytes.get(offset).is_some_and(u8::is_ascii_alphabetic) {
        offset += 1;
    }
    offset
}
