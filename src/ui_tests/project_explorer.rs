//! Filesystem and graph interactions through the production Rune view and event loop.
use super::click;
use crate::{
    app::State,
    jobs,
    projection::{FocusTarget, Grouping},
    view,
};
use codecallnavigator::{
    graph::GraphIndex,
    model::{GraphData, Node},
};
use rune::{Env, Runtime};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

struct Fixture {
    directory: tempfile::TempDir,
    logical_root: PathBuf,
    runtime: Runtime<State>,
}
impl Fixture {
    fn new(size: (f32, f32), dark: bool, extra_files: usize) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        fs::create_dir_all(root.join("a/deep")).unwrap();
        fs::write(
            root.join("a/deep/Shared.cs"),
            "class Alpha {}\nclass Beta {}\n",
        )
        .unwrap();
        fs::write(root.join("z.cs"), "class Gamma {}\n").unwrap();
        fs::write(root.join("tests.cs"), "class Tests {}\n").unwrap();
        fs::write(root.join(".hidden"), "hidden").unwrap();
        fs::write(root.join("README.txt"), "not analyzed").unwrap();
        for index in 0..extra_files {
            fs::write(root.join(format!("file-{index:04}.txt")), "x").unwrap();
        }
        let mut state = State::new();
        state.scene = true;
        state.status = "Temporary explorer fixture · no analyzer executed".into();
        state.root = rune::TextEdit::new(root.to_string_lossy());
        state.graph = Some(Arc::new(GraphIndex::new(Arc::new(data(root))).unwrap()));
        state.nav.grouping = Grouping::Project;
        state.changed();
        let mut runtime = Runtime::new(state, view::view, size, Env::new(dark, 1.0));
        runtime.poll = Some(jobs::poll);
        runtime.on_key = Some(view::key);
        let logical_root = root.to_path_buf();
        let mut fixture = Self {
            directory,
            logical_root,
            runtime,
        };
        fixture.settle();
        fixture
    }
    fn path(&self, relative: &str) -> String {
        self.logical_root
            .join(relative)
            .to_string_lossy()
            .into_owned()
    }
    fn graph_path(&self, relative: &str) -> String {
        fs::canonicalize(self.directory.path())
            .unwrap()
            .join(relative)
            .to_string_lossy()
            .into_owned()
    }
    fn key(&self, relative: &str) -> String {
        let path = self.path(relative);
        self.runtime
            .state
            .explorer
            .rows
            .iter()
            .find(|row| row.path == path)
            .unwrap_or_else(|| panic!("Missing {relative}"))
            .key()
    }
    fn settle(&mut self) {
        for _ in 0..45 {
            self.runtime.frame(1.0 / 60.0);
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    fn file_click(&mut self, relative: &str) {
        let key = self.key(relative);
        let scroll_key = key.clone();
        self.runtime
            .update(move |_, tx| tx.scroll_into_view_in("explorer-scroll", scroll_key));
        self.settle();
        click(&mut self.runtime, &key);
    }
    fn double_click(&mut self, relative: &str) {
        let key = self.key(relative);
        self.runtime.update({
            let key = key.clone();
            move |_, tx| tx.scroll_into_view_in("explorer-scroll", key)
        });
        self.settle();
        let (x, y) = self.runtime.find(&key).unwrap();
        for _ in 0..2 {
            self.runtime.pointer_down(x, y);
            self.runtime.pointer_up(x, y);
            self.runtime.frame(0.05);
        }
        self.settle();
    }
    fn assert_selected(&self, relative: &str) {
        let expected = BTreeSet::from([self.path(relative)]);
        assert_eq!(self.runtime.state.explorer.selected, expected);
    }
    fn assert_file_focus(&self, relative: &str) {
        let expected = Some(FocusTarget::FileClasses(self.graph_path(relative)));
        assert_eq!(self.runtime.state.nav.focus_target, expected);
        assert_eq!(self.runtime.state.graph_view.focus_keys.len(), 2);
        assert_eq!(focused(&self.runtime).as_deref(), Some("graph-canvas"));
    }
    #[cfg(unix)]
    fn select_symlink_root(&mut self) -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        let alias = directory.path().join("selected-project");
        std::os::unix::fs::symlink(self.directory.path(), &alias).unwrap();
        std::os::unix::fs::symlink(
            self.directory.path(),
            self.directory.path().join("child-link"),
        )
        .unwrap();
        let root = alias.to_string_lossy().into_owned();
        self.logical_root = alias;
        self.runtime.update(move |state, _| {
            state.root = rune::TextEdit::new(root);
            state.root_edited();
            state.select_method(0, crate::graph_interaction::SelectionAction::Inspect);
        });
        self.settle();
        directory
    }
    fn expand(&mut self) {
        self.file_click("a");
        self.settle();
        self.file_click("a/deep");
        self.settle();
    }
}
fn data(root: &Path) -> GraphData {
    // Compiler-backed analyzers publish canonical source paths even for aliased roots.
    let root = fs::canonicalize(root).unwrap();
    GraphData {
        nodes: [
            ("Alpha", "a/deep/Shared.cs", "main", false),
            ("Beta", "a/deep/Shared.cs", "second", false),
            ("Gamma", "z.cs", "main", false),
            ("Tests", "tests.cs", "main", true),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (class, file, build, is_test))| Node {
            id: format!("method-{index}"),
            label: format!("{class}.run()"),
            container: class.into(),
            build: build.into(),
            project: "Explorer fixture".into(),
            file: root.join(file).to_string_lossy().into_owned(),
            language: "C#".into(),
            kind: "method".into(),
            is_test,
            ..Default::default()
        })
        .collect(),
        ..Default::default()
    }
}
fn focused(runtime: &Runtime<State>) -> Option<String> {
    runtime
        .elements()
        .into_iter()
        .find(|element| element.focused)
        .and_then(|element| element.key)
}

