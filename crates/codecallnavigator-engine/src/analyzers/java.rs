#[cfg(test)]
use super::process::run_process;
use super::{AnalysisJob, ensure_unchanged};
use crate::{discovery::BuildTarget, model::GraphData};
use anyhow::Result;
#[cfg(test)]
use std::process::Command;
use std::{
    fs,
    path::{Path, PathBuf},
};

mod build;
mod class_facts;
mod compose;
mod contexts;
mod engine;
mod flow_inventory;
mod framework;
mod gradle;
mod http;
mod lambda_binding;
mod lambda_correlation;
#[cfg(test)]
mod lambda_correlation_tests;
mod lambda_names;
#[cfg(test)]
mod lambda_tests;
mod maven;
mod metadata;
mod normalize;
mod policy;
mod source_inventory;
mod tables;
#[cfg(test)]
mod visibility_tests;
use build::JavaSourceSet;
pub(super) use build::{PreparedJava, prepare, reject_overlaps};
#[cfg(test)]
use metadata::unescape as tsv_unescape;
#[cfg(test)]
use normalize::normalize_bytecode;
#[cfg(test)]
use normalize::{java_certainty, method_key};

#[derive(Clone)]
struct JavaInput {
    path: PathBuf,
    source_set: Option<JavaSourceSet>,
}

pub(super) fn analyze_java(
    build: &BuildTarget,
    context: &AnalysisJob<'_>,
    prepared: PreparedJava,
) -> Result<GraphData> {
    ensure_unchanged(&prepared.authored_before)?;
    (context.progress)(format!(
        "Compiling all Java source sets for {} (no tests run)…",
        build.label
    ));
    let metadata = build::compile(build, context)?;
    prepared.verify_compiled(&metadata)?;
    let graph = analyze_compiled(build, context, &metadata)?;
    ensure_unchanged(&prepared.authored_before)?;
    Ok(graph)
}

fn analyze_compiled(
    build: &BuildTarget,
    context: &AnalysisJob<'_>,
    metadata: &build::JavaBuildMetadata,
) -> Result<GraphData> {
    let contexts = contexts::plan(&metadata.source_sets)?;
    let bytecode_before = contexts::snapshot(&contexts)?;
    let before_engine = build::snapshot(build, metadata, true)?;
    let mut graph = compiled_graph(build, metadata);
    let mut composer = compose::Composer::default();
    let mut inventories = engine::SourceInventories::default();
    for (index, planned) in contexts.iter().enumerate() {
        (context.progress)(format!(
            "Reading Java context {} / {} ({}/{})…",
            planned.owner.project,
            planned.owner.source_set,
            index + 1,
            contexts.len()
        ));
        let root = context.path.join(format!("context-{index}"));
        let staged = contexts::stage(planned, &metadata.source_sets, &root, context)?;
        let output = engine::run(&staged.inputs, &planned.owner, &root, context)?;
        let sources = inventories.load(&staged.inputs, &output, context)?;
        let part = normalize::with_inventories(
            build,
            &output,
            normalize::CompiledContext {
                inputs: &staged.inputs,
                owner: Some(&planned.owner),
                inventories: sources,
            },
        )?;
        composer.append(&mut graph, part)?;
        graph.diagnostics.extend(staged.diagnostics);
        // Successful contexts carry no persisted temporary paths. Bound disk use to
        // one context; retain a failing context's files for diagnosis.
        let _ = fs::remove_dir_all(&root);
    }
    ensure_unchanged(&before_engine)?;
    ensure_unchanged(&bytecode_before)?;
    compose::finish(&mut graph)?;
    graph.fingerprints = before_engine;
    graph.fingerprints.extend(bytecode_before);
    let visibility = contexts
        .iter()
        .map(|planned| {
            serde_json::json!({
                "project": planned.owner.project_dir, "sourceSet": planned.owner.source_set,
                "classpath": planned.classpath,
            })
        })
        .collect::<Vec<_>>();
    let order_hash = blake3::hash(&serde_json::to_vec(&visibility)?)
        .to_hex()
        .to_string();
    graph.engine_versions = serde_json::json!({"java_context_protocol": 1, "java_source_inventory": source_inventory::VERSION, "compile_contexts": visibility, "classpath_order_hash": order_hash}).to_string();
    Ok(graph)
}

