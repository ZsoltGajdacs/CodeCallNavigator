//! Headless entrypoint using the same instrumented pipeline as the native app.
use anyhow::{Context, Result, ensure};
use codegraph::{analyzers, discovery, metrics, store::SqliteStore};
use std::{path::PathBuf, sync::atomic::AtomicBool};

struct Arguments {
    paths: Vec<PathBuf>,
    timings: Option<PathBuf>,
}

fn arguments(values: impl Iterator<Item = String>) -> Result<Arguments> {
    let mut values = values.peekable();
    let mut result = Arguments {
        paths: Vec::new(),
        timings: None,
    };
    let mut options = true;
    while let Some(value) = values.next() {
        match value.as_str() {
            "--" if options => options = false,
            "--timings" if options => {
                ensure!(
                    result.timings.is_none(),
                    "--timings may only be specified once"
                );
                result.timings = Some(
                    values
                        .next()
                        .context("--timings requires a JSON output path")?
                        .into(),
                );
            }
            _ => result.paths.push(value.into()),
        }
    }
    ensure!(
        !result.paths.is_empty(),
        "Usage: codegraph analyze [--timings REPORT.json] SOLUTION_OR_PROJECT_OR_POM_OR_TSCONFIG_OR_BUILD_ROOT..."
    );
    Ok(result)
}

pub fn analyze() -> Result<()> {
    let arguments = arguments(std::env::args().skip(2))?;
    let metrics = metrics::Metrics::default();
    let _scope = metrics.enter();
    let result = run(arguments.paths);
    metrics.finish();
    if let Some(path) = arguments.timings {
        std::fs::write(&path, serde_json::to_vec_pretty(&metrics.report())?)
            .with_context(|| format!("Cannot write timing report {}", path.display()))?;
    }
    result
}

fn run(paths: Vec<PathBuf>) -> Result<()> {
    let targets = discover(paths)?;
    let runtime = analyzers::AnalyzerRuntime::discover()?;
    let cancelled = AtomicBool::new(false);
    let graph = analyzers::analyze(targets, &runtime, &cancelled, &|message| {
        eprintln!("{message}")
    })?;
    let store = SqliteStore::open(crate::jobs::data_directory())?;
    let generation = {
        let _timer = metrics::stage("publication");
        store.publish(&graph, &cancelled)?
    };
    println!(
        "Published {generation}: {} symbols, {} relationships",
        graph.nodes.len(),
        graph.edges.len()
    );
    for diagnostic in graph.diagnostics {
        eprintln!("Analysis note: {diagnostic}");
    }
    Ok(())
}

fn discover(paths: Vec<PathBuf>) -> Result<Vec<discovery::BuildTarget>> {
    let _timer = metrics::stage("discovery");
    let mut targets = Vec::new();
    for path in paths {
        let path = path.canonicalize()?;
        if path.is_dir() {
            let found = discovery::discover(&path)?;
            let selected = discovery::default_selection(&found);
            targets.extend(
                found
                    .into_iter()
                    .filter(|target| selected.contains(&target.identity())),
            );
        } else {
            targets.push(discovery::target_from_file(&path)?);
        }
    }
    Ok(targets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timing_option_requires_a_unique_destination_and_preserves_paths() {
        let parse = |values: &[&str]| arguments(values.iter().map(|value| (*value).into()));
        let arguments = parse(&["--timings", "report.json", "a/pom.xml", "b"]).unwrap();
        assert_eq!(arguments.timings, Some("report.json".into()));
        assert_eq!(
            arguments.paths,
            vec![PathBuf::from("a/pom.xml"), PathBuf::from("b")]
        );
        assert!(parse(&["--timings"]).is_err());
        assert!(parse(&["a", "--timings", "one", "--timings", "two"]).is_err());
        assert_eq!(
            parse(&["--", "--timings"]).unwrap().paths,
            vec![PathBuf::from("--timings")]
        );
    }
}
