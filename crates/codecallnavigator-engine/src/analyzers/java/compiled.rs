//! Retained compiler preparation and independently guarded extraction outputs.
use super::{
    analyze_compiled,
    build::{self, PreparedJava},
    contexts,
    maven::MavenSources,
    refresh_outputs,
};
use crate::{
    analyzers::{AnalysisJob, ensure_unchanged},
    discovery::BuildTarget,
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
}

#[derive(Debug)]
pub(in crate::analyzers) struct ExtractionOutputsChanged;

impl std::fmt::Display for ExtractionOutputsChanged {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Java extraction outputs changed")
    }
}

impl CompiledJava {
    pub(in crate::analyzers) fn verify(&self) -> Result<()> {
        ensure_unchanged(&self.fingerprints)?;
        ensure_unchanged(&self.extraction).context(ExtractionOutputsChanged)
    }

    pub(in crate::analyzers) fn reconcile_outputs(&mut self) -> Result<bool> {
        ensure_unchanged(&self.fingerprints)?;
        if !crate::fingerprint::is_stale(&self.extraction)? {
            return Ok(false);
        }
        // Only owned extraction output changes may advance this baseline.
        // Classpath entries consumed by another compile remain guarded above.
        build::require_compiled_outputs(&self.metadata)?;
        self.extraction = extraction_fingerprints(&self.metadata)?.0;
        self.verify()?;
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
) -> Result<CompiledJava> {
    ensure_unchanged(&prepared.configuration_before)?;
    (context.progress)(format!(
        "Compiling all Java source sets for {}…",
        build.label
    ));
    let compilation = build::compile(build, context)?;
    let metadata = compilation.metadata;
    prepared.verify_compiled(build, &metadata)?;
    let mut fingerprints = prepared.configuration_before.clone();
    fingerprints.extend(build::snapshot(build, &metadata, true)?);
    fingerprints.extend(
        compilation
            .observations
            .fingerprints(&metadata)
            .map_err(|error| compilation.observations.with_consumption(error))?,
    );
    let (extraction, external) = extraction_fingerprints(&metadata)?;
    fingerprints.extend(external);
    let compiled = CompiledJava {
        preflight: prepared,
        metadata,
        fingerprints,
        extraction,
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
    Ok(graph)
}
