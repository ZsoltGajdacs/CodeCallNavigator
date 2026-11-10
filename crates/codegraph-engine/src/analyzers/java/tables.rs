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
    Ok(BufReader::new(file).lines().filter_map(|line| match line {
        Ok(line) if line.is_empty() => None,
        Ok(line) => Some(Ok(line.split('\t').map(str::to_owned).collect())),
        Err(error) => Some(Err(error.into())),
    }))
}

pub(super) fn tsv_table(
    path: &Path,
    header: &[&str],
) -> Result<impl Iterator<Item = Result<Vec<String>>>> {
    let mut rows = tsv_rows(path)?;
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
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("rows.tsv");
        fs::write(&path, b"\xff\nvalid\n").unwrap();
        assert!(tsv_table(&path, &["header"]).is_err());
        fs::write(&path, b"header\n\xff\n").unwrap();
        assert!(
            tsv_table(&path, &["header"])
                .unwrap()
                .collect::<Result<Vec<_>>>()
                .is_err()
        );
        fs::write(&path, "wrong\nvalid\n").unwrap();
        assert!(tsv_table(&path, &["header"]).is_err());
    }
}
