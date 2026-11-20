//! The library records the effective native selection, including discovery defaults.
use super::Observations;
use anyhow::Result;
use codecallnavigator_core::artifact::{Roots, selection};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Serialize)]
struct AnalyzerChoice<'a> {
    path: String,
    kind: &'a str,
    language: &'a str,
    backend: &'static str,
    version: String,
    mode: &'static str,
}

pub(super) fn capture(
    observations: &Observations<'_>,
    dependency_identity: Option<&str>,
) -> Result<Value> {
    let analysis = observations.analysis;
    let mut analyzers = observations
        .evidence
        .iter()
        .map(|evidence| describe(evidence, observations.roots))
        .collect::<Result<Vec<_>>>()?;
    analyzers.sort_by(|a, b| (&a.path, a.kind, a.language).cmp(&(&b.path, b.kind, b.language)));
    Ok(json!({
        "builds": analysis.selection,
        "targets": analysis.targets,
        "roots": analysis.roots,
        "analyzers": analyzers,
        "evidence_policy": "evaluated_build.v1",
        "engine_version": env!("CARGO_PKG_VERSION"),
        "dependency_identity": dependency_identity,
        "engine_versions": analysis.engine_versions
    }))
}

fn describe<'a>(
    evidence: &'a codecallnavigator_engine::analyzers::evidence::TargetEvidence,
    roots: &Roots,
) -> Result<AnalyzerChoice<'a>> {
    let target = &evidence.target;
    let absolute = target.path.to_string_lossy();
    let path = selection::capture(std::slice::from_ref(target), roots)?
        .targets
        .into_iter()
        .next()
        .map_or_else(|| absolute.into_owned(), |target| target.path);
    Ok(AnalyzerChoice {
        path,
        kind: &target.kind,
        language: &target.language,
        backend: evidence.analyzer.backend,
        version: evidence.analyzer.version.clone(),
        mode: evidence.analyzer.mode,
    })
}