fn compiled_graph(build: &BuildTarget, metadata: &build::JavaBuildMetadata) -> GraphData {
    GraphData {
        roots: vec![build.path.to_string_lossy().into_owned()],
        identity_roots: metadata
            .modules
            .iter()
            .map(|root| root.to_string_lossy().into_owned())
            .collect(),
        folder_project_builds: if build.kind == "gradle" {
            vec![build.path.to_string_lossy().into_owned()]
        } else {
            Vec::new()
        },
        architecture: super::architecture::empty(),
        ..GraphData::default()
    }
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

fn contains_class(root: &Path) -> Result<bool> {
    contains_project_class(root, root)
}

fn contains_project_class(root: &Path, path: &Path) -> Result<bool> {
    if !path.is_dir() || path.strip_prefix(root)?.starts_with("META-INF/versions") {
        return Ok(false);
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() && contains_project_class(root, &path)? {
            return Ok(true);
        }
        if path
            .extension()
            .is_some_and(|extension| extension == "class")
            && entry.file_name() != "module-info.class"
        {
            return Ok(true);
        }
    }
    Ok(false)
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
        let output = run_callback_engine(&runtime, directory.path(), &input);
        let build = BuildTarget {
            path: directory.path().into(),
            language: "java".into(),
            kind: "gradle".into(),
            label: "callback".into(),
        };
        let graph = normalize_bytecode(&build, &output, &[input], None).unwrap();
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
        let output = run_callback_engine(&runtime, directory.path(), &input);
        let build = BuildTarget {
            path: directory.path().into(),
            language: "java".into(),
            kind: "gradle".into(),
            label: "entries".into(),
        };
        let graph = normalize_bytecode(&build, &output, &[input], None).unwrap();
        let entries = graph.entry_points.unwrap();
        assert_eq!(entries.application_starts.len(), 1);
        assert!(
            !entries
                .candidates
                .iter()
                .any(|id| id.contains(":<clinit>()"))
        );
        assert!(!entries.candidates.iter().any(|id| id.contains(":<init>()")));
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

    #[test]
    #[ignore = "requires the pinned bytecode distribution and installed JDK 25"]
    fn source_methods_exclude_initializers_object_overrides_and_compiler_helpers() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = AnalyzerRuntime::discover().unwrap();
        let input = compile_callback_fixture(&runtime, directory.path());
        let output = run_callback_engine(&runtime, directory.path(), &input);
        let build = BuildTarget {
            path: directory.path().into(),
            language: "java".into(),
            kind: "gradle".into(),
            label: "source policy".into(),
        };
        let graph = normalize_bytecode(&build, &output, &[input], None).unwrap();
        let has = |fragment: &str| {
            graph
                .nodes
                .iter()
                .any(|node| node.signature.contains(fragment))
        };
        for name in [
            "visible",
            "hidden",
            "packageMethod",
            "protectedMethod",
            "constructorOnly",
            "initializerOnly",
        ] {
            assert!(has(&format!("Methods:{name}()")), "Missing {name}");
        }
        for fragment in [
            ":<init>(",
            ":<clinit>(",
            ":toString()",
            ":hashCode()",
            ":equals(java.lang.Object)",
            "Value:x()",
            "Choice:values()",
            "Choice:valueOf(",
        ] {
            assert!(!has(fragment), "Unexpected {fragment}");
        }
        for fragment in [
            "Methods:toString(int)",
            "Methods:equals(java.lang.String)",
            "Value:actual()",
            "Choice:actual()",
        ] {
            assert!(has(fragment), "Missing {fragment}");
        }
        assert_eq!(
            graph
                .nodes
                .iter()
                .filter(|node| node.signature.contains("StringBox:get()"))
                .count(),
            1
        );
        assert!(
            graph
                .nodes
                .iter()
                .all(|node| node.kind != "inherited-reference")
        );
        graph.validate().unwrap();
    }

    #[test]
    #[ignore = "requires the pinned bytecode distribution and installed JDK 25"]
    fn no_debug_lambda_and_constructor_only_empty_graph_are_supported() {
        let runtime = AnalyzerRuntime::discover().unwrap();
        for (source, expected) in [
            (
                "package fixture; public class Flow { void run() { Runnable callback = () -> leaf(); callback.run(); } void leaf() {} }",
                3,
            ),
            ("package fixture; public class Flow {}", 0),
            ("package fixture; class A{}", 2),
            ("package fixture; class Gone { void stale() {} }", 0),
            (
                "package fixture; class DifferentlyNamed { void actual() {} }",
                1,
            ),
        ] {
            let directory = tempfile::tempdir().unwrap();
            if source.contains("class A{}") {
                let extra = directory.path().join("src/fixture/A$B.java");
                fs::create_dir_all(extra.parent().unwrap()).unwrap();
                fs::write(extra, "package fixture; class A$B { void actual() {} static class C { void nested() {} } }").unwrap();
            }
            let input = compile_source_fixture(&runtime, directory.path(), source, "-g:none");
            if source.contains("class Gone") {
                fs::remove_file(directory.path().join("src/fixture/Flow.java")).unwrap();
            }
            fs::write(
                directory.path().join("src/fixture/Excluded.java"),
                "class Excluded { invalid",
            )
            .unwrap();
            let output = run_callback_engine(&runtime, directory.path(), &input);
            let build = BuildTarget {
                path: directory.path().into(),
                language: "java".into(),
                kind: "gradle".into(),
                label: "no debug".into(),
            };
            let graph = normalize_bytecode(&build, &output, &[input], None).unwrap();
            assert_eq!(graph.nodes.len(), expected);
            graph.validate().unwrap();
        }
    }

    fn compile_callback_fixture(runtime: &AnalyzerRuntime, root: &Path) -> JavaInput {
        compile_source_fixture(
            runtime,
            root,
            "package fixture; public final class Flow { public static void main(String[] args) { new Flow().start(); } public void start() { Runnable callback = () -> leaf(); callback.run(); } private void leaf() {} public void uncalled() {} } class StaticInit { static int value = initialize(); static int initialize() { return 1; } } class InvalidPrivate { private static void main(String[] args) {} } class InvalidInstance { public void main(String[] args) {} } class InvalidReturn { public static int main(String[] args) { return 0; } } class InvalidArgs { public static void main(String arg) {} } class Methods { public Methods() { constructorOnly(); } static { initializerOnly(); } private void constructorOnly() {} private static void initializerOnly() {} public void visible() {} private void hidden() {} void packageMethod() {} protected void protectedMethod() {} public String toString() { return \"x\"; } public String toString(int x) { return \"x\"; } public int hashCode() { return 1; } public boolean equals(Object x) { return false; } public boolean equals(String x) { return false; } } record Value(int x) { public int actual() { return x; } } enum Choice { ONE; void actual() {} } interface Box<T> { T get(); } class StringBox implements Box<String> { public String get() { return \"value\"; } }",
            "-g",
        )
    }

    pub(super) fn compile_source_fixture(
        runtime: &AnalyzerRuntime,
        root: &Path,
        source: &str,
        debug: &str,
    ) -> JavaInput {
        let source_root = root.join("src");
        let source_file = source_root.join("fixture/Flow.java");
        fs::create_dir_all(source_file.parent().unwrap()).unwrap();
        fs::write(&source_file, source).unwrap();
        fs::write(
            source_root.join("fixture/package-info.java"),
            "@Deprecated package fixture;",
        )
        .unwrap();
        let source_files: Vec<_> = fs::read_dir(source_file.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        let classes = root.join("classes");
        run_process(
            Command::new(runtime.java_home.as_ref().unwrap().join("bin/javac"))
                .arg(debug)
                .arg("-d")
                .arg(&classes)
                .args(source_files),
            root,
            "javac",
            &AtomicBool::new(false),
        )
        .unwrap();
        JavaInput {
            path: classes.clone(),
            source_set: Some(JavaSourceSet {
                application: None,
                project: "fixture".into(),
                source_set: "main".into(),
                compile_classpath: Vec::new(),
                classpath_aliases: Vec::new(),
                project_dir: root.into(),
                is_test: false,
                source_roots: vec![source_root],
                generated_roots: Vec::new(),
                class_dirs: vec![classes],
                build_inputs: Vec::new(),
            }),
        }
    }

    pub(super) fn run_callback_engine(
        runtime: &AnalyzerRuntime,
        root: &Path,
        input: &JavaInput,
    ) -> PathBuf {
        let output = root.join("analysis");
        run_process(
            Command::new(runtime.java_home.as_ref().unwrap().join("bin/java"))
                .arg("-cp")
                .arg(runtime.bytecode_lib().unwrap().join("*"))
                .arg("codecallnavigator.bytecode.Analyzer")
                .arg(&output)
                .arg("25")
                .arg(&input.path),
            root,
            "engine",
            &AtomicBool::new(false),
        )
        .unwrap();
        let cancel = AtomicBool::new(false);
        let progress = |_| {};
        let job = AnalysisJob {
            runtime,
            path: root,
            shared_path: root,
            cancel: &cancel,
            progress: &progress,
        };
        engine::source_inventories(std::slice::from_ref(input), &output, &job).unwrap();
        output
    }
}

#[cfg(test)]
mod compiled_acceptance {
    use super::*;
    use crate::analyzers::AnalyzerRuntime;
    use std::sync::atomic::AtomicBool;

    #[test]
    #[ignore = "requires successful real-build metadata via CODECALLNAVIGATOR_COMPILED_METADATA"]
    fn recurring_billing_successful_build_outputs_preserve_both_generated_copies() {
        let manifest = std::env::var_os("CODECALLNAVIGATOR_COMPILED_METADATA")
            .expect("Set CODECALLNAVIGATOR_COMPILED_METADATA");
        let metadata: build::JavaBuildMetadata =
            serde_json::from_slice(&fs::read(manifest).unwrap()).unwrap();
        let build = BuildTarget {
            path: PathBuf::from(std::env::var_os("CODECALLNAVIGATOR_RECURRING_BILLING").unwrap())
                .canonicalize()
                .unwrap(),
            language: "java".into(),
            kind: "maven".into(),
            label: "recurring-billing compiled acceptance".into(),
        };
        let root = tempfile::Builder::new()
            .prefix("codecallnavigator compiled acceptance ")
            .tempdir()
            .unwrap();
        let runtime = AnalyzerRuntime::discover().unwrap();
        let cancel = AtomicBool::new(false);
        let progress = |message| eprintln!("{message}");
        let context = AnalysisJob {
            runtime: &runtime,
            path: root.path(),
            shared_path: root.path(),
            cancel: &cancel,
            progress: &progress,
        };
        let graph = analyze_compiled(&build, &context, &metadata).unwrap();
        assert!(graph.nodes.iter().all(|node| !node.file.is_empty() && Path::new(&node.file).starts_with(&build.path)), "Every declaration must belong to an actual selected-project source file");
        for module in ["recurring-billing-impl", "recurring-billing-test-utils"] {
            let classes = graph
                .nodes
                .iter()
                .filter(|node| {
                    node.project.ends_with(module)
                        && node.is_generated
                        && (node.namespace == "se.fortnox.messages.v1"
                            || node.namespace == "se.fortnox.notices.v1")
                })
                .map(|node| &node.container)
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(
                classes.len(),
                10,
                "Missing generated declarations for {module}"
            );
        }
        if let Some(path) = std::env::var_os("CODECALLNAVIGATOR_GRAPH_EVIDENCE") {
            fs::write(path, serde_json::to_vec(&graph).unwrap()).unwrap();
        }
        eprintln!(
            "Recurring-billing: {} declarations, {} relationships",
            graph.nodes.len(),
            graph.edges.len()
        );
    }
}
