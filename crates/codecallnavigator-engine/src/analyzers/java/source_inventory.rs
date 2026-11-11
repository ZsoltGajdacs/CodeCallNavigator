//! Validated source declaration inventory, independent of graph inclusion policy.
#[cfg(test)]
use super::JavaInput;
use super::{
    lambda_correlation::{LambdaCorrelations, PendingCorrelations},
    lambda_names::SourceLambda,
    metadata::{method_key, unescape},
    tables::tsv_rows,
};
use anyhow::{Result, ensure};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::Arc,
};

pub(super) const VERSION: u32 = 8;

pub(super) type Inventories = HashMap<usize, Arc<SourceInventory>>;

#[derive(Default)]
pub(super) struct SourceInventory {
    declarations: HashSet<String>,
    lambdas: HashMap<String, Vec<SourceLambda>>,
    pub architecture: Vec<crate::architecture::FileFacts>,
    pub has_architecture: bool,
    pub has_flow: bool,
    pub has_framework: bool,
    pub has_http: bool,
    pub has_portable_lambdas: bool,
    pub has_lambda_correlations: bool,
    pub lambda_correlations: LambdaCorrelations,
    pub http_routes: Vec<super::http::SourceRoute>,
    pub flows: Vec<super::flow_inventory::SourceFlow>,
    pub framework: Vec<super::framework::SourcePair>,
    pub framework_diagnostics: Vec<String>,
    pub framework_listeners: Vec<String>,
}

impl SourceInventory {
    pub fn read_current(path: &Path) -> Result<Self> {
        let inventory = Self::read(path)?;
        ensure!(
            inventory.has_lambda_correlations,
            "Java source inventory lacks compiled lambda correlation capability kind_v8; rebuild the matching Java adapter"
        );
        Ok(inventory)
    }

    pub fn read(path: &Path) -> Result<Self> {
        let mut inventory = Self::default();
        let mut correlations = PendingCorrelations::default();
        let mut rows = tsv_rows(path)?;
        let mut header = rows.next().transpose()?;
        inventory.capabilities(&mut header);
        ensure!(
            header
                .as_ref()
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
        for row in rows {
            let row = row?;
            ensure!(row.len() == 9, "Malformed Java source inventory");
            let fields: Vec<_> = row.iter().map(|field| unescape(field)).collect();
            match fields[0].as_str() {
                "method" => {
                    inventory
                        .declarations
                        .insert(method_key(&fields[1], &fields[2]));
                }
                "flow" => {
                    inventory.flows.push(serde_json::from_str(&fields[3])?);
                }
                "lambda-binding" => correlations.binding(&fields[3])?,
                "lambda-correlation" => correlations.correlation(&fields[3])?,
                "http-route" => inventory
                    .http_routes
                    .push(serde_json::from_str(&fields[3])?),
                "framework" => inventory.framework.push(serde_json::from_str(&fields[3])?),
                "framework-listener" => inventory.framework_listeners.push(fields[3].clone()),
                "framework-diagnostic" => inventory.framework_diagnostics.push(fields[3].clone()),
                "architecture" => {
                    inventory.architecture.push(architecture_row(&fields)?);
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
        inventory.lambda_correlations =
            correlations.validate(&inventory.flows, inventory.has_lambda_correlations)?;
        Ok(inventory)
    }

    fn capabilities(&mut self, header: &mut Option<Vec<String>>) {
        self.has_lambda_correlations = header
            .as_ref()
            .is_some_and(|fields| fields.first().is_some_and(|field| field == "kind_v8"));
        self.has_portable_lambdas = header.as_ref().is_some_and(|fields| {
            fields
                .first()
                .is_some_and(|field| matches!(field.as_str(), "kind_v7" | "kind_v8"))
        });
        self.has_http = header.as_ref().is_some_and(|fields| {
            fields
                .first()
                .is_some_and(|field| matches!(field.as_str(), "kind_v6" | "kind_v7" | "kind_v8"))
        });
        self.has_framework = header.as_ref().is_some_and(|fields| {
            fields.first().is_some_and(|field| {
                matches!(
                    field.as_str(),
                    "kind_v5" | "kind_v6" | "kind_v7" | "kind_v8"
                )
            })
        });
        self.has_flow = header.as_ref().is_some_and(|fields| {
            fields.first().is_some_and(|field| {
                matches!(
                    field.as_str(),
                    "kind_v4" | "kind_v5" | "kind_v6" | "kind_v7" | "kind_v8"
                )
            })
        });
        self.has_architecture = header.as_ref().is_some_and(|fields| {
            fields.first().is_some_and(|field| {
                matches!(
                    field.as_str(),
                    "kind_v3" | "kind_v4" | "kind_v5" | "kind_v6" | "kind_v7" | "kind_v8"
                )
            })
        });
        if self.has_architecture {
            header.as_mut().unwrap()[0] = "kind_v2".into();
        }
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

#[cfg(test)]
pub(super) fn inventories(output: &Path, inputs: &[JavaInput]) -> Result<Inventories> {
    inputs
        .iter()
        .enumerate()
        .filter(|(_, input)| input.source_set.is_some())
        .map(|(index, _)| {
            Ok((
                index + 1,
                Arc::new(SourceInventory::read(
                    &output.join(format!("source-{}.tsv", index + 1)),
                )?),
            ))
        })
        .collect()
}

fn architecture_row(fields: &[String]) -> Result<crate::architecture::FileFacts> {
    let list = |index: usize| {
        fields[index]
            .split(',')
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    let file = &fields[1];
    ensure!(
        Path::new(file).is_absolute(),
        "Java architecture source path is not absolute"
    );
    let kinds = list(4);
    Ok(crate::architecture::FileFacts {
        file: file.clone(),
        language: "java".into(),
        namespaces: list(2),
        names: list(3),
        is_interface: kinds.iter().any(|kind| kind == "interface"),
        kinds,
        annotations: list(5),
        method_annotations: list(6),
        base_types: list(7),
        annotated_interfaces: list(8),
        content: std::fs::read_to_string(file)?,
        ..Default::default()
    })
}

#[cfg(test)]
mod capability_tests {
    use super::SourceInventory;
    #[test]
    fn legacy_inventory_is_readable_but_cannot_satisfy_current_runtime() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("source.tsv");
        let suffix = "\tsignature\treturn_type\tstart_line\tend_line\tstart_column\tend_column\tcontext\tassigned_name\n";
        for version in ["kind_v2", "kind_v3", "kind_v4", "kind_v5", "kind_v6"] {
            std::fs::write(&path, format!("{version}{suffix}")).unwrap();
            assert!(!SourceInventory::read(&path).unwrap().has_portable_lambdas);
            assert!(
                SourceInventory::read_current(&path)
                    .err()
                    .unwrap()
                    .to_string()
                    .contains("rebuild the matching Java adapter")
            );
        }
        std::fs::write(&path, format!("kind_v7{suffix}")).unwrap();
        assert!(SourceInventory::read(&path).unwrap().has_portable_lambdas);
        assert!(SourceInventory::read_current(&path).is_err());
        std::fs::write(&path, format!("kind_v8{suffix}")).unwrap();
        assert!(SourceInventory::read_current(&path).unwrap().has_http);
    }
}
