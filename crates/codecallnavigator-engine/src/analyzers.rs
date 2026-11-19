//! Orchestrates existing analyzers; language adapters never resolve calls themselves.
mod architecture;
mod compose;
mod csharp;
mod java;
mod process;
mod session;
pub use session::PreparedAnalysis;

/// A stable key for the inputs a failed native operation consumed.
#[derive(Debug)]
pub struct ConsumedRevision(String);

impl ConsumedRevision {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ConsumedRevision {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Native analysis retained its consumed-input observation")
    }
}
mod typescript;
use crate::{
    discovery::BuildTarget,
    fingerprint,
    model::{FileFingerprint, GraphData},
};
use anyhow::{Context, Result, ensure};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    thread::available_parallelism,
    time::{SystemTime, UNIX_EPOCH},
};

fn build_worker_count() -> usize {
    available_parallelism().map_or(1, |cpus| (cpus.get() / 2).max(1))
}

#[derive(Clone, Debug)]
pub struct AnalyzerRuntime {
    pub bundle_root: PathBuf,
    pub java_home: Option<PathBuf>,
}

impl AnalyzerRuntime {
    pub fn discover() -> Result<Self> {
        let executable = std::env::current_exe()?;
        let resources = executable
            .parent()
            .and_then(Path::parent)
            .unwrap_or(Path::new("."))
            .join("Resources");
        let bundle_root = if let Some(configured) = std::env::var_os("CODECALLNAVIGATOR_RESOURCES")
        {
            let root = PathBuf::from(configured);
            ensure!(
                root.is_dir(),
                "CODECALLNAVIGATOR_RESOURCES is not a directory: {}",
                root.display()
            );
            root
        } else if resources.join("analyzers").is_dir() {
            resources
        } else {
            crate::repository_root()
        };
        let java_home = discover_jdk25();
        Ok(Self {
            bundle_root,
            java_home,
        })
    }

    fn callatlas(&self) -> Result<PathBuf> {
        choose_artifact(
            &self.bundle_root,
            &[
                "analyzers/callatlas/callatlas.dll",
                "vendor/callatlas/src/CallAtlas.Cli/bin/Release/net11.0/callatlas.dll",
            ],
        )
    }

    fn bytecode_lib(&self) -> Result<PathBuf> {
        choose_artifact(
            &self.bundle_root,
            &[
                "analyzers/java-bytecode/lib/codecallnavigator-bytecode.jar",
                "tools/java-bytecode/build/install/codecallnavigator-bytecode/lib/codecallnavigator-bytecode.jar",
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
    Ok(analyze_observed(builds, runtime, cancel, progress)?.graph)
}

pub struct ObservedAnalysis {
    pub graph: GraphData,
    pub output_attribution_limit: Option<String>,
}

pub fn analyze_observed(
    builds: Vec<BuildTarget>,
    runtime: &AnalyzerRuntime,
    cancel: &AtomicBool,
    progress: &dyn Fn(String),
) -> Result<ObservedAnalysis> {
    PreparedAnalysis::new(builds, runtime.clone())?.into_observed(cancel, progress)
}

struct AnalysisJob<'a> {
    runtime: &'a AnalyzerRuntime,
    path: &'a Path,
    /// Shared immutable intermediates live for the complete selected-build analysis.
    shared_path: &'a Path,
    cancel: &'a AtomicBool,
    progress: &'a dyn Fn(String),
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
            "codecallnavigator-{}-{stamp}-{}",
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
    let roots = roots
        .iter()
        .cloned()
        .map(fingerprint::ScanRoot::Required)
        .collect::<Vec<_>>();
    fingerprint::capture_excluding(fingerprint::collect_inputs(&roots, &[], true)?, &[])
}

fn engine_versions() -> Result<String> {
    let lock: serde_json::Value =
        serde_json::from_str(include_str!("../../../analyzers.lock.json"))?;
    Ok(serde_json::json!({"input_observation_policy":"scoped_v1", "java_compile_observation": 1, "callatlas": lock["callatlas"], "callatlas_protocol": "callatlas.ir.v3", "csharp_input_policy": codecallnavigator_core::artifact::provenance::CSHARP_INPUT_POLICY, "callatlas_framework_facts": "1", "callatlas_http_facts": "1", "java-callgraph2": lock["java-callgraph2"]}).to_string())
}

fn ensure_unchanged(before: &[FileFingerprint]) -> Result<()> {
    let _timer = crate::metrics::stage("verification");
    fingerprint::require_unchanged(
        before,
        "Project inputs changed during analysis. Refresh again after editing stops; the previous graph is preserved.",
    )?;
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