#[test]
fn single_click_is_isolated_and_large_tree_is_virtualized() {
    let mut fixture = Fixture::new((1050.0, 680.0), true, 300);
    assert!(fixture.runtime.state.explorer.rows.len() > 300);
    let rendered = fixture
        .runtime
        .elements()
        .iter()
        .filter(|element| {
            element
                .key
                .as_deref()
                .is_some_and(|key| key.starts_with("explorer-row-"))
        })
        .count();
    assert!(
        rendered < 40,
        "Rendered {rendered} of 300+ filesystem entries"
    );
    fixture
        .runtime
        .update(|_, tx| tx.focus("graph-canvas", false));
    fixture.runtime.frame(0.0);
    let navigation = serde_json::to_value(&fixture.runtime.state.nav).unwrap();
    let history = fixture.runtime.state.back.len();
    fixture.file_click(".hidden");
    fixture.assert_selected(".hidden");
    assert_eq!(
        serde_json::to_value(&fixture.runtime.state.nav).unwrap(),
        navigation
    );
    assert_eq!(fixture.runtime.state.back.len(), history);
    assert!(fixture.runtime.state.source_popup.active.is_none());
    assert_eq!(focused(&fixture.runtime).as_deref(), Some("graph-canvas"));
}

#[test]
fn graph_selection_reveals_all_files_and_reselecting_restores_highlights() {
    let mut fixture = Fixture::new((1420.0, 900.0), true, 70);
    click(&mut fixture.runtime, "graph-node-0");
    fixture.settle();
    let files = fixture.runtime.state.explorer.selected.clone();
    assert!(files.contains(&fixture.path("a/deep/Shared.cs")));
    assert!(files.contains(&fixture.path("z.cs")));
    assert!(
        fixture
            .runtime
            .state
            .explorer
            .expanded
            .contains(&fixture.path("a/deep"))
    );
    let first = fixture.key("a/deep/Shared.cs");
    let element = fixture
        .runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some(&first))
        .expect("First selected nested file is realized");
    assert!(element.visible[3] > 20.0);
    let graph_focus = focused(&fixture.runtime);
    fixture.file_click(".hidden");
    assert_eq!(focused(&fixture.runtime), graph_focus);
    fixture.assert_selected(".hidden");
    click(&mut fixture.runtime, "graph-node-0");
    fixture.settle();
    assert_eq!(fixture.runtime.state.explorer.selected, files);
    assert_eq!(focused(&fixture.runtime), graph_focus);
}

