//! Validated source declaration inventory, independent of graph inclusion policy.
use super::{
    JavaInput,
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
    lambdas: HashMap<String, Vec<(u32, u32)>>,
}

impl SourceInventory {
    pub fn read(path: &Path) -> Result<Self> {
        let mut inventory = Self::default();
        for row in read_tsv(path)?.iter().skip(1) {
            ensure!(row.len() == 5, "Malformed Java source inventory");
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
                        .push((fields[3].parse()?, fields[4].parse()?));
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

    pub fn has_lambda(&self, class: &str, line: u32) -> bool {
        self.lambdas.get(class).is_some_and(|ranges| {
            ranges
                .iter()
                .any(|&(start, end)| line == 0 || (start..=end).contains(&line))
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
