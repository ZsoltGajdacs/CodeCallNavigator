//! Retained compiler preparation and independently guarded extraction outputs.
use super::{
    analyze_compiled,
    build::{self, PreparedJava},
    compile_observation::{FreshOutputs, Observations},
    contexts,
    evidence::Roles,
    maven::MavenSources,
    refresh_outputs,
};
use crate::{
    analyzers::{
        AnalysisJob, CleanSnapshotCheckout, ensure_unchanged,
        evidence::{AnalyzerDescriptor, EvidenceInput, EvidenceRole},
    },
    discovery::BuildTarget,
    fingerprint::InputKind,
    model::{FileFingerprint, GraphData},
};
use anyhow::{Context, Result};
use std::path::Path;

/// Native inputs guard compiler reuse. The separate class view can be reconciled
/// between attempts when no changed class was a native compiler input.
pub(in crate::analyzers) struct CompiledJava {
    preflight: PreparedJava,
    metadata: build::JavaBuildMetadata,
    fingerprints: Vec<FileFingerprint>,
    extraction: Vec<FileFingerprint>,
    watches: Vec<FileFingerprint>,
    observations: Observations,
    roles: Roles,
    output_attribution_limit: Option<String>,
}

#[derive(Debug)]
pub(in crate::analyzers) struct ExtractionOutputsChanged;

impl std::fmt::Display for ExtractionOutputsChanged {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Java extraction outputs changed")
    }
}

impl CompiledJava {
    pub(in crate::analyzers) fn descriptor(
        &self,
        versions: &serde_json::Value,
    ) -> Result<AnalyzerDescriptor> {
        Ok(AnalyzerDescriptor {
            backend: "Java CallGraph2",
            version: versions["java-callgraph2"]["version"]
                .as_str()
                .context("Missing Java analyzer version")?
                .to_owned(),
            mode: "compiled bytecode and Java source",
        })
    }

    pub(in crate::analyzers) fn evidence(&self) -> Vec<EvidenceInput> {
        self.fingerprints
            .iter()
            .chain(
                self.extraction
                    .iter()
                    .filter(|input| input.kind != InputKind::Directory),
            )
            .map(|input| EvidenceInput::new(input.clone(), self.roles.classify(input)))
            .chain(
                self.watches
                    .iter()
                    .cloned()
                    .map(|input| EvidenceInput::new(input, EvidenceRole::Watch)),
            )
            .collect()
    }

    pub(in crate::analyzers) fn output_attribution_limit(&self) -> Option<&str> {
        self.output_attribution_limit.as_deref()
    }

    pub(in crate::analyzers) fn verify(&self) -> Result<()> {
        self.observations.verify_membership()?;
        ensure_unchanged(&self.fingerprints)?;
        ensure_unchanged(&self.extraction).context(ExtractionOutputsChanged)
    }

    pub(in crate::analyzers) fn reconcile_outputs(&mut self) -> Result<bool> {
        self.observations.verify_membership()?;
        ensure_unchanged(&self.fingerprints)?;
        if !crate::fingerprint::is_stale(&self.extraction)? {
            return Ok(false);
        }
        // Only owned extraction output changes may advance this baseline.
        // Classpath entries consumed by another compile remain guarded above.
        build::require_compiled_outputs(&self.metadata)?;
        self.extraction = extraction_fingerprints(&self.metadata)?.0;
        self.verify()?;
        self.output_attribution_limit =
            Some("Java outputs changed after the observed compiler attempt".into());
        Ok(true)
    }

    pub(in crate::analyzers) fn preflight(&self) -> &PreparedJava {
        &self.preflight
    }

    pub(in crate::analyzers) fn maven_sources(&self) -> MavenSources {
        MavenSources::new(
            self.fingerprints
                .iter()
                .filter(|input| {
                    let path = Path::new(&input.path);
                    input.kind == crate::fingerprint::InputKind::File
                        && path
                            .extension()
                            .is_some_and(|extension| extension == "java")
                        && !self
                            .metadata
                            .output_roots
                            .iter()
                            .any(|root| path.starts_with(root))
                })
                .cloned()
                .collect(),
        )
    }
}

pub(in crate::analyzers) fn compile(
    build: &BuildTarget,
    context: &AnalysisJob<'_>,
    prepared: PreparedJava,
    snapshot_checkout: Option<&CleanSnapshotCheckout>,
) -> Result<CompiledJava> {
    ensure_unchanged(&prepared.configuration_before)?;
    (context.progress)(format!(
        "Compiling all Java source sets for {}…",
        build.label
    ));
    let declared_before = prepared.producer_inputs()?;
    let initial_outputs = FreshOutputs::before(prepared.metadata());
    let compilation = build::compile(build, context)?;
    let metadata = compilation.metadata;
    prepared.verify_compiled(build, &metadata)?;
    ensure_unchanged(&declared_before)?;
    let output_attribution_limit = initial_outputs
        .and_then(|before| before.verify(&metadata, &compilation.observations, snapshot_checkout))
        .err()
        .map(|error| format!("{error:#}"));
    let mut fingerprints = prepared.configuration_before.clone();
    fingerprints.extend(build::declared_inputs(&metadata)?);
    let mut watches = build::scheduling_inputs(build, &metadata)?;
    let journal = compilation
        .observations
        .fingerprints(&metadata)
        .map_err(|error| compilation.observations.with_consumption(error))?;
    // The compiler rechecks scoped membership. Raw directory fingerprints
    // preserve polling coverage without certifying unrelated children.
    let (journal_watches, journal_files): (Vec<_>, Vec<_>) = journal
        .into_iter()
        .partition(|input| input.kind == InputKind::Directory);
    watches.extend(journal_watches);
    fingerprints.extend(journal_files);
    let (extraction, external) = extraction_fingerprints(&metadata)?;
    fingerprints.extend(external);
    let roles = Roles::new(build, context.runtime, &metadata);
    let compiled = CompiledJava {
        preflight: prepared,
        metadata,
        fingerprints,
        watches,
        observations: compilation.observations,
        roles,
        extraction,
        output_attribution_limit,
    };
    compiled.verify()?;
    Ok(compiled)
}

fn extraction_fingerprints(
    metadata: &build::JavaBuildMetadata,
) -> Result<(Vec<FileFingerprint>, Vec<FileFingerprint>)> {
    let owned = refresh_outputs::ownership(metadata)?.bytecode_directories;
    let contexts = contexts::plan(&metadata.source_sets)?;
    Ok(contexts::snapshot(&contexts)?
        .into_iter()
        .partition(|input| {
            owned
                .iter()
                .any(|root| Path::new(&input.path).starts_with(root))
        }))
}

pub(in crate::analyzers) fn extract_java(
    build: &BuildTarget,
    context: &AnalysisJob<'_>,
    compiled: &CompiledJava,
) -> Result<GraphData> {
    compiled.verify()?;
    let mut graph = analyze_compiled(build, context, &compiled.metadata)?;
    compiled.verify()?;
    graph.fingerprints.extend(compiled.fingerprints.clone());
    graph.fingerprints.extend(compiled.extraction.clone());
    graph.fingerprints.extend(compiled.watches.clone());
    Ok(graph)
}