#[test]
fn double_click_frames_classes_from_both_builds_and_history_restores_navigation() {
    let mut fixture = Fixture::new((1420.0, 900.0), false, 0);
    fixture.expand();
    let original = serde_json::to_value(&fixture.runtime.state.nav).unwrap();
    let history = fixture.runtime.state.back.len();
    fixture.double_click("a/deep/Shared.cs");
    fixture.assert_file_focus("a/deep/Shared.cs");
    assert_eq!(fixture.runtime.state.back.len(), history + 1);
    assert_eq!(focused(&fixture.runtime).as_deref(), Some("graph-canvas"));
    for node in &fixture.runtime.state.graph_view.nodes {
        let top = fixture.runtime.state.nav.camera.screen(node.position);
        assert!(top[0] >= 0.0 && top[1] >= 0.0);
        assert!(
            top[0] + node.size[0] * fixture.runtime.state.nav.camera.zoom
                <= fixture.runtime.state.viewport[0] + 0.1
        );
        assert!(
            top[1] + node.size[1] * fixture.runtime.state.nav.camera.zoom
                <= fixture.runtime.state.viewport[1] + 0.1
        );
    }
    fixture.runtime.update(|state, _| state.history(false));
    fixture.settle();
    assert_eq!(
        serde_json::to_value(&fixture.runtime.state.nav).unwrap(),
        original
    );
    fixture.runtime.update(|state, _| state.history(true));
    fixture.settle();
    assert!(matches!(
        fixture.runtime.state.nav.focus_target,
        Some(FocusTarget::FileClasses(_))
    ));
}

#[test]
fn unanalyzed_and_filtered_files_preserve_graph_navigation() {
    let mut fixture = Fixture::new((1050.0, 680.0), true, 0);
    fixture
        .runtime
        .update(|state, _| state.nav.filter.include_tests = false);
    let original = serde_json::to_value(&fixture.runtime.state.nav).unwrap();
    fixture.double_click("README.txt");
    assert!(fixture.runtime.state.status.contains("No analyzed classes"));
    assert_eq!(
        serde_json::to_value(&fixture.runtime.state.nav).unwrap(),
        original
    );
    fixture.double_click("tests.cs");
    assert!(fixture.runtime.state.status.contains("filters hide"));
    assert_eq!(
        serde_json::to_value(&fixture.runtime.state.nav).unwrap(),
        original
    );
}

#[test]
fn closed_sidebar_sync_and_root_changes_do_not_publish_old_entries() {
    let mut fixture = Fixture::new((1050.0, 680.0), true, 0);
    fixture.runtime.update(|state, _| {
        state.ui.sidebar.set_open(false);
        state.select_method(0, crate::graph_interaction::SelectionAction::Inspect);
    });
    fixture.settle();
    assert!(!fixture.runtime.state.ui.sidebar.is_open());
    assert!(
        fixture
            .runtime
            .state
            .explorer
            .selected
            .contains(&fixture.path("a/deep/Shared.cs"))
    );
    let next = tempfile::tempdir().unwrap();
    fs::write(next.path().join("replacement.txt"), "new").unwrap();
    let root = next.path().to_string_lossy().into_owned();
    fixture.runtime.update(move |state, _| {
        state.explorer.refresh();
        state.root = rune::TextEdit::new(root);
        state.root_edited();
    });
    fixture.settle();
    assert_eq!(fixture.runtime.state.explorer.rows.len(), 1);
    assert_eq!(
        fixture.runtime.state.explorer.rows[0].label,
        "replacement.txt"
    );
    assert!(fixture.runtime.state.explorer.selected.is_empty());
}

#[test]
#[cfg(unix)]
fn selected_symlink_root_maps_canonical_sources_without_following_child_links() {
    let mut fixture = Fixture::new((1420.0, 900.0), true, 0);
    let _alias_directory = fixture.select_symlink_root();
    let child_link = fixture.path("child-link");
    let rows = &fixture.runtime.state.explorer.rows;
    let link = rows.iter().find(|row| row.path == child_link).unwrap();
    assert!(matches!(link.kind, crate::explorer::RowKind::Symlink));
    let child_prefix = format!("{child_link}/");
    assert!(!rows.iter().any(|row| row.path.starts_with(&child_prefix)));
    fixture.assert_selected("a/deep/Shared.cs");
    assert!(
        fixture
            .runtime
            .state
            .explorer
            .expanded
            .contains(&fixture.path("a/deep"))
    );
    fixture
        .runtime
        .update(|_, tx| tx.focus("graph-canvas", false));
    fixture.runtime.frame(0.0);
    fixture.file_click(".hidden");
    assert_eq!(focused(&fixture.runtime).as_deref(), Some("graph-canvas"));
    fixture.double_click("a/deep/Shared.cs");
    fixture.assert_file_focus("a/deep/Shared.cs");
    fixture.assert_selected("a/deep/Shared.cs");
    assert_eq!(focused(&fixture.runtime).as_deref(), Some("graph-canvas"));
}

