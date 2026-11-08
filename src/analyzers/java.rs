use super::process::run_process;
use super::{AnalysisJob, choose_artifact, ensure_unchanged, find_tool, source_snapshot};
use crate::{
    discovery::BuildTarget,
    fingerprint,
    model::{Edge, EntryPoints, FileFingerprint, GraphData, Node},
};
use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JavaSourceSet {
    project: String,
    is_test: bool,
    source_roots: Vec<PathBuf>,
    generated_roots: Vec<PathBuf>,
    class_dirs: Vec<PathBuf>,
    #[serde(default)]
    build_inputs: Vec<PathBuf>,
}
#[derive(Clone)]
struct JavaInput {
    path: PathBuf,
    source_set: JavaSourceSet,
}

pub(super) fn analyze_java(build: &BuildTarget, context: &AnalysisJob<'_>) -> Result<GraphData> {
    let AnalysisJob {
        runtime,
        path: job,
        cancel,
        progress,
    } = context;
    let java_home = runtime.java_home.as_ref().context(
        "Java analysis requires an installed JDK 25; configure JAVA_HOME or install JDK 25",
    )?;
    let before_metadata = source_snapshot(std::slice::from_ref(&build.path))?;
    let source_sets = gradle_metadata(build, context)?;
    ensure_unchanged(&before_metadata)?;
    let authored_roots = authored_java_roots(build, &source_sets);
    let before_compile = source_snapshot(&authored_roots)?;
    (context.progress)(format!(
        "Compiling all Java source sets for {} (no tests run)…",
        build.label
    ));
    gradle_task(build, context, "codegraphCompileSources")?;
    ensure_unchanged(&before_compile)?;
    let inputs = java_inputs(&source_sets)?;
    ensure!(
        !inputs.is_empty(),
        "Gradle emitted no Java class directories"
    );
    let bytecode_before = compiled_snapshot(&inputs)?;
    let source_roots = java_source_roots(build, &source_sets);
    let before_engine = source_snapshot(&source_roots)?;
    let output = job.join("bytecode");
    progress(format!("Reading compiled call graph for {}…", build.label));
    let mut command = Command::new(java_home.join("bin/java"));
    command
        .arg("-Xmx8g")
        .arg("-cp")
        .arg(runtime.bytecode_lib()?.join("*"))
        .arg("codegraph.bytecode.Analyzer")
        .arg(&output)
        .arg("25");
    for input in &inputs {
        command.arg(&input.path);
    }
    run_process(&mut command, job, "bytecode", cancel)?;
    ensure_unchanged(&before_compile)?;
    ensure_unchanged(&before_engine)?;
    ensure_unchanged(&bytecode_before)?;
    let mut graph = normalize_bytecode(build, &output, &inputs)?;
    graph.fingerprints = before_engine;
    Ok(graph)
}

fn java_source_roots(build: &BuildTarget, sets: &[JavaSourceSet]) -> Vec<PathBuf> {
    let mut roots = vec![build.path.clone()];
    roots.extend(sets.iter().flat_map(|source| source.source_roots.clone()));
    roots.extend(
        sets.iter()
            .flat_map(|source| source.generated_roots.clone()),
    );
    roots.extend(sets.iter().flat_map(|source| source.build_inputs.clone()));
    roots
}

fn authored_java_roots(build: &BuildTarget, sets: &[JavaSourceSet]) -> Vec<PathBuf> {
    let mut roots = vec![build.path.clone()];
    for source in sets {
        roots.extend(
            source
                .source_roots
                .iter()
                .filter(|path| {
                    !source
                        .generated_roots
                        .iter()
                        .any(|generated| path.starts_with(generated))
                })
                .cloned(),
        );
        roots.extend(source.build_inputs.clone());
    }
    roots
}

fn compiled_snapshot(inputs: &[JavaInput]) -> Result<Vec<FileFingerprint>> {
    let mut paths = Vec::new();
    for input in inputs {
        compiled_paths(&input.path, &mut paths)?;
    }
    fingerprint::capture(paths)
}

