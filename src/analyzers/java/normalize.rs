//! Normalize typed engine output into project graph declarations and relationships.
use super::{
    JavaInput,
    build::JavaSourceSet,
    metadata::{ClassMetadata, MethodMetadata},
    tables::{tsv_rows, tsv_table},
};
use crate::{
    discovery::BuildTarget,
    model::{Edge, EntryPoints, GraphData, Node},
};
use anyhow::{Context, Result, ensure};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::{Path, PathBuf},
};
#[cfg(test)]
pub(super) fn normalize_bytecode(
    build: &BuildTarget,
    output: &Path,
    inputs: &[JavaInput],
    owner: Option<&JavaSourceSet>,
) -> Result<GraphData> {
    with_inventories(
        build,
        output,
        CompiledContext {
            inputs,
            owner,
            inventories: super::source_inventory::inventories(output, inputs)?,
        },
    )
}

pub(super) struct CompiledContext<'a> {
    pub inputs: &'a [JavaInput],
    pub owner: Option<&'a JavaSourceSet>,
    pub inventories: super::source_inventory::Inventories,
}

pub(super) fn with_inventories(
    build: &BuildTarget,
    output: &Path,
    context: CompiledContext<'_>,
) -> Result<GraphData> {
    let _stage = crate::metrics::stage("java.normalization");
    let status = super::engine::status(&output.join("analysis-status.properties"))?;
    ensure!(
        status.get("analysisComplete").map(String::as_str) == Some("true"),
        "Bytecode analyzer reported incomplete output"
    );
    ensure!(
        status.get("frameSnapshotLimitMethods").map(String::as_str) == Some("0"),
        "Bytecode analyzer exceeded its analysis limit"
    );
    let raw = PathBuf::from(
        status
            .get("outputDirectory")
            .context("Bytecode analyzer omitted its output directory")?,
    );
    let mut collector = BytecodeCollector::new(build, context.inputs, context.owner);
    collector.source_inventory = context.inventories;
    collector.classes(tsv_table(
        &output.join("class_metadata.tsv"),
        &[
            "input_index",
            "input_path",
            "class_entry",
            "class_name",
            "source_file",
            "flags",
            "annotations",
        ],
    )?)?;
    collector.methods(tsv_table(
        &output.join("method_metadata.tsv"),
        &[
            "input_index",
            "input_path",
            "class_entry",
            "class_name",
            "source_file",
            "method",
            "return_type",
            "flags",
            "start_line",
            "end_line",
            "annotations",
        ],
    )?)?;
    collector.validate_methods(tsv_rows(&raw.join("method_info.txt"))?)?;
    collector.edges(tsv_rows(&raw.join("method_call.txt"))?)?;
    collector.finish()
}

struct BytecodeCollector<'a> {
    build: &'a BuildTarget,
    inputs: &'a [JavaInput],
    graph: GraphData,
    classes: HashMap<String, ClassMetadata>,
    indices: HashMap<String, u32>,
    class_facts: HashMap<String, super::class_facts::ClassFacts>,
    prefixes: Vec<Option<String>>,
    declared: HashSet<String>,
    placeholder_count: usize,
    excluded_edges: usize,
    application_starts: Vec<String>,
    owner: Option<String>,
    source_inventory: super::source_inventory::Inventories,
    excluded: HashSet<String>,
}

impl<'a> BytecodeCollector<'a> {
    fn new(build: &'a BuildTarget, inputs: &'a [JavaInput], owner: Option<&JavaSourceSet>) -> Self {
        Self {
            build,
            inputs,
            graph: GraphData {
                roots: vec![build.path.to_string_lossy().into_owned()],
                ..GraphData::default()
            },
            classes: HashMap::new(),
            indices: HashMap::new(),
            class_facts: HashMap::new(),
            prefixes: inputs
                .iter()
                .map(|input| {
                    input
                        .source_set
                        .as_ref()
                        .map(|source| identity_prefix(build, source))
                })
                .collect(),
            declared: HashSet::new(),
            placeholder_count: 0,
            excluded_edges: 0,
            application_starts: Vec::new(),
            owner: owner.map(|source| identity_prefix(build, source)),
            source_inventory: HashMap::new(),
            excluded: HashSet::new(),
        }
    }