#[test]
fn explorer_layouts_and_optional_native_captures() {
    let output =
        std::env::var_os("CODECALLNAVIGATOR_EXPLORER_CAPTURES").map(std::path::PathBuf::from);
    for (dark, size, name) in [
        (true, (1420.0, 900.0), "dark-1420.png"),
        (false, (1050.0, 680.0), "light-1050.png"),
    ] {
        let mut fixture = Fixture::new(size, dark, 8);
        fixture.expand();
        fixture.double_click("a/deep/Shared.cs");
        assert_file_cards_clear_filters(&fixture.runtime);
        for key in ["choose-folder", "explorer-refresh", "explorer-scroll"] {
            let element = fixture
                .runtime
                .elements()
                .into_iter()
                .find(|e| e.key.as_deref() == Some(key))
                .unwrap();
            assert!(
                element.visible[2] > 50.0 && element.visible[3] > 10.0,
                "{key}: {:?}",
                element.visible
            );
            assert!(element.rect[0] + element.rect[2] <= size.0);
        }
        if let Some(output) = &output {
            capture(&mut fixture.runtime, size, &output.join(name));
        }
        let resized = if size.0 > 1100.0 {
            (1050.0, 680.0)
        } else {
            (1420.0, 900.0)
        };
        fixture.runtime.set_viewport(resized, 1.0);
        fixture.settle();
        assert_file_cards_clear_filters(&fixture.runtime);
    }
}
fn assert_file_cards_clear_filters(runtime: &Runtime<State>) {
    let elements = runtime.elements();
    let filters = elements
        .iter()
        .find(|element| element.key.as_deref() == Some("graph-filters"))
        .unwrap();
    let canvas = elements
        .iter()
        .find(|element| element.key.as_deref() == Some("graph-canvas"))
        .unwrap();
    let bottom = filters.rect[1] + filters.rect[3];
    for node in &runtime.state.graph_view.nodes {
        let top = canvas.rect[1] + runtime.state.nav.camera.screen(node.position)[1];
        assert!(
            top >= bottom,
            "{} lies under graph filters: top={top}, filters bottom={bottom}",
            node.label
        );
    }
}

fn capture(runtime: &mut Runtime<State>, size: (f32, f32), path: &Path) {
    use rune::render::{Gpu, OFFSCREEN_FORMAT, Offscreen, Renderer, write_png};
    let gpu = Gpu::try_new(Gpu::instance(), None)
        .expect("Native Metal access for requested explorer evidence");
    let pixels = (size.0 as u32, size.1 as u32);
    let format = if runtime.srgb_blend() {
        OFFSCREEN_FORMAT.remove_srgb_suffix()
    } else {
        OFFSCREEN_FORMAT
    };
    let mut renderer = Renderer::new(&gpu.device, format);
    renderer.set_srgb_blend(runtime.srgb_blend());
    renderer.upload_images(&gpu.device, &gpu.queue, &mut runtime.images);
    renderer.prepare_surfaces(&gpu.device, &gpu.queue, &runtime.surfaces, pixels);
    renderer.prepare(
        &gpu.device,
        &gpu.queue,
        &mut runtime.text.atlas,
        &runtime.instances,
        pixels,
    );
    let target = Offscreen::with_format(&gpu.device, pixels.0, pixels.1, format);
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    renderer.draw(&mut encoder, &target.view, runtime.clear_color());
    target.copy(&mut encoder);
    renderer.submit(&gpu.queue, encoder);
    let rgba = target.read(&gpu.device);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    write_png(path, pixels.0, pixels.1, &rgba).unwrap();
}
