//! Headless acceptance entry point for the same pipeline used by the desktop app.
use anyhow::{Context, Result, ensure};
use codegraph_engine::{
    analyzers::{AnalyzerRuntime, analyze},
    discovery::BuildTarget,
};
use std::{path::PathBuf, sync::atomic::AtomicBool};

fn main() -> Result<()> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    ensure!(
        arguments.len() == 3,
        "Usage: analyzer_check csharp|java BUILD_PATH OUTPUT_JSON"
    );
    let path = PathBuf::from(&arguments[1]).canonicalize()?;
    let language = arguments[0].clone();
    let kind = if language == "java" {
        "gradle"
    } else if matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("sln" | "slnx")
    ) {
        "solution"
    } else {
        "project"
    };
    let target = BuildTarget {
        label: path
            .file_name()
            .context("Build has no name")?
            .to_string_lossy()
            .into_owned(),
        path,
        language,
        kind: kind.into(),
    };
    let graph = analyze(
        vec![target],
        &AnalyzerRuntime::discover()?,
        &AtomicBool::new(false),
        &|message| eprintln!("{message}"),
    )?;
    std::fs::write(&arguments[2], serde_json::to_vec_pretty(&graph)?)?;
    println!(
        "{} symbols, {} relationships, {} diagnostics",
        graph.nodes.len(),
        graph.edges.len(),
        graph.diagnostics.len()
    );
    Ok(())
}