fn compiled_paths(root: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
    paths.push(root.into());
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            compiled_paths(&entry.path(), paths)?;
        } else if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "class")
        {
            paths.push(entry.path());
        }
    }
    Ok(())
}

fn gradle_metadata(build: &BuildTarget, context: &AnalysisJob<'_>) -> Result<Vec<JavaSourceSet>> {
    (context.progress)(format!(
        "Reading Gradle source-set ownership for {}…",
        build.label
    ));
    gradle_task(build, context, "codegraphExportMetadata")?;
    Ok(serde_json::from_slice(&fs::read(
        context.path.join("gradle-metadata.json"),
    )?)?)
}

fn gradle_task(build: &BuildTarget, context: &AnalysisJob<'_>, task: &str) -> Result<()> {
    let runtime = context.runtime;
    let java_home = runtime
        .java_home
        .as_ref()
        .context("Java analysis requires an installed JDK 25")?;
    let metadata = context.path.join("gradle-metadata.json");
    let script = choose_artifact(
        &runtime.bundle_root,
        &[
            "tools/gradle-metadata.gradle",
            "analyzers/gradle-metadata.gradle",
        ],
    )?;
    run_process(
        Command::new(gradle_command(&build.path)?)
            .current_dir(&build.path)
            .env("JAVA_HOME", java_home)
            .arg("--no-daemon")
            .arg("--console=plain")
            .arg("-I")
            .arg(script)
            .arg(format!("-PcodegraphMetadataFile={}", metadata.display()))
            .arg(task),
        context.path,
        task,
        context.cancel,
    )?;
    Ok(())
}

fn gradle_command(root: &Path) -> Result<PathBuf> {
    let wrapper = root.join("gradlew");
    if wrapper.is_file() {
        return Ok(wrapper);
    }
    if let Some(path) = find_tool("gradle") {
        return Ok(path);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("No home folder for Gradle discovery")?;
    let versions = home.join(".gradle/wrapper/dists/gradle-9.4.0-bin");
    for entry in fs::read_dir(versions).context("No Gradle wrapper or installed Gradle found")? {
        let path = entry?.path().join("gradle-9.4.0/bin/gradle");
        if path.is_file() {
            return Ok(path);
        }
    }
    bail!("No Gradle wrapper or installed Gradle found")
}

fn java_inputs(source_sets: &[JavaSourceSet]) -> Result<Vec<JavaInput>> {
    let mut inputs = Vec::new();
    let mut seen = HashSet::new();
    for source in source_sets {
        for path in &source.class_dirs {
            if seen.insert(path.clone()) && contains_class(path)? {
                inputs.push(JavaInput {
                    path: path.clone(),
                    source_set: source.clone(),
                });
            }
        }
    }
    Ok(inputs)
}

fn contains_class(path: &Path) -> Result<bool> {
    if !path.is_dir() {
        return Ok(false);
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() && contains_class(&entry.path())? {
            return Ok(true);
        }
        if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "class")
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn read_tsv(path: &Path) -> Result<Vec<Vec<String>>> {
    Ok(fs::read_to_string(path)
        .with_context(|| format!("Cannot read analyzer output {}", path.display()))?
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.split('\t').map(str::to_owned).collect())
        .collect())
}

fn tsv_unescape(value: &str) -> String {
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

fn properties(path: &Path) -> Result<HashMap<String, String>> {
    Ok(fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.to_owned(), tsv_unescape(value)))
        .collect())
}

fn normalize_bytecode(
    build: &BuildTarget,
    output: &Path,
    inputs: &[JavaInput],
) -> Result<GraphData> {
    let status = properties(&output.join("analysis-status.properties"))?;
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
    let mut collector = BytecodeCollector::new(build, inputs);
    collector.classes(&read_tsv(&output.join("class_metadata.tsv"))?)?;
    collector.methods(&read_tsv(&output.join("method_metadata.tsv"))?)?;
    collector.validate_methods(&read_tsv(&raw.join("method_info.txt"))?)?;
    collector.edges(&read_tsv(&raw.join("method_call.txt"))?)?;
    collector.finish()
}

