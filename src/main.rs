pub use codegraph::{analyzers, discovery, fingerprint};
mod app;
mod camera;
mod edges;
mod icons;
mod inspector;
mod jobs;
mod method_scenes;
mod navigation;
mod presentation;
mod projection;
mod relationship;
mod scenes;
mod search;
mod source;
mod source_popup;
mod theme;
#[cfg(test)]
mod ui_tests;
mod view;
mod workspace;

fn main() {
    if std::env::args().nth(1).as_deref() == Some("analyze") {
        if let Err(error) = analyze_cli() {
            eprintln!("{error:#}");
            std::process::exit(1);
        }
        return;
    }
    let spec = rune::AppSpec {
        min_size: Some((1050.0, 680.0)),
        scenes: &[
            ("fixture", scenes::install),
            ("source-preview", source_popup::scenes::source),
            ("source-files", source_popup::scenes::files),
            ("method-callees", method_scenes::callees),
            ("method-callers", method_scenes::callers),
            ("cached", scenes::cached),
            ("cached-focus", scenes::cached_focus),
        ],
        setup: Some(jobs::setup),
        poll: Some(jobs::poll),
        on_key: Some(view::key),
        ..rune::AppSpec::new("CodeGraph", (1420.0, 900.0), app::State::new, view::view)
    };
    rune::App::new(spec)
        .standard_menus()
        .keep_running()
        .on_close(jobs::close)
        .run();
}

fn analyze_cli() -> anyhow::Result<()> {
    use std::{path::PathBuf, sync::atomic::AtomicBool};
    let mut targets = Vec::new();
    for argument in std::env::args().skip(2) {
        let path = PathBuf::from(argument).canonicalize()?;
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
    anyhow::ensure!(
        !targets.is_empty(),
        "Usage: codegraph analyze SOLUTION_OR_PROJECT_OR_POM_OR_TSCONFIG_OR_BUILD_ROOT..."
    );
    let runtime = analyzers::AnalyzerRuntime::discover()?;
    let cancelled = AtomicBool::new(false);
    let graph = analyzers::analyze(targets, &runtime, &cancelled, &|message| {
        eprintln!("{message}")
    })?;
    let store = codegraph::store::SqliteStore::open(jobs::data_directory())?;
    let generation = store.publish(&graph, &cancelled)?;
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
