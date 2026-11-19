use super::cancellation::CANCELLED;
use super::snapshot_retention::{Candidate, Observations};
use anyhow::{Context, Result, ensure};
use codecallnavigator_core::{
    artifact,
    discovery::{self, BuildTarget},
    metrics,
    model::{FileFingerprint, GraphData},
    protocol::Event,
};
use codecallnavigator_engine::analyzers::{self, AnalyzerRuntime};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
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
    native_attempt(
        targets,
        &Attempt {
            cancel: &CANCELLED,
            progress,
        },
    )
}

fn native_attempt(targets: Vec<BuildTarget>, attempt: &Attempt<'_>) -> Result<GraphData> {
    analyzers::analyze(
        targets,
        &AnalyzerRuntime::discover()?,
        attempt.cancel,
        attempt.progress,
    )
}

pub(super) fn roots(
    project: &Path,
    configured: Vec<(String, PathBuf)>,
) -> Result<Vec<(String, PathBuf)>> {
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
            expected_generation: None,
        },
        targets,
        progress,
    )
}

pub struct RootOptions {
    pub configured: Vec<(String, PathBuf)>,
    pub infer_linked: bool,
    pub expected_generation: Option<String>,
}

pub fn analyze_with_linked_roots(
    project: &Path,
    options: RootOptions,
    targets: Vec<BuildTarget>,
    progress: &dyn Fn(String),
) -> Result<Event> {
    publish(
        Publication {
            project,
            options,
            targets,
        },
        Attempt {
            cancel: &CANCELLED,
            progress,
        },
        native_attempt,
    )
}

/// The selected project and roots belong to the same locked publication attempt.
pub(super) struct Publication<'a> {
    pub project: &'a Path,
    pub options: RootOptions,
    pub targets: Vec<BuildTarget>,
}

/// Cancellation and reporting belong to one attempt, independent of native state.
pub(super) struct Attempt<'a> {
    pub cancel: &'a AtomicBool,
    pub progress: &'a dyn Fn(String),
}

/// Produce native data only after acquiring the project lock and generation guard.
/// The resulting owned graph is mutated by inference, never a retained raw cache.
pub(super) fn publish<F>(
    publication: Publication<'_>,
    attempt: Attempt<'_>,
    produce: F,
) -> Result<Event>
where
    F: FnOnce(Vec<BuildTarget>, &Attempt<'_>) -> Result<GraphData>,
{
    let Publication {
        project,
        options,
        mut targets,
    } = publication;
    let project = project.canonicalize().context("Cannot open project")?;
    ensure!(project.is_dir(), "Project must be a directory");
    let roots = roots(&project, options.configured)?;
    let lock = artifact::AnalysisLock::acquire(&project)?;
    let snapshot = Candidate::capture(&project, attempt.progress);
    if let Some(expected) = &options.expected_generation {
        lock.require_generation(expected)?;
    }
    let metrics = metrics::Metrics::default();
    let _scope = metrics.enter();
    targets.sort_by_key(BuildTarget::identity);
    targets.dedup_by(|a, b| a.identity() == b.identity());
    let (boundaries, boundary_inputs) =
        codecallnavigator_core::boundary::Configuration::load(&project, &targets)?;
    let application_inputs = application_inputs(&project)?;
    let selection = targets.clone();
    let mut graph = produce(targets, &attempt)?;
    graph.fingerprints.extend(boundary_inputs);
    graph.fingerprints.extend(application_inputs);
    let cache = super::inference::resolve(&project, &mut graph, &boundaries)?;
    let roots = if options.infer_linked {
        super::linked_roots::infer(&graph, &project, roots)?
    } else {
        roots
    };
    let (bytes, provenance, refresh_validation) = encode_publication(&graph, &selection, &roots)?;
    let digest = blake3::hash(bytes.as_ref()).to_hex().to_string();
    verify_inputs(&graph.fingerprints, &refresh_validation, attempt.cancel)?;
    lock.publish_encoded(&bytes, attempt.cancel)?;
    retain_snapshot(
        snapshot,
        Observations {
            analysis: &provenance,
            roots: &roots,
            analyzer: &graph.fingerprints,
            refresh: &refresh_validation,
            targets: &selection,
            generated_sources: graph
                .refresh_outputs
                .as_ref()
                .map_or(&[], |output| output.generated_source_directories.as_slice()),
        },
        &bytes,
        &attempt,
    );
    super::inference::finish(cache, attempt.progress);
    metrics.finish();
    Ok(Event::Complete {
        roots,
        digest,
        fingerprints: graph.fingerprints,
        engine_versions: graph.engine_versions,
        metrics: serde_json::to_value(metrics.report())?,
    })
}

fn encode_publication(
    graph: &GraphData,
    selection: &[BuildTarget],
    roots: &artifact::Roots,
) -> Result<(
    artifact::EncodedArtifact,
    artifact::provenance::Analysis,
    Vec<FileFingerprint>,
)> {
    let mut artifact = artifact::canonical_with_identities(graph, roots, &graph.export_identities)?;
    let mut provenance = artifact::provenance::build_selected(
        &graph.fingerprints,
        selection,
        &graph.engine_versions,
        roots,
    )?;
    let validation = refresh_inputs(graph, selection, roots, &mut provenance)?;
    artifact.graph["analysis"] = serde_json::to_value(&provenance)?;
    Ok((artifact.encode()?, provenance, validation))
}

fn retain_snapshot(
    candidate: Option<Candidate>,
    observed: Observations<'_>,
    bytes: &artifact::EncodedArtifact,
    attempt: &Attempt<'_>,
) {
    if let Some(candidate) = candidate
        && let Err(error) = candidate.retain(observed, bytes, attempt.cancel)
    {
        (attempt.progress)(format!("Local snapshot was not retained: {error:#}"));
    }
}

fn verify_inputs(
    analysis: &[FileFingerprint],
    refresh: &[FileFingerprint],
    cancel: &AtomicBool,
) -> Result<()> {
    codecallnavigator_core::fingerprint::require_unchanged_cancellable(
        analysis,
        "Project inputs changed before graph publication",
        cancel,
    )?;
    codecallnavigator_core::fingerprint::require_unchanged_cancellable(
        refresh,
        "Input membership changed before graph publication",
        cancel,
    )
}

fn refresh_inputs(
    graph: &GraphData,
    selection: &[BuildTarget],
    roots: &artifact::Roots,
    provenance: &mut artifact::provenance::Analysis,
) -> Result<Vec<FileFingerprint>> {
    let Some(outputs) = &graph.refresh_outputs else {
        return Ok(Vec::new());
    };
    let safe = selection.iter().all(|target| target.language == "java");
    let captured = artifact::refresh_inputs::capture(provenance, outputs, safe, roots)?;
    provenance.refresh_inputs = Some(captured.value);
    Ok(captured.validation)
}

fn application_inputs(project: &Path) -> Result<Vec<FileFingerprint>> {
    codecallnavigator_core::fingerprint::capture_excluding(
        [
            project.join(".codecallnavigator/layers.yml"),
            project.join(".interactive-review/layers.yml"),
        ],
        &[],
    )
}
