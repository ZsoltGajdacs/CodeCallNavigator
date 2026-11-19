//! Streaming validated transport for analyzer tables.
use anyhow::{Context, Result, ensure};
use std::{
    fs,
    io::{BufRead, BufReader},
    path::Path,
};

pub(super) fn tsv_rows(path: &Path) -> Result<impl Iterator<Item = Result<Vec<String>>>> {
    let file = fs::File::open(path)
        .with_context(|| format!("Cannot read analyzer output {}", path.display()))?;
    Ok(rows(BufReader::new(file)))
}

fn rows(reader: impl BufRead) -> impl Iterator<Item = Result<Vec<String>>> {
    reader.lines().filter_map(|line| match line {
        Ok(line) if line.is_empty() => None,
        Ok(line) => Some(Ok(line.split('\t').map(str::to_owned).collect())),
        Err(error) => Some(Err(error.into())),
    })
}

pub(super) fn tsv_table(
    path: &Path,
    header: &[&str],
) -> Result<impl Iterator<Item = Result<Vec<String>>>> {
    table(tsv_rows(path)?, path, header)
}

fn table(
    mut rows: impl Iterator<Item = Result<Vec<String>>>,
    path: &Path,
    header: &[&str],
) -> Result<impl Iterator<Item = Result<Vec<String>>>> {
    let actual = rows
        .next()
        .transpose()?
        .context("Analyzer table header is missing")?;
    ensure!(
        actual.iter().map(String::as_str).eq(header.iter().copied()),
        "Unsupported analyzer table header: {}",
        path.display()
    );
    Ok(rows)
}

pub(super) fn read_tsv(path: &Path) -> Result<Vec<Vec<String>>> {
    tsv_rows(path)?.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_utf8_in_header_or_body_is_rejected() {
        let path = Path::new("rows.tsv");
        let parse = |bytes: &'static [u8]| table(rows(bytes), path, &["header"]);
        assert!(parse(b"\xff\nvalid\n").is_err());
        assert!(
            parse(b"header\n\xff\n")
                .unwrap()
                .collect::<Result<Vec<_>>>()
                .is_err()
        );
        assert!(parse(b"wrong\nvalid\n").is_err());
    }
}