struct BytecodeCollector<'a> {
    build: &'a BuildTarget,
    inputs: &'a [JavaInput],
    graph: GraphData,
    classes: HashMap<String, Vec<String>>,
    indices: HashMap<String, u32>,
    declared: HashSet<String>,
    placeholder_count: usize,
    excluded_edges: usize,
    application_starts: Vec<String>,
}

impl<'a> BytecodeCollector<'a> {
    fn new(build: &'a BuildTarget, inputs: &'a [JavaInput]) -> Self {
        Self {
            build,
            inputs,
            graph: GraphData {
                roots: vec![build.path.to_string_lossy().into_owned()],
                ..GraphData::default()
            },
            classes: HashMap::new(),
            indices: HashMap::new(),
            declared: HashSet::new(),
            placeholder_count: 0,
            excluded_edges: 0,
            application_starts: Vec::new(),
        }
    }

    fn classes(&mut self, rows: &[Vec<String>]) -> Result<()> {
        for row in rows.iter().skip(1) {
            ensure!(row.len() == 7, "Malformed class metadata");
            let values = row
                .iter()
                .map(|value| tsv_unescape(value))
                .collect::<Vec<_>>();
            ensure!(
                self.classes.insert(values[3].clone(), values).is_none(),
                "Duplicate project class identity across class directories"
            );
        }
        Ok(())
    }

    fn methods(&mut self, rows: &[Vec<String>]) -> Result<()> {
        for row in rows.iter().skip(1) {
            ensure!(row.len() == 11, "Malformed method metadata");
            let values = row
                .iter()
                .map(|value| tsv_unescape(value))
                .collect::<Vec<_>>();
            let key = method_key(&values[5], &values[6]);
            ensure!(
                self.declared.insert(key.clone()),
                "Duplicate method identity in compiled project"
            );
            let node = self.java_node(&values, false)?;
            if is_application_start(&values)? {
                self.application_starts.push(node.id.clone());
            }
            self.insert_node(key, node)?;
        }
        Ok(())
    }

    fn validate_methods(&self, rows: &[Vec<String>]) -> Result<()> {
        let mut reported = HashSet::new();
        for row in rows {
            ensure!(row.len() >= 8, "Malformed method_info output");
            let dimensions: usize = row[3].parse()?;
            ensure!(dimensions <= 255, "Invalid JVM array dimensions");
            reported.insert(method_key(
                &row[0],
                &format!("{}{}", row[2], "[]".repeat(dimensions)),
            ));
        }
        ensure!(
            !reported.is_empty() && reported == self.declared,
            "Analyzer method declarations differ from compiled input metadata"
        );
        Ok(())
    }

