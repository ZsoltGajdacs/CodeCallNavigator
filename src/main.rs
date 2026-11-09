pub use codegraph::{analyzers, discovery, fingerprint};
mod app;
mod architecture_workspace;
mod camera;
mod cli;
mod edges;
mod icons;
mod inspector;
mod jobs;
mod layer_scenes;
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
        if let Err(error) = cli::analyze() {
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
            ("architectural-layers", layer_scenes::install),
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
