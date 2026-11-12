pub use codecallnavigator::{discovery, fingerprint};
mod app;
mod architecture_workspace;
mod cache_jobs;
mod camera;
mod canvas_fixture;
mod cli;
mod edges;
mod engine_client;
mod explorer;
mod flow_scenes;
mod flow_view;
mod flow_workspace;
mod graph_emphasis;
mod graph_interaction;
mod graph_layout;
mod icons;
mod inspector;
mod jobs;
mod layer_scenes;
mod method_scenes;
mod navigation;
mod pan_profile;
mod presentation;
mod projection;
mod relationship;
mod runtime;
mod scenes;
mod search;
mod search_shortcut;
mod source;
mod source_path;
mod source_popup;
mod source_view;
mod theme;
mod traversal_scenes;
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
            ("dense-canvas", canvas_fixture::install),
            ("saved-canvas", canvas_fixture::saved),
            ("class-tree", traversal_scenes::classes),
            ("aligned-classes", traversal_scenes::aligned_classes),
            ("method-tree", traversal_scenes::methods),
            ("function-flow", flow_scenes::install),
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
        ..rune::AppSpec::new("CodeCallNavigator", (1420.0, 900.0), app::State::new, view::view)
    };
    let app = rune::App::new(spec)
        .modifier_keys()
        .on_pointer_activity(search::pointer_activity)
        .on_key_activity(search::key_activity)
        .on_focus(search::focus_changed)
        .standard_menus()
        .keep_running()
        .on_close(jobs::close);
    match pan_profile::attach(app) {
        Ok(app) => app.run(),
        Err(error) => {
            eprintln!("{error:#}");
            std::process::exit(1);
        }
    }
}