    fn classes(&mut self, rows: impl Iterator<Item = Result<Vec<String>>>) -> Result<()> {
        for row in rows {
            let row = row?;
            let class = ClassMetadata::parse(&row)?;
            if let Some(source) = self.source(&class)? {
                self.class_facts.insert(
                    class.name.clone(),
                    super::class_facts::ClassFacts::new(source, &class),
                );
            }
            ensure!(
                self.classes.insert(class.name.clone(), class).is_none(),
                "Duplicate selected class identity"
            );
        }
        Ok(())
    }

    fn methods(&mut self, rows: impl Iterator<Item = Result<Vec<String>>>) -> Result<()> {
        for row in rows {
            let row = row?;
            let method = MethodMetadata::parse(&row)?;
            if self.source(&method.class)?.is_none() {
                continue;
            }
            let key = method_key(&method.signature, &method.return_type);
            if self.owns_caller(&method.signature)? {
                ensure!(
                    self.declared.insert(key.clone()),
                    "Duplicate method identity in compiled project"
                );
            }
            let node = self.java_node(&method, false)?;
            let inventory = self
                .source_inventory
                .get(&method.class.input)
                .context("Java source declaration inventory is missing")?;
            if super::policy::excluded(&method, inventory) {
                self.excluded.insert(node.id.clone());
            }
            if method.is_application_start() {
                self.application_starts.push(node.id.clone());
            }
            self.insert_node(key, node)?;
        }
        Ok(())
    }

    fn validate_methods(&self, rows: impl Iterator<Item = Result<Vec<String>>>) -> Result<()> {
        let mut reported = HashSet::new();
        for row in rows {
            let row = row?;
            ensure!(row.len() >= 8, "Malformed method_info output");
            let name = row[0]
                .split_once(':')
                .context("Malformed method declaration")?
                .0;
            let Some(class) = self.classes.get(name) else {
                anyhow::bail!("Engine declared unknown class {name}");
            };
            if self.source(class)?.is_none() || !self.owns_caller(&row[0])? {
                continue;
            }
            let dimensions: usize = row[3].parse()?;
            ensure!(dimensions <= 255, "Invalid JVM array dimensions");
            reported.insert(method_key(
                &row[0],
                &format!("{}{}", row[2], "[]".repeat(dimensions)),
            ));
        }
        ensure!(
            (self.owner.is_some() || !reported.is_empty()) && reported == self.declared,
            "Analyzer method declarations differ from compiled input metadata"
        );
        Ok(())
    }

    fn edges(&mut self, rows: impl Iterator<Item = Result<Vec<String>>>) -> Result<()> {
        let mut aggregate = BTreeMap::<(u32, u32, String, String), u32>::new();
        for row in rows {
            self.collect_edge(&row?, &mut aggregate)?;
        }
        self.graph.edges = aggregate
            .into_iter()
            .map(|((from, to, kind, certainty), count)| Edge {
                provenance: String::new(),
                from,
                to,
                kind,
                certainty,
                count,
            })
            .collect();
        Ok(())
    }

    fn collect_edge(
        &mut self,
        row: &[String],
        aggregate: &mut BTreeMap<(u32, u32, String, String), u32>,
    ) -> Result<()> {
        ensure!(row.len() >= 12, "Malformed method_call output");
        if row[1] != "1" {
            return Ok(());
        }
        let (kind, target) = row[3]
            .strip_prefix('(')
            .and_then(|value| value.split_once(')'))
            .context("Malformed analyzer call type")?;
        if !self.owns_caller(&row[2])? {
            return Ok(());
        }
        let from = self.endpoint(&row[2], &row[5])?;
        let to = self.endpoint(target, &row[8])?;
        let (Some(from), Some(to)) = (from, to) else {
            self.excluded_edges += 1;
            return Ok(());
        };
        let key = (from, to, kind.to_owned(), java_certainty(kind).to_owned());
        let count = aggregate.entry(key).or_default();
        *count = count
            .checked_add(1)
            .context("Too many repeated call sites")?;
        Ok(())
    }

