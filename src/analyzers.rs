//! Orchestrates existing analyzers; language adapters never resolve calls themselves.
mod csharp;
mod java;
mod process;
use crate::{
    discovery::BuildTarget,
    fingerprint,
    model::{FileFingerprint, GraphData},
};
use anyhow::{Context, Result, bail, ensure};
use process::check_cancel;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug)]
pub struct AnalyzerRuntime {
    pub bundle_root: PathBuf,
    pub java_home: Option<PathBuf>,
    pub dot: PathBuf,
}

impl AnalyzerRuntime {
    pub fn discover() -> Result<Self> {
        let executable = std::env::current_exe()?;
        let resources = executable
            .parent()
            .and_then(Path::parent)
            .unwrap_or(Path::new("."))
            .join("Resources");
        let bundle_root = if resources.join("analyzers").is_dir() {
            resources
        } else {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        };
        let java_home = discover_jdk25();
        let bundled_dot = bundle_root.join("analyzers/graphviz/bin/dot");
        let dot = if bundled_dot.is_file() {
            bundled_dot
        } else {
            find_tool("dot").unwrap_or_default()
        };
        Ok(Self {
            bundle_root,
            java_home,
            dot,
        })
    }

    fn callatlas(&self) -> Result<PathBuf> {
        choose_artifact(
            &self.bundle_root,
            &[
                "analyzers/callatlas/callatlas.dll",
                "vendor/callatlas/src/CallAtlas.Cli/bin/Release/net10.0/callatlas.dll",
            ],
        )
    }

    fn bytecode_lib(&self) -> Result<PathBuf> {
        choose_artifact(
            &self.bundle_root,
            &[
                "analyzers/java-bytecode/lib/codegraph-bytecode.jar",
                "tools/java-bytecode/build/install/codegraph-bytecode/lib/codegraph-bytecode.jar",
            ],
        )
        .map(|jar| jar.parent().unwrap().into())
    }
}

fn discover_jdk25() -> Option<PathBuf> {
    let configured = std::env::var_os("JAVA_HOME")
        .map(PathBuf::from)
        .filter(|path| is_jdk25(path));
    configured.or_else(|| {
        Command::new("/usr/libexec/java_home")
            .args(["-v", "25"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()))
    })
}

fn is_jdk25(path: &Path) -> bool {
    if !path.join("bin/javac").is_file() {
        return false;
    }
    let release = fs::read_to_string(path.join("release")).unwrap_or_default();
    let version = release
        .lines()
        .find_map(|line| line.strip_prefix("JAVA_VERSION="));
    version.is_some_and(|version| {
        version
            .trim_matches('"')
            .split(|character: char| !character.is_ascii_digit())
            .next()
            == Some("25")
    })
}

fn choose_artifact(root: &Path, choices: &[&str]) -> Result<PathBuf> {
    choices.iter().map(|choice| root.join(choice)).find(|path| path.is_file())
        .with_context(|| format!("Analyzer artifacts missing under {}. Run tools/bootstrap.py before building the app.", root.display()))
}

fn find_tool(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&paths)
        .chain([
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/bin"),
        ])
        .map(|path| path.join(name))
        .find(|path| path.is_file())
}

pub fn analyze(
    builds: Vec<BuildTarget>,
    runtime: &AnalyzerRuntime,
    cancel: &AtomicBool,
    progress: &dyn Fn(String),
) -> Result<GraphData> {
    ensure!(!builds.is_empty(), "Select at least one build");
    let mut job = JobDirectory::new()?;
    let mut graph = GraphData::default();
    for (index, build) in builds.iter().enumerate() {
        check_cancel(cancel)?;
        progress(format!(
            "Analyzing {} ({}/{})",
            build.label,
            index + 1,
            builds.len()
        ));
        let workspace = job.path.join(index.to_string());
        fs::create_dir_all(&workspace)?;
        let context = AnalysisJob {
            runtime,
            path: &workspace,
            cancel,
            progress,
        };
        let part = analyze_build(build, &context)
            .with_context(|| format!("Analysis failed. Logs: {}", workspace.display()))?;
        append_graph(&mut graph, part)?;
    }
    progress("Verifying source state…".into());
    ensure_unchanged(&graph.fingerprints)?;
    graph.engine_versions = engine_versions()?;
    graph.validate()?;
    ensure!(
        !graph.nodes.is_empty(),
        "Analyzers emitted no project symbols"
    );
    check_cancel(cancel)?;
    job.remove_on_drop = true;
    Ok(graph)
}

