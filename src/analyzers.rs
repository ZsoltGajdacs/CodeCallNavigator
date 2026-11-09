//! Orchestrates existing analyzers; language adapters never resolve calls themselves.
mod architecture;
mod compose;
mod csharp;
mod java;
mod process;
mod typescript;
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
    let _timer = crate::metrics::stage("analysis");
    ensure!(!builds.is_empty(), "Select at least one build");
    let mut job = JobDirectory::new()?;
    let mut composition = compose::GraphComposition::default();
    let context = AnalysisJob {
        runtime,
        path: &job.path,
        shared_path: &job.path,
        cancel,
        progress,
    };
    let jobs = {
        let _timer = crate::metrics::stage("preparation");
        prepare_jobs(builds, &context)?
    };
    let total = jobs.len();
    let mut versions = serde_json::Map::new();
    for (index, prepared) in jobs.into_iter().enumerate() {
        check_cancel(cancel)?;
        progress(format!(
            "Analyzing {} ({}/{total})",
            prepared.build.label,
            index + 1
        ));
        let context = AnalysisJob {
            runtime,
            path: &prepared.path,
            shared_path: &job.path,
            cancel,
            progress,
        };
        let part = analyze_build(&prepared.build, &context, prepared.preparation)
            .with_context(|| format!("Analysis failed. Logs: {}", prepared.path.display()))?;
        if !part.engine_versions.is_empty() {
            versions.insert(
                prepared.build.path.to_string_lossy().into_owned(),
                serde_json::from_str(&part.engine_versions)?,
            );
        }
        {
            let _timer = crate::metrics::stage("composition");
            composition.append(part)?;
        }
    }
    let mut graph = composition.finish();
    progress("Verifying source state…".into());
    ensure_unchanged(&graph.fingerprints)?;
    let mut engines: serde_json::Value = serde_json::from_str(&engine_versions()?)?;
    if !versions.is_empty() {
        engines["projects"] = serde_json::Value::Object(versions);
    }
    graph.engine_versions = engines.to_string();
    graph.validate()?;
    crate::metrics::count("symbols", graph.nodes.len() as u64);
    crate::metrics::count("relationships", graph.edges.len() as u64);
    check_cancel(cancel)?;
    job.remove_on_drop = true;
    Ok(graph)
}

fn prepare_jobs(builds: Vec<BuildTarget>, context: &AnalysisJob<'_>) -> Result<Vec<PreparedJob>> {
    let retain_worker = builds.len() == 1;
    let mut jobs = Vec::with_capacity(builds.len());
    for (index, build) in builds.into_iter().enumerate() {
        check_cancel(context.cancel)?;
        let path = context.path.join(index.to_string());
        fs::create_dir_all(&path)?;
        let context = AnalysisJob {
            path: &path,
            ..*context
        };
        let mut preparation = prepare_build(&build, &context)
            .with_context(|| format!("Build preparation failed. Logs: {}", path.display()))?;
        if !retain_worker && let Preparation::TypeScript(prepared) = &mut preparation {
            prepared.release_worker();
        }
        jobs.push(PreparedJob {
            build,
            path,
            preparation,
        });
    }
    java::reject_overlaps(jobs.iter().filter_map(|job| match &job.preparation {
        Preparation::Java(prepared) => Some((&job.build, prepared)),
        _ => None,
    }))?;
    typescript::reject_overlaps(jobs.iter().filter_map(|job| match &job.preparation {
        Preparation::TypeScript(prepared) => Some(prepared),
        _ => None,
    }))?;
    Ok(jobs)
}

struct AnalysisJob<'a> {
    runtime: &'a AnalyzerRuntime,
    path: &'a Path,
    /// Shared immutable intermediates live for the complete selected-build analysis.
    shared_path: &'a Path,
    cancel: &'a AtomicBool,
    progress: &'a dyn Fn(String),
}

struct PreparedJob {
    build: BuildTarget,
    path: PathBuf,
    preparation: Preparation,
}

enum Preparation {
    Csharp,
    Java(java::PreparedJava),
    TypeScript(typescript::PreparedTypeScript),
}

fn prepare_build(build: &BuildTarget, context: &AnalysisJob<'_>) -> Result<Preparation> {
    match build.language.as_str() {
        "csharp" => Ok(Preparation::Csharp),
        "java" => java::prepare(build, context).map(Preparation::Java),
        "typescript" => typescript::prepare(build, context).map(Preparation::TypeScript),
        other => bail!("Unsupported language: {other}"),
    }
}

fn analyze_build(
    build: &BuildTarget,
    context: &AnalysisJob<'_>,
    prepared: Preparation,
) -> Result<GraphData> {
    match prepared {
        Preparation::Csharp => csharp::analyze_csharp(build, context),
        Preparation::Java(prepared) => java::analyze_java(build, context, prepared),
        Preparation::TypeScript(prepared) => typescript::analyze(build, context, prepared),
    }
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
    let _timer = crate::metrics::stage("verification");
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