    fn edges(&mut self, rows: &[Vec<String>]) -> Result<()> {
        let mut aggregate = BTreeMap::<(u32, u32, String, String), u32>::new();
        for row in rows {
            self.collect_edge(row, &mut aggregate)?;
        }
        self.graph.edges = aggregate
            .into_iter()
            .map(|((from, to, kind, certainty), count)| Edge {
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
        let values = vec![
            class[0].clone(),
            class[1].clone(),
            class[2].clone(),
            class_name.into(),
            class[4].clone(),
            signature.into(),
            return_type.into(),
            "0".into(),
            "0".into(),
            "0".into(),
            class[6].clone(),
        ];
        let node = self.java_node(&values, true)?;
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

    fn java_node(&self, row: &[String], unavailable: bool) -> Result<Node> {
        let input_index: usize = row[0].parse()?;
        let input = self
            .inputs
            .get(
                input_index
                    .checked_sub(1)
                    .context("Invalid analyzer input index")?,
            )
            .context("Analyzer input provenance is missing")?;
        let source = &input.source_set;
        let namespace = row[3]
            .rsplit_once('.')
            .map(|(namespace, _)| namespace)
            .unwrap_or("");
        let file = source_location(source, namespace, &row[4]);
        let generated = row[10].contains("/Generated;")
            || source
                .generated_roots
                .iter()
                .any(|root| Path::new(&file).starts_with(root));
        let signature = format!("{} -> {}", row[5], row[6]);
        Ok(Node {
            id: format!(
                "{}|{}",
                self.build.path.display(),
                method_key(&row[5], &row[6])
            ),
            project: if source.project == ":" {
                self.build.label.clone()
            } else {
                format!("{}{}", self.build.label, source.project)
            },
            build: self.build.path.to_string_lossy().into_owned(),
            language: "java".into(),
            kind: if unavailable {
                "inherited-reference"
            } else {
                "method"
            }
            .into(),
            label: row[5]
                .split_once(':')
                .map(|(_, method)| method.split('(').next().unwrap_or(method))
                .unwrap_or(&row[5])
                .into(),
            signature,
            namespace: namespace.into(),
            container: row[3].clone(),
            file,
            line: row[8].parse()?,
            is_test: source.is_test,
            is_generated: generated,
            body_unavailable: unavailable,
        })
    }

    fn finish(mut self) -> Result<GraphData> {
        let called: HashSet<_> = self.graph.edges.iter().map(|edge| edge.to).collect();
        let mut candidates: Vec<_> = self
            .graph
            .nodes
            .iter()
            .enumerate()
            .filter(|(index, node)| {
                !node.body_unavailable
                    && !node.signature.contains(":<clinit>()")
                    && !called.contains(&(*index as u32))
            })
            .map(|(_, node)| node.id.clone())
            .collect();
        candidates.sort();
        self.application_starts.sort();
        self.graph.entry_points = Some(EntryPoints {
            candidates,
            application_starts: self.application_starts,
        });
        self.graph.diagnostics.push(format!("Java: {} inherited/reference symbols have no declared body; {} external relationships excluded. These references are engine-reported, not resolved by name.", self.placeholder_count, self.excluded_edges));
        self.graph.diagnostics.push("Java DI, reflection, framework callback ordering and dynamic runtime targets are supported only where reported by java-callgraph2. Bytecode source columns are unavailable.".into());
        self.graph.validate()?;
        Ok(self.graph)
    }
}

/// BCEL access flags and descriptor, rather than a method-name heuristic.
fn is_application_start(row: &[String]) -> Result<bool> {
    let flags: u32 = row[7].parse().context("Invalid method access flags")?;
    Ok(flags & 0x0009 == 0x0009
        && row[6] == "void"
        && row[5]
            .split_once(':')
            .is_some_and(|(_, method)| method == "main(java.lang.String[])"))
}

fn method_key(signature: &str, return_type: &str) -> String {
    format!("{signature}|{return_type}")
}
fn java_certainty(kind: &str) -> &'static str {
    match kind {
        "STA" | "SPE" => "direct",
        "VIR" | "INT" => "dispatch",
        "_LM" => "reference",
        _ => "analyzer-reported",
    }
}

fn source_location(source: &JavaSourceSet, namespace: &str, file: &str) -> String {
    if file.is_empty() || file == "<Unknown>" {
        return String::new();
    }
    let relative = PathBuf::from(namespace.replace('.', "/")).join(file);
    source
        .source_roots
        .iter()
        .chain(&source.generated_roots)
        .map(|root| root.join(&relative))
        .find(|path| path.is_file())
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::AnalyzerRuntime;
    use crate::graph::{Filter, GraphIndex};
    use std::sync::{Arc, atomic::AtomicBool};
    #[test]
    fn return_types_distinguish_bridge_methods_and_references_stay_uncertain() {
        assert_ne!(
            method_key("Example:get()", "Object"),
            method_key("Example:get()", "String")
        );
        assert_eq!(java_certainty("_LM"), "reference");
        assert_eq!(java_certainty("_ACT_I"), "analyzer-reported");
        assert_eq!(tsv_unescape("a\\tb\\\\c"), "a\tb\\c");
    }

    #[test]
    #[ignore = "requires the pinned bytecode distribution and installed JDK 25"]
    fn authored_lambda_flow_survives_the_default_production_filter() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = AnalyzerRuntime::discover().unwrap();
        let input = compile_callback_fixture(&runtime, directory.path());
        let output = run_callback_engine(&runtime, directory.path(), &input.path);
        let build = BuildTarget {
            path: directory.path().into(),
            language: "java".into(),
            kind: "gradle".into(),
            label: "callback".into(),
        };
        let graph = normalize_bytecode(&build, &output, &[input]).unwrap();
        let start = graph
            .nodes
            .iter()
            .position(|node| node.signature.starts_with("fixture.Flow:start()"))
            .unwrap() as u32;
        let leaf = graph
            .nodes
            .iter()
            .position(|node| node.signature.starts_with("fixture.Flow:leaf()"))
            .unwrap() as u32;
        let callback = graph
            .nodes
            .iter()
            .find(|node| node.signature.contains("lambda$"))
            .unwrap();
        assert!(
            !callback.is_generated,
            "Authored lambdas are synthetic bytecode, not generated source"
        );
        let index = GraphIndex::new(Arc::new(graph)).unwrap();
        assert_eq!(
            index
                .shortest_path(start, leaf, &Filter::default())
                .unwrap()
                .len(),
            3
        );
    }

    #[test]
    #[ignore = "requires the pinned bytecode distribution and installed JDK 25"]
    fn real_bytecode_distinguishes_application_start_from_uncalled_and_invalid_main() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = AnalyzerRuntime::discover().unwrap();
        let input = compile_callback_fixture(&runtime, directory.path());
        let output = run_callback_engine(&runtime, directory.path(), &input.path);
        let build = BuildTarget {
            path: directory.path().into(),
            language: "java".into(),
            kind: "gradle".into(),
            label: "entries".into(),
        };
        let graph = normalize_bytecode(&build, &output, &[input]).unwrap();
        let entries = graph.entry_points.unwrap();
        assert_eq!(entries.application_starts.len(), 1);
        assert!(
            !entries
                .candidates
                .iter()
                .any(|id| id.contains(":<clinit>()"))
        );
        assert!(entries.candidates.iter().any(|id| id.contains(":<init>()")));
        assert!(entries.application_starts[0].contains(":main(java.lang.String[])|void"));
        assert!(
            entries
                .candidates
                .iter()
                .any(|id| id.contains(":uncalled()|void"))
        );
        assert!(
            !entries
                .application_starts
                .iter()
                .any(|id| id.contains("Invalid"))
        );
    }

