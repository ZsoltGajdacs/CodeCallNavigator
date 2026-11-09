//! Validated source declaration inventory, independent of graph inclusion policy.
use super::{
    JavaInput,
    lambda_names::SourceLambda,
    metadata::{method_key, read_tsv, unescape},
};
use anyhow::{Result, ensure};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

#[derive(Default)]
pub(super) struct SourceInventory {
    declarations: HashSet<String>,
    lambdas: HashMap<String, Vec<SourceLambda>>,
}

impl SourceInventory {
    pub fn read(path: &Path) -> Result<Self> {
        let mut inventory = Self::default();
        let rows = read_tsv(path)?;
        ensure!(
            rows.first()
                .is_some_and(|row| row.iter().map(String::as_str).eq([
                    "kind_v2",
                    "signature",
                    "return_type",
                    "start_line",
                    "end_line",
                    "start_column",
                    "end_column",
                    "context",
                    "assigned_name"
                ])),
            "Unsupported Java source inventory format; rebuild the Java adapter"
        );
        for row in rows.iter().skip(1) {
            ensure!(row.len() == 9, "Malformed Java source inventory");
            let fields: Vec<_> = row.iter().map(|field| unescape(field)).collect();
            match fields[0].as_str() {
                "method" => {
                    inventory
                        .declarations
                        .insert(method_key(&fields[1], &fields[2]));
                }
                "lambda" => {
                    inventory
                        .lambdas
                        .entry(fields[1].clone())
                        .or_default()
                        .push(SourceLambda::parse(&fields)?);
                }
                _ => anyhow::bail!("Unknown Java source inventory kind"),
            }
        }
        Ok(inventory)
    }

    pub fn declares(&self, signature: &str, return_type: &str) -> bool {
        self.declarations
            .contains(&method_key(signature, return_type))
    }

    pub fn lambda_label(&self, method: &super::metadata::MethodMetadata) -> String {
        let name = super::lambda_names::method_name(&method.signature);
        if method.flags & 0x1000 == 0 || !name.starts_with("lambda$") {
            return name.into();
        }
        self.lambdas
            .get(&method.class.name)
            .and_then(|sources| {
                super::lambda_names::resolve(sources, method.start_line, method.end_line)
            })
            .unwrap_or_else(|| super::lambda_names::fallback(name))
    }

    pub fn has_lambda(&self, class: &str, line: u32) -> bool {
        self.lambdas.get(class).is_some_and(|ranges| {
            ranges
                .iter()
                .any(|source| line == 0 || source.contains(line, line))
        })
    }
}

pub(super) fn inventories(
    output: &Path,
    inputs: &[JavaInput],
) -> Result<HashMap<usize, SourceInventory>> {
    inputs
        .iter()
        .enumerate()
        .filter(|(_, input)| input.source_set.is_some())
        .map(|(index, _)| {
            Ok((
                index + 1,
                SourceInventory::read(&output.join(format!("source-{}.tsv", index + 1)))?,
            ))
        })
        .collect()
}
