use super::cancellation::CANCELLED;
use super::snapshot_retention::{Candidate, Observations, VerifiedSnapshot, dependency_identity};
use anyhow::{Context, Result, ensure};
use codecallnavigator_core::{
    artifact::{
        self,
        local_snapshots::{SnapshotRecord, WorkingPublication},
    },
    discovery::{self, BuildTarget},
    metrics,
    model::{FileFingerprint, GraphData},
    protocol::{Event, GraphRetention},
};
use codecallnavigator_engine::analyzers::{self, AnalyzerRuntime, ObservedAnalysis};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};
mod metadata;

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
    Ok(native_attempt(
        targets,
        &Attempt {
            cancel: &CANCELLED,
            progress,
            diagnostic: progress,
        },
    )?
    .graph)
}

fn native_attempt(targets: Vec<BuildTarget>, attempt: &Attempt<'_>) -> Result<ObservedAnalysis> {
    analyzers::analyze_observed(
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
            diagnostic: progress,
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
    pub diagnostic: &'a dyn Fn(String),
}

/// Produce native data only after acquiring the project lock and generation guard.
/// The resulting owned graph is mutated by inference, never a retained raw cache.
pub(super) fn publish<F>(
    mut publication: Publication<'_>,
    attempt: Attempt<'_>,
    produce: F,
) -> Result<Event>
where
    F: FnOnce(Vec<BuildTarget>, &Attempt<'_>) -> Result<ObservedAnalysis>,
{
    let project = publication
        .project
        .canonicalize()
        .context("Cannot open project")?;
    ensure!(project.is_dir(), "Project must be a directory");
    let roots = roots(
        &project,
        std::mem::take(&mut publication.options.configured),
    )?;
    let lock = artifact::AnalysisLock::acquire(&project)?;
    if let Some(expected) = &publication.options.expected_generation {
        lock.require_generation(expected)?;
    }
    let metrics = metrics::Metrics::default();
    let _scope = metrics.enter();
    let outcome = prepare(
        AnalysisBinding {
            project: &project,
            roots,
            targets: publication.targets,
            infer_linked: publication.options.infer_linked,
            use_inference_cache: true,
        },
        &attempt,
        produce,
    )?;
    let retention = outcome.publish_working(&lock, &attempt)?;
    (attempt.diagnostic)(retention.to_string());
    super::inference::finish(outcome.cache, attempt.diagnostic);
    metrics.finish();
    Ok(Event::Complete {
        roots: outcome.roots,
        digest: outcome.digest,
        fingerprints: outcome.graph.fingerprints,
        engine_versions: outcome.graph.engine_versions,
        metrics: serde_json::to_value(metrics.report())?,
        retention: Some(retention),
    })
}

pub(super) fn prepare_revision(
    project: &Path,
    targets: Vec<BuildTarget>,
    progress: &dyn Fn(String),
) -> Result<PreparedRevision> {
    let outcome = prepare(
        AnalysisBinding {
            project,
            roots: roots(project, Vec::new())?,
            targets,
            infer_linked: false,
            use_inference_cache: false,
        },
        &Attempt {
            cancel: &CANCELLED,
            progress,
            diagnostic: progress,
        },
        native_attempt,
    )?;
    let verified = outcome
        .verified
        .context("The requested revision graph could not be verified")?;
    verified.require_current()?;
    Ok(PreparedRevision {
        encoded: outcome.encoded,
        metadata: outcome.metadata,
        verified,
    })
}

pub(super) struct PreparedRevision {
    pub encoded: artifact::EncodedArtifact,
    pub metadata: serde_json::Value,
    pub verified: VerifiedSnapshot,
}

struct AnalysisBinding<'a> {
    project: &'a Path,
    roots: Vec<(String, PathBuf)>,
    targets: Vec<BuildTarget>,
    infer_linked: bool,
    use_inference_cache: bool,
}

fn prepare<F>(
    binding: AnalysisBinding<'_>,
    attempt: &Attempt<'_>,
    produce: F,
) -> Result<PreparedGraph>
where
    F: FnOnce(Vec<BuildTarget>, &Attempt<'_>) -> Result<ObservedAnalysis>,
{
    let AnalysisBinding {
        project,
        roots,
        mut targets,
        infer_linked,
        use_inference_cache,
    } = binding;
    (attempt.progress)("Preparing analysis inputs…".into());
    let snapshot = Candidate::capture(project);
    targets.sort_by_key(BuildTarget::identity);
    targets.dedup_by(|a, b| a.identity() == b.identity());
    let (boundaries, boundary_inputs) =
        codecallnavigator_core::boundary::Configuration::load(project, &targets)?;
    let application_inputs = application_inputs(project)?;
    let selection = targets.clone();
    let ObservedAnalysis {
        mut graph,
        output_attribution_limit,
    } = produce(targets, attempt)?;
    graph.fingerprints.extend(boundary_inputs);
    graph.fingerprints.extend(application_inputs);
    (attempt.progress)("Resolving graph relationships…".into());
    let cache = super::inference::resolve(project, &mut graph, &boundaries, use_inference_cache)?;
    let roots = if infer_linked {
        super::linked_roots::infer(&graph, project, roots)?
    } else {
        roots
    };
    PreparedGraph::capture(
        GraphInputs {
            graph,
            roots,
            project,
            selection: &selection,
            snapshot,
            output_attribution_limit: output_attribution_limit.as_deref(),
            cache,
        },
        attempt,
    )
}

struct GraphInputs<'a> {
    graph: GraphData,
    roots: Vec<(String, PathBuf)>,
    project: &'a Path,
    selection: &'a [BuildTarget],
    snapshot: Result<Candidate>,
    output_attribution_limit: Option<&'a str>,
    cache: Option<codecallnavigator_core::boundary::DecisionCache>,
}

struct PreparedGraph {
    graph: GraphData,
    roots: Vec<(String, PathBuf)>,
    encoded: artifact::EncodedArtifact,
    digest: String,
    metadata: serde_json::Value,
    verified: Result<VerifiedSnapshot>,
    cache: Option<codecallnavigator_core::boundary::DecisionCache>,
}

impl PreparedGraph {
    fn capture(inputs: GraphInputs<'_>, attempt: &Attempt<'_>) -> Result<Self> {
        let GraphInputs {
            graph,
            selection,
            roots,
            project,
            snapshot,
            output_attribution_limit,
            cache,
        } = inputs;
        (attempt.progress)("Encoding the graph…".into());
        let (encoded, provenance, refresh_validation) =
            encode_publication(&graph, selection, &roots)?;
        let digest = blake3::hash(encoded.as_ref()).to_hex().to_string();
        let observations = Observations {
            analysis: &provenance,
            roots: &roots,
            analyzer: &graph.fingerprints,
            refresh: &refresh_validation,
            targets: selection,
            project,
            generated_sources: graph
                .refresh_outputs
                .as_ref()
                .map_or(&[], |output| output.generated_source_directories.as_slice()),
            output_attribution_limit,
        };
        let verified = snapshot.and_then(|candidate| {
            (attempt.progress)("Verifying Git snapshot attribution…".into());
            candidate.verify(&observations, attempt.cancel)
        });
        let fallback_identity = verified
            .is_err()
            .then(|| dependency_identity(&observations))
            .transpose()?;
        let identity = verified
            .as_ref()
            .ok()
            .map(VerifiedSnapshot::dependency_identity)
            .or(fallback_identity.as_deref());
        let metadata = metadata::capture(&observations, &graph.engine_versions, identity)?;
        (attempt.progress)("Validating analyzed inputs…".into());
        verify_inputs(&graph.fingerprints, &refresh_validation, attempt.cancel)?;
        Ok(Self {
            graph,
            roots,
            encoded,
            digest,
            metadata,
            verified,
            cache,
        })
    }

    fn publish_working(
        &self,
        lock: &artifact::AnalysisLock,
        attempt: &Attempt<'_>,
    ) -> Result<GraphRetention> {
        (attempt.progress)("Publishing the graph…".into());
        match self
            .verified
            .as_ref()
            .map_err(|error| format!("{error:#}"))
            .and_then(|verified| {
                verified
                    .require_current()
                    .map_err(|error| format!("{error:#}"))?;
                Ok(verified)
            }) {
            Ok(verified) => {
                let record = lock.publish_library(
                    WorkingPublication {
                        encoded: &self.encoded,
                        metadata: &self.metadata,
                        verified: Some((verified.common_dir(), verified.identity(&self.metadata)?)),
                    },
                    attempt.cancel,
                )?;
                let SnapshotRecord { id, identity, .. } =
                    record.context("Graph library omitted its verified snapshot record")?;
                Ok(GraphRetention::Verified {
                    commit: identity.commit,
                    snapshot: id,
                })
            }
            Err(reason) => {
                lock.publish_library(
                    WorkingPublication {
                        encoded: &self.encoded,
                        metadata: &self.metadata,
                        verified: None,
                    },
                    attempt.cancel,
                )?;
                Ok(GraphRetention::WorkingOnly { reason })
            }
        }
    }
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
