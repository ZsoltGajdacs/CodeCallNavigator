//! Standalone discovery and native-payload acceptance entry points.
use anyhow::{Context, Result, bail, ensure};
use codegraph_engine::{
    analyzers::{self, AnalyzerRuntime},
    discovery,
};
use std::{path::PathBuf, sync::atomic::AtomicBool};

fn run() -> Result<()> {
    let mut arguments = std::env::args().skip(1);
    let command = arguments
        .next()
        .context("Usage: codegraph-engine discover ROOT | analyze-native TARGET...")?;
    match command.as_str() {
        "discover" => {
            let root = arguments.next().context("discover requires ROOT")?;
            ensure!(arguments.next().is_none(), "discover accepts one ROOT");
            println!(
                "{}",
                serde_json::to_string(&discovery::discover(&PathBuf::from(root))?)?
            );
        }
        "analyze-native" => {
            let mut targets = Vec::new();
            for value in arguments {
                let path = PathBuf::from(value).canonicalize()?;
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
            let graph = analyzers::analyze(
                targets,
                &AnalyzerRuntime::discover()?,
                &AtomicBool::new(false),
                &|message| eprintln!("{message}"),
            )?;
            println!("{}", serde_json::to_string(&graph)?);
        }
        other => bail!("Unknown command: {other}"),
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(2);
    }
}