    fn endpoint(&mut self, signature: &str, return_type: &str) -> Result<Option<u32>> {
        let key = method_key(signature, return_type);
        if let Some(index) = self.indices.get(&key) {
            return Ok(Some(*index));
        }
        let class_name = signature
            .split_once(':')
            .map(|(class, _)| class)
            .context("Malformed bytecode method signature")?;
        let Some(class) = self.classes.get(class_name) else {
            return Ok(None);
        };
        if self.source(class)?.is_none() {
            return Ok(None);
        }
        let method = MethodMetadata::inherited(class.clone(), signature, return_type);
        let node = self.java_node(&method, true)?;
        self.excluded.insert(node.id.clone());
        self.placeholder_count += 1;
        Ok(Some(self.insert_node(key, node)?))
    }

    fn insert_node(&mut self, key: String, node: Node) -> Result<u32> {
        let index = u32::try_from(self.graph.nodes.len())?;
        ensure!(
            self.indices.insert(key, index).is_none(),
            "Duplicate normalized method identity"
        );
        self.graph.nodes.push(node);
        Ok(index)
    }

    fn source(&self, class: &ClassMetadata) -> Result<Option<&JavaSourceSet>> {
        Ok(self
            .inputs
            .get(class.input - 1)
            .context("Analyzer input provenance is missing")?
            .source_set
            .as_ref())
    }

    fn owns_caller(&self, signature: &str) -> Result<bool> {
        let name = signature
            .split_once(':')
            .context("Malformed caller signature")?
            .0;
        let Some(class) = self.classes.get(name) else {
            return Ok(false);
        };
        let prefix = self
            .prefixes
            .get(class.input - 1)
            .context("Analyzer input provenance is missing")?;
        Ok(prefix
            .as_ref()
            .is_some_and(|prefix| self.owner.as_ref().is_none_or(|owner| owner == prefix)))
    }

    fn java_node(&self, row: &MethodMetadata, unavailable: bool) -> Result<Node> {
        let source = self
            .source(&row.class)?
            .context("Project provenance is missing")?;
        let facts = self
            .class_facts
            .get(&row.class.name)
            .context("Class source facts are missing")?;
        let prefix = self
            .prefixes
            .get(row.class.input - 1)
            .and_then(Option::as_ref)
            .context("Class identity prefix is missing")?;
        let generated = row.annotations.contains("/Generated;") || facts.generated_file;
        let signature = format!("{} -> {}", row.signature, row.return_type);
        Ok(Node {
            id: format!("{}{}", prefix, method_key(&row.signature, &row.return_type)),
            project: source.project.clone(),
            build: self.build.path.to_string_lossy().into_owned(),
            language: "java".into(),
            kind: if unavailable {
                "inherited-reference"
            } else {
                "method"
            }
            .into(),
            label: self
                .source_inventory
                .get(&row.class.input)
                .map(|inventory| inventory.lambda_label(row))
                .unwrap_or_else(|| super::lambda_names::method_name(&row.signature).into()),
            signature,
            namespace: facts.namespace.clone(),
            container: row.class.name.clone(),
            file: facts.file.clone(),
            line: row.start_line,
            is_test: source.is_test,
            is_generated: generated,
            body_unavailable: unavailable,
        })
    }

    fn finish(mut self) -> Result<GraphData> {
        self.graph.entry_points = Some(EntryPoints {
            framework_entries: Vec::new(),
            candidates: Vec::new(),
            application_starts: self.application_starts,
        });
        crate::method_policy::prune(&mut self.graph, &self.excluded)?;
        if self.owner.is_none() {
            super::compose::finalize_entries(&mut self.graph);
        }
        self.graph.diagnostics.push(format!("Java: {} inherited/reference symbols without declared bodies excluded; {} external relationships excluded. These references are engine-reported, not resolved by name.", self.placeholder_count, self.excluded_edges));
        self.graph.diagnostics.push("Java DI, reflection, framework callback ordering and dynamic runtime targets are supported only where reported by java-callgraph2. Bytecode source columns are unavailable.".into());
        self.graph.validate()?;
        Ok(self.graph)
    }
}

pub(super) use super::metadata::method_key;
pub(super) fn java_certainty(kind: &str) -> &'static str {
    match kind {
        "STA" | "SPE" => "direct",
        "VIR" | "INT" => "dispatch",
        "_LM" => "reference",
        _ => "analyzer-reported",
    }
}

/// JSON tuple encoding prevents separators in project paths or names from colliding.
pub(super) fn identity_prefix(build: &BuildTarget, source: &JavaSourceSet) -> String {
    format!(
        "{}|",
        serde_json::json!([build.path, source.project_dir, source.source_set])
    )
}
