//! Current on-disk source, independent of graph generations and persistence.
mod syntax;

use rune_kit::syntax::Highlight;
use std::{fs::OpenOptions, io::Read, ops::Range, path::Path};

pub const MAX_SOURCE_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug)]
pub struct SourceLine {
    pub range: Range<usize>,
    pub highlights: Vec<Highlight>,
}

#[derive(Debug)]
pub struct SourceFile {
    pub text: String,
    pub lines: Vec<SourceLine>,
    pub max_columns: usize,
}

pub fn load(path: &str) -> Result<SourceFile, String> {
    if path.is_empty() {
        return Err("No source file was recorded for this method.".into());
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let file = options
        .open(path)
        .map_err(|error| format!("Cannot open source file: {error}"))?;
    if !file
        .metadata()
        .map_err(|error| error.to_string())?
        .is_file()
    {
        return Err("Source path is not a regular file.".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_SOURCE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Cannot read source file: {error}"))?;
    if bytes.len() as u64 > MAX_SOURCE_BYTES {
        return Err("Source file exceeds the 16 MiB preview limit.".into());
    }
    let text = String::from_utf8(bytes)
        .map_err(|_| "Source file is not valid UTF-8.".to_string())?
        .replace("\r\n", "\n");
    Ok(from_text(text, Path::new(path)))
}

/// Highlight an immutable generation snapshot without normalizing its byte offsets.
pub fn from_snapshot(text: String, path: &Path) -> SourceFile {
    from_text(text, path)
}

fn from_text(text: String, path: &Path) -> SourceFile {
    let mut offset = 0;
    let mut max_columns = 0;
    let mut lines: Vec<_> = text
        .split('\n')
        .map(|line| {
            let range = offset..offset + line.len();
            offset += line.len() + 1;
            max_columns = max_columns.max(line.chars().map(column_width).sum());
            SourceLine {
                range,
                highlights: Vec::new(),
            }
        })
        .collect();
    distribute(&mut lines, syntax::highlight(path, &text));
    SourceFile {
        text,
        lines,
        max_columns,
    }
}

// A conservative display width keeps CJK/emoji rows horizontally reachable.
fn column_width(ch: char) -> usize {
    if ch == '\t' {
        4
    } else if ch.is_ascii() {
        1
    } else {
        2
    }
}

fn distribute(lines: &mut [SourceLine], highlights: Vec<Highlight>) {
    let mut line = 0;
    for (range, paint) in highlights {
        line += lines[line..].partition_point(|current| current.range.end < range.start);
        for current in &mut lines[line..] {
            if current.range.start >= range.end {
                break;
            }
            let start = current.range.start.max(range.start);
            let end = current.range.end.min(range.end);
            if start < end {
                current.highlights.push((
                    start - current.range.start..end - current.range.start,
                    paint,
                ));
            }
        }
    }
}

/// Returns a zero-based position, with an explanation when the graph has no usable location.
pub fn target_line(requested: u32, count: usize) -> (usize, Option<String>) {
    if requested == 0 || requested as usize > count {
        (
            0,
            Some(
                "The recorded method line is unavailable in this file; showing its beginning."
                    .into(),
            ),
        )
    } else {
        (requested as usize - 1, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::{self, File},
        io::{Seek, SeekFrom, Write},
    };

    #[test]
    fn reads_real_unicode_crlf_source_and_preserves_final_empty_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Demo.java");
        fs::write(&path, "class Café {\r\n\tString x = \"猫\";\r\n}\r\n").unwrap();
        let source = load(path.to_str().unwrap()).unwrap();
        assert_eq!(source.lines.len(), 4);
        assert_eq!(
            &source.text[source.lines[1].range.clone()],
            "\tString x = \"猫\";"
        );
        assert_eq!(&source.text[source.lines[3].range.clone()], "");
        assert!(!source.text.contains('\r'));
        assert!(source.lines[1].highlights.len() >= 3);
        assert!(source.max_columns >= 18);
    }

    #[test]
    fn source_errors_are_explicit_and_bounded() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load("").unwrap_err().contains("No source"));
        assert!(
            load(dir.path().join("missing.java").to_str().unwrap())
                .unwrap_err()
                .contains("Cannot open")
        );
        assert!(
            load(dir.path().to_str().unwrap())
                .unwrap_err()
                .contains("regular file")
        );
        let path = dir.path().join("invalid.cs");
        fs::write(&path, [255]).unwrap();
        assert!(load(path.to_str().unwrap()).unwrap_err().contains("UTF-8"));
        let mut file = File::create(&path).unwrap();
        file.seek(SeekFrom::Start(MAX_SOURCE_BYTES)).unwrap();
        file.write_all(&[0]).unwrap();
        assert!(load(path.to_str().unwrap()).unwrap_err().contains("16 MiB"));
    }

    #[test]
    fn target_lines_do_not_claim_invalid_locations() {
        assert_eq!(target_line(2, 4), (1, None));
        assert!(target_line(0, 4).1.is_some());
        assert!(target_line(5, 4).1.is_some());
        assert_eq!(target_line(5, 4).0, 0);
    }

    #[test]
    fn empty_source_is_one_empty_line() {
        let source = from_text(String::new(), Path::new("Empty.ts"));
        assert_eq!(source.lines.len(), 1);
        assert_eq!(source.lines[0].range, 0..0);
    }
}