    fn compile_callback_fixture(runtime: &AnalyzerRuntime, root: &Path) -> JavaInput {
        let source_root = root.join("src");
        let source_file = source_root.join("fixture/Flow.java");
        fs::create_dir_all(source_file.parent().unwrap()).unwrap();
        fs::write(&source_file, "package fixture; public final class Flow { public static void main(String[] args) { new Flow().start(); } public void start() { Runnable callback = () -> leaf(); callback.run(); } private void leaf() {} public void uncalled() {} } class StaticInit { static int value = initialize(); static int initialize() { return 1; } } class InvalidPrivate { private static void main(String[] args) {} } class InvalidInstance { public void main(String[] args) {} } class InvalidReturn { public static int main(String[] args) { return 0; } } class InvalidArgs { public static void main(String arg) {} }").unwrap();
        let classes = root.join("classes");
        run_process(
            Command::new(runtime.java_home.as_ref().unwrap().join("bin/javac"))
                .arg("-g")
                .arg("-d")
                .arg(&classes)
                .arg(source_file),
            root,
            "javac",
            &AtomicBool::new(false),
        )
        .unwrap();
        JavaInput {
            path: classes.clone(),
            source_set: JavaSourceSet {
                project: ":".into(),
                is_test: false,
                source_roots: vec![source_root],
                generated_roots: Vec::new(),
                class_dirs: vec![classes],
                build_inputs: Vec::new(),
            },
        }
    }

    fn run_callback_engine(runtime: &AnalyzerRuntime, root: &Path, classes: &Path) -> PathBuf {
        let output = root.join("analysis");
        run_process(
            Command::new(runtime.java_home.as_ref().unwrap().join("bin/java"))
                .arg("-cp")
                .arg(runtime.bytecode_lib().unwrap().join("*"))
                .arg("codegraph.bytecode.Analyzer")
                .arg(&output)
                .arg("25")
                .arg(classes),
            root,
            "engine",
            &AtomicBool::new(false),
        )
        .unwrap();
        output
    }
}
