//! The library records the effective native selection, including discovery defaults.
use super::Observations;
use anyhow::{Context, Result, bail};
use codecallnavigator_core::{
    artifact::{Roots, selection},
    discovery::BuildTarget,
};
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
    runtime_versions: &str,
    dependency_identity: Option<&str>,
) -> Result<Value> {
    let runtime: Value = serde_json::from_str(runtime_versions)
        .context("Cannot record effective analyzer versions")?;
    let analysis = observations.analysis;
    let mut analyzers = observations
        .targets
        .iter()
        .map(|target| describe(target, observations.roots, &runtime))
        .collect::<Result<Vec<_>>>()?;
    analyzers.sort_by(|a, b| (&a.path, a.kind, a.language).cmp(&(&b.path, b.kind, b.language)));
    Ok(json!({
        "builds": analysis.selection,
        "targets": analysis.targets,
        "roots": analysis.roots,
        "analyzers": analyzers,
        "engine_version": env!("CARGO_PKG_VERSION"),
        "dependency_identity": dependency_identity,
        "engine_versions": analysis.engine_versions
    }))
}

fn describe<'a>(
    target: &'a BuildTarget,
    roots: &Roots,
    runtime: &Value,
) -> Result<AnalyzerChoice<'a>> {
    let absolute = target.path.to_string_lossy();
    let (backend, version, mode) = match target.language.as_str() {
        "java" => (
            "Java CallGraph2",
            &runtime["java-callgraph2"]["version"],
            "compiled bytecode and Java source",
        ),
        "csharp" => ("CallAtlas", &runtime["callatlas"]["revision"], "Roslyn"),
        "typescript" => (
            "TypeScript",
            &runtime["projects"][absolute.as_ref()]["typescript"],
            "compiler API",
        ),
        language => bail!("Cannot identify the analyzer for {language}"),
    };
    let path = selection::capture(std::slice::from_ref(target), roots)?
        .targets
        .into_iter()
        .next()
        .map_or_else(|| absolute.into_owned(), |target| target.path);
    Ok(AnalyzerChoice {
        path,
        kind: &target.kind,
        language: &target.language,
        backend,
        version: version
            .as_str()
            .context("Native analysis omitted its analyzer version")?
            .to_owned(),
        mode,
    })
}
