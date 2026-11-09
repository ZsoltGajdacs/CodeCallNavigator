//! Validated boundary for the bytecode adapter's escaped TSV metadata.
use anyhow::{Context, Result, ensure};
use std::{fs, path::Path};

#[derive(Clone)]
pub(super) struct ClassMetadata {
    pub input: usize,
    pub name: String,
    pub source_file: String,
    pub annotations: String,
}

pub(super) struct MethodMetadata {
    pub class: ClassMetadata,
    pub signature: String,
    pub return_type: String,
    pub flags: u32,
    pub start_line: u32,
    pub annotations: String,
}

impl ClassMetadata {
    pub fn parse(row: &[String]) -> Result<Self> {
        ensure!(row.len() == 7, "Malformed class metadata");
        let fields = row.iter().map(|value| unescape(value)).collect::<Vec<_>>();
        let input = fields[0]
            .parse::<usize>()
            .context("Invalid analyzer input index")?;
        ensure!(input > 0, "Invalid analyzer input index");
        ensure!(!fields[3].is_empty(), "Class name is missing");
        fields[5].parse::<u32>().context("Invalid class flags")?;
        Ok(Self {
            input,
            name: fields[3].clone(),
            source_file: fields[4].clone(),
            annotations: fields[6].clone(),
        })
    }
}

impl MethodMetadata {
    pub fn parse(row: &[String]) -> Result<Self> {
        ensure!(row.len() == 11, "Malformed method metadata");
        let fields = row.iter().map(|value| unescape(value)).collect::<Vec<_>>();
        let class = ClassMetadata::parse(&[
            row[0].clone(),
            row[1].clone(),
            row[2].clone(),
            row[3].clone(),
            row[4].clone(),
            row[7].clone(),
            row[10].clone(),
        ])?;
        ensure!(
            fields[5]
                .split_once(':')
                .is_some_and(|(name, _)| name == class.name),
            "Method class provenance mismatch"
        );
        fields[9]
            .parse::<u32>()
            .context("Invalid method end line")?;
        Ok(Self {
            class,
            signature: fields[5].clone(),
            return_type: fields[6].clone(),
            flags: fields[7].parse()?,
            start_line: fields[8].parse()?,
            annotations: fields[10].clone(),
        })
    }

    pub fn is_application_start(&self) -> bool {
        self.flags & 0x0009 == 0x0009
            && self.return_type == "void"
            && self
                .signature
                .split_once(':')
                .is_some_and(|(_, method)| method == "main(java.lang.String[])")
    }

    pub fn inherited(class: ClassMetadata, signature: &str, return_type: &str) -> Self {
        Self {
            annotations: class.annotations.clone(),
            class,
            signature: signature.into(),
            return_type: return_type.into(),
            flags: 0,
            start_line: 0,
        }
    }
}

pub(super) fn read_tsv(path: &Path) -> Result<Vec<Vec<String>>> {
    Ok(fs::read_to_string(path)
        .with_context(|| format!("Cannot read analyzer output {}", path.display()))?
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.split('\t').map(str::to_owned).collect())
        .collect())
}

pub(super) fn unescape(value: &str) -> String {
    let mut output = String::new();
    let mut chars = value.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        output.push(match chars.next() {
            Some('t') => '\t',
            Some('n') => '\n',
            Some('r') => '\r',
            Some(value) => value,
            None => '\\',
        });
    }
    output
}
