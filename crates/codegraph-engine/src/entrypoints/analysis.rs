use super::cancellation::CANCELLED;
use anyhow::{Context, Result, ensure};
use codegraph_core::{
    artifact,
    discovery::{self, BuildTarget},
    metrics,
    model::GraphData,
    protocol::Event,
};
use codegraph_engine::analyzers::{self, AnalyzerRuntime};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub fn select_targets(project: &Path, paths: Vec<PathBuf>) -> Result<Vec<BuildTarget>> {
    let paths = if paths.is_empty() {
        vec![project.into()]
    } else {
        paths
    };
    let mut selected = BTreeMap::new();
    for path in paths {
        let path = project.join(path).canonicalize()?;
        if path.is_dir() {
            for target in default_targets(&path)? {
                selected.insert(target.identity(), target);
            }
        } else {
            let target = discovery::target_from_file(&path)?;
            selected.insert(target.identity(), target);
        }
    }
    Ok(selected.into_values().collect())
}

fn default_targets(project: &Path) -> Result<Vec<BuildTarget>> {
    let found = discovery::discover(project)?;
    let selected = discovery::default_selection(&found);
    Ok(found
        .into_iter()
        .filter(|target| selected.contains(&target.identity()))
        .collect())
}

pub fn native(targets: Vec<BuildTarget>, progress: &dyn Fn(String)) -> Result<GraphData> {
    analyzers::analyze(targets, &AnalyzerRuntime::discover()?, &CANCELLED, progress)
}

fn roots(project: &Path, configured: Vec<(String, PathBuf)>) -> Result<Vec<(String, PathBuf)>> {
    let mut roots = Vec::new();
    for (name, path) in configured {
        let path = project
            .join(path)
            .canonicalize()
            .context("Cannot open logical root")?;
        ensure!(path.is_dir(), "Logical root must be a directory");
        ensure!(
            name != "repo" || path == project,
            "Logical root repo must name the project"
        );
        roots.push((name, path));
    }
    if !roots.iter().any(|(name, _)| name == "repo") {
        roots.push(("repo".into(), project.into()));
    }
    roots.sort_by(|a, b| a.0.cmp(&b.0));
    artifact::validate_roots(&roots)?;
    Ok(roots)
}

pub fn analyze(
    project: &Path,
    configured: Vec<(String, PathBuf)>,
    targets: Vec<BuildTarget>,
    progress: &dyn Fn(String),
) -> Result<Event> {
    analyze_with_linked_roots(
        project,
        RootOptions {
            configured,
            infer_linked: false,
        },
        targets,
        progress,
    )
}

pub struct RootOptions {
    pub configured: Vec<(String, PathBuf)>,
    pub infer_linked: bool,
}

pub fn analyze_with_linked_roots(
    project: &Path,
    options: RootOptions,
    targets: Vec<BuildTarget>,
    progress: &dyn Fn(String),
) -> Result<Event> {
    let project = project.canonicalize().context("Cannot open project")?;
    ensure!(project.is_dir(), "Project must be a directory");
    let roots = roots(&project, options.configured)?;
    let lock = artifact::AnalysisLock::acquire(&project)?;
    let metrics = metrics::Metrics::default();
    let _scope = metrics.enter();
    let mut targets = targets;
    targets.sort_by_key(BuildTarget::identity);
    targets.dedup_by(|a, b| a.identity() == b.identity());
    let graph = native(targets, progress)?;
    let roots = if options.infer_linked {
        super::linked_roots::infer(&graph, &project, roots)?
    } else {
        roots
    };
    let artifact = artifact::canonical_with_identities(&graph, &roots, &graph.export_identities)?;
    let digest = blake3::hash(&artifact.bytes()?).to_hex().to_string();
    ensure!(
        !codegraph_core::fingerprint::is_stale(&graph.fingerprints)?,
        "Project inputs changed before graph publication"
    );
    lock.publish(&artifact, &CANCELLED)?;
    metrics.finish();
    Ok(Event::Complete {
        roots,
        digest,
        fingerprints: graph.fingerprints,
        engine_versions: graph.engine_versions,
        metrics: serde_json::to_value(metrics.report())?,
    })
}