struct AnalysisJob<'a> {
    runtime: &'a AnalyzerRuntime,
    path: &'a Path,
    cancel: &'a AtomicBool,
    progress: &'a dyn Fn(String),
}

fn analyze_build(build: &BuildTarget, context: &AnalysisJob<'_>) -> Result<GraphData> {
    match build.language.as_str() {
        "csharp" => csharp::analyze_csharp(build, context),
        "java" => java::analyze_java(build, context),
        other => bail!("Unsupported language: {other}"),
    }
}

fn append_graph(graph: &mut GraphData, mut part: GraphData) -> Result<()> {
    let offset = u32::try_from(graph.nodes.len())?;
    for edge in &mut part.edges {
        edge.from = edge
            .from
            .checked_add(offset)
            .context("Graph exceeds node limit")?;
        edge.to = edge
            .to
            .checked_add(offset)
            .context("Graph exceeds node limit")?;
    }
    graph.nodes.extend(part.nodes);
    graph.edges.extend(part.edges);
    graph.diagnostics.extend(part.diagnostics);
    graph.roots.extend(part.roots);
    graph.fingerprints.extend(part.fingerprints);
    Ok(())
}

static JOB_SEQUENCE: AtomicU64 = AtomicU64::new(0);
struct JobDirectory {
    path: PathBuf,
    remove_on_drop: bool,
}
impl JobDirectory {
    fn new() -> Result<Self> {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!(
            "codegraph-{}-{stamp}-{}",
            std::process::id(),
            JOB_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path)?;
        Ok(Self {
            path,
            remove_on_drop: false,
        })
    }
}
impl Drop for JobDirectory {
    fn drop(&mut self) {
        if self.remove_on_drop {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

fn source_snapshot(roots: &[PathBuf]) -> Result<Vec<FileFingerprint>> {
    fingerprint::capture(fingerprint::source_paths(roots)?)
}

fn engine_versions() -> Result<String> {
    let lock: serde_json::Value = serde_json::from_str(include_str!("../analyzers.lock.json"))?;
    Ok(serde_json::json!({"callatlas": lock["callatlas"], "callatlas_protocol": "callatlas.ir.v3", "java-callgraph2": lock["java-callgraph2"]}).to_string())
}

fn ensure_unchanged(before: &[FileFingerprint]) -> Result<()> {
    ensure!(
        !fingerprint::is_stale(before)?,
        "Project inputs changed during analysis. Refresh again after editing stops; the previous graph is preserved."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edits_after_the_pre_analysis_snapshot_are_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("App.java");
        fs::write(&source, "class App {}").unwrap();
        let before = source_snapshot(&[directory.path().into()]).unwrap();
        fs::write(source, "class Changed {}").unwrap();
        assert!(ensure_unchanged(&before).is_err());
    }

    #[test]
    fn configured_java_home_requires_a_full_jdk25() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir(directory.path().join("bin")).unwrap();
        fs::write(directory.path().join("bin/javac"), "").unwrap();
        fs::write(
            directory.path().join("release"),
            "JAVA_VERSION=\"21.0.3\"\n",
        )
        .unwrap();
        assert!(!is_jdk25(directory.path()));
        fs::write(
            directory.path().join("release"),
            "JAVA_VERSION=\"25.0.3\"\n",
        )
        .unwrap();
        assert!(is_jdk25(directory.path()));
    }
}
