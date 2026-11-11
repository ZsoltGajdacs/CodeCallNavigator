//! Validated boundary for the bytecode adapter's escaped TSV metadata.
use crate::model::Visibility;
use anyhow::{Context, Result, ensure};

#[derive(Clone)]
pub(super) struct ClassMetadata {
    pub input: usize,
    pub name: String,
    pub source_file: String,
    pub annotations: String,
    pub visibility: Option<Visibility>,
}

pub(super) struct MethodMetadata {
    pub class: ClassMetadata,
    pub signature: String,
    pub return_type: String,
    pub flags: u32,
    pub start_line: u32,
    pub end_line: u32,
    pub annotations: String,
}

impl ClassMetadata {
    pub fn parse(row: &[String]) -> Result<Self> {
        ensure!(row.len() == 8, "Malformed class metadata");
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
            visibility: match fields[7].as_str() {
                "public" => Some(Visibility::Public),
                "private" => Some(Visibility::Private),
                "protected" => Some(Visibility::Protected),
                "package" => Some(Visibility::Package),
                "" => None,
                other => anyhow::bail!("Invalid Java class visibility: {other}"),
            },
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
            String::new(),
        ])?;
        ensure!(
            fields[5]
                .split_once(':')
                .is_some_and(|(name, _)| name == class.name),
            "Method class provenance mismatch"
        );
        let end_line = fields[9]
            .parse::<u32>()
            .context("Invalid method end line")?;
        Ok(Self {
            class,
            signature: fields[5].clone(),
            return_type: fields[6].clone(),
            flags: fields[7].parse()?,
            start_line: fields[8].parse()?,
            end_line,
            annotations: fields[10].clone(),
        })
    }

    pub fn visibility(&self) -> Option<Visibility> {
        let name = super::lambda_names::method_name(&self.signature);
        if self.flags & 0x1000 != 0 && name.starts_with("lambda$") {
            return None;
        }
        Some(match self.flags & 0x0007 {
            0x0001 => Visibility::Public,
            0x0002 => Visibility::Private,
            0x0004 => Visibility::Protected,
            _ => Visibility::Package,
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
            end_line: 0,
        }
    }
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

pub(super) fn method_key(signature: &str, return_type: &str) -> String {
    format!("{signature}|{return_type}")
}
