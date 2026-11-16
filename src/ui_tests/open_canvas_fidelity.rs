//! Native paint evidence and focused bounds checks for the Open Canvas workspace.
//!
//! Explicit visual inputs exercise production presentation, Graphviz, real temporary
//! source files and the explorer. They do not represent analyzer success. Metal
//! captures prove rendering; they do not certify physical-window presentation.
use super::{click, project_explorer::capture};
use crate::{app::State, freshness_workspace, jobs, source_popup::Contents, view};
use codecallnavigator::{artifact::verification::InputStatus, graph::GraphIndex};
use rune::{Env, Key, KeyEvent, Mods, Runtime};
use std::{fs, path::Path, sync::Arc, time::Duration};

struct Fixture {
    runtime: Runtime<State>,
    _directory: tempfile::TempDir,
}

impl Fixture {
    fn new(size: (f32, f32), dark: bool) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("Recurring-billing");
        fs::create_dir_all(root.join("src")).unwrap();
        let source = root.join("src/ContractMigration.java");
        fs::write(
            &source,
            "class ContractMigration {\n    void submit(Request request) {\n        validate(request);\n        save(request);\n    }\n}\n",
        )
        .unwrap();
        fs::write(root.join("README.md"), "Explicit visual fixture\n").unwrap();
        let mut state = State::fixture();
        crate::traversal_scenes::methods(&mut state);
        assert!(state.warning.is_empty(), "{}", state.warning);
        state.scene = true;
        state.root = rune::TextEdit::new(root.to_string_lossy());
        let mut data = state.graph.as_ref().unwrap().data.to_graph_data();
        for node in &mut data.nodes {
            node.file = source.to_string_lossy().into_owned();
            node.line = 2;
        }
        state.graph = Some(Arc::new(GraphIndex::new(Arc::new(data)).unwrap()));
        state.status =
            "Explicit UI fixture · real files and Graphviz · no analyzer executed".into();
        state.refresh_neighbors();
        let mut runtime = Runtime::new(state, view::view, size, Env::new(dark, 1.0));
        runtime.poll = Some(jobs::poll);
        runtime.on_key = Some(view::key);
        let mut fixture = Self {
            runtime,
            _directory: directory,
        };
        fixture.settle();
        fixture
    }

    fn settle(&mut self) {
        for _ in 0..60 {
            self.runtime.frame(1.0 / 60.0);
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn escape(&mut self) {
        self.runtime.key(KeyEvent {
            key: Key::Escape,
            mods: Mods::default(),
        });
        self.settle();
    }

    fn save(&mut self, output: &Path, name: &str, size: (f32, f32)) {
        capture(&mut self.runtime, size, &output.join(format!("{name}.png")));
        let tree: Vec<_> = self
            .runtime
            .elements()
            .into_iter()
            .map(|element| {
                serde_json::json!({
                    "key": element.key,
                    "role": element.role,
                    "label": element.label,
                    "text": element.text,
                    "rect": element.rect,
                    "visible": element.visible,
                })
            })
            .collect();
        fs::write(
            output.join(format!("{name}.tree.json")),
            serde_json::to_vec_pretty(&tree).unwrap(),
        )
        .unwrap();
    }

    fn open_source(&mut self) {
        click(&mut self.runtime, "view-selected-source");
        self.wait_source();
        assert!(
            self.runtime
                .state
                .source_popup
                .active
                .as_ref()
                .unwrap()
                .pinned
        );
        self.assert_source_anchor();
    }

    fn hover_source(&mut self) {
        let selected = self.runtime.state.selected().unwrap();
        let index = self
            .runtime
            .state
            .graph_view
            .nodes
            .iter()
            .position(|node| node.member == Some(selected))
            .unwrap();
        let navigation = serde_json::to_value(&self.runtime.state.nav).unwrap();
        let history = self.runtime.state.back.len();
        let (x, y) = self.runtime.find(&format!("graph-node-{index}")).unwrap();
        self.runtime.pointer_move(x, y);
        self.wait_source();
        assert!(
            !self
                .runtime
                .state
                .source_popup
                .active
                .as_ref()
                .unwrap()
                .pinned
        );
        assert_eq!(
            serde_json::to_value(&self.runtime.state.nav).unwrap(),
            navigation
        );
        assert_eq!(self.runtime.state.back.len(), history);
        self.assert_source_anchor();
    }

    fn assert_source_anchor(&self) {
        let popup = rect(&self.runtime, "source-popup");
        let canvas = rect(&self.runtime, "graph-canvas");
        assert!(
            popup[1] >= canvas[1] - 0.5,
            "Preview covers toolbar: {popup:?}"
        );
        assert_inside(
            popup,
            [
                0.0,
                0.0,
                self.runtime.state.source_popup.window[0],
                self.runtime.state.source_popup.window[1],
            ],
            "Source preview",
        );
        match &self
            .runtime
            .state
            .source_popup
            .active
            .as_ref()
            .unwrap()
            .contents
        {
            Contents::Source(source) => assert!(source.text.contains("class ContractMigration")),
            _ => panic!("Preview must contain the actual temporary Java source"),
        }
    }

    fn wait_source(&mut self) {
        for _ in 0..200 {
            self.runtime.frame(1.0 / 60.0);
            if matches!(
                self.runtime
                    .state
                    .source_popup
                    .active
                    .as_ref()
                    .map(|p| &p.contents),
                Some(Contents::Source(_))
            ) {
                self.settle();
                return;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        panic!("The production source loader did not read the temporary Java file");
    }
}

fn rect(runtime: &Runtime<State>, key: &str) -> [f32; 4] {
    runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("Missing {key}"))
        .rect
}

fn assert_inside(inner: [f32; 4], outer: [f32; 4], description: &str) {
    assert!(
        inner[2] > 0.0
            && inner[3] > 0.0
            && inner[0] >= outer[0] - 0.5
            && inner[1] >= outer[1] - 0.5
            && inner[0] + inner[2] <= outer[0] + outer[2] + 0.5
            && inner[1] + inner[3] <= outer[1] + outer[3] + 0.5,
        "{description}: {inner:?} outside {outer:?}"
    );
}

fn assert_workspace_bounds(runtime: &Runtime<State>, size: (f32, f32)) {
    let window = [0.0, 0.0, size.0, size.1];
    for key in [
        "workspace-action-band",
        "project-picker-trigger",
        "open-search",
        "graph-filters",
        "toggle-inspector",
        "status-bar",
        "graph-footer",
    ] {
        assert_inside(rect(runtime, key), window, key);
    }
    let activity = rect(runtime, "status-bar");
    let controls = rect(runtime, "graph-footer");
    assert!(
        activity[0] + activity[2] <= controls[0] + 0.5
            || controls[0] + controls[2] <= activity[0] + 0.5
            || activity[1] + activity[3] <= controls[1] + 0.5
            || controls[1] + controls[3] <= activity[1] + 0.5,
        "Floating surfaces overlap: {activity:?}, {controls:?}"
    );
    let inspector = rect(runtime, "inspector-scroll");
    let source = rect(runtime, "view-selected-source");
    let flow = rect(runtime, "open-flowchart");
    assert_inside(source, inspector, "View source");
    assert_inside(flow, inspector, "Open flowchart");
    assert!(
        (source[1] - flow[1]).abs() < 0.5,
        "Primary source/flow actions must share a row: {source:?}, {flow:?}"
    );
    assert_inside(
        rect(runtime, "selected-source-location"),
        inspector,
        "Source location",
    );
}

fn assert_activity_title(runtime: &Runtime<State>, expected: &str) {
    let title = runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("activity-title"))
        .and_then(|e| e.text);
    assert_eq!(title.as_deref(), Some(expected));
}

#[test]
fn long_source_title_keeps_pin_and_close_inside_minimum_preview() {
    let mut runtime = super::runtime();
    runtime.state.source_popup.size = [360.0, 240.0];
    runtime.state.source_popup.active = Some(crate::source_popup::Preview {
        node: crate::projection::ViewNode {
            label: "veryLongSourceMethodNameWithManyParameters".repeat(12),
            ..runtime.state.graph_view.nodes[0].clone()
        },
        contents: Contents::Loading,
        path: Some("/temporary/VeryLongSourceFilename.java".into()),
        requested_line: 1,
        target: 0,
        location_warning: None,
        pinned: true,
    });
    runtime.invalidate();
    for _ in 0..10 {
        runtime.frame(1.0 / 60.0);
    }
    let popup = rect(&runtime, "source-popup");
    for key in ["source-pin", "source-close"] {
        assert_inside(rect(&runtime, key), popup, key);
    }
    click(&mut runtime, "source-close");
    assert!(runtime.state.source_popup.active.is_none());
}

/// Risk: clipping, compositing and pixel output cannot be proved by view-tree tests.
/// This opt-in journey uses the actual Metal renderer with real filesystem/source
/// boundaries, while explicitly remaining offscreen integration evidence.
#[test]
#[ignore = "requires Metal; CODECALLNAVIGATOR_FIDELITY_CAPTURES optionally retains captures"]
fn native_metal_workspace_fidelity_captures() {
    let temporary = tempfile::tempdir().unwrap();
    let output = std::env::var_os("CODECALLNAVIGATOR_FIDELITY_CAPTURES")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| temporary.path().to_owned());
    fs::create_dir_all(&output).unwrap();
    for (dark, size, name) in [
        (false, (1420.0, 900.0), "light-1420"),
        (true, (1420.0, 900.0), "dark-1420"),
        (false, (1050.0, 680.0), "light-1050"),
        (true, (1050.0, 680.0), "dark-1050"),
        (false, (1128.0, 952.0), "light-reference-1128"),
    ] {
        let mut fixture = Fixture::new(size, dark);
        assert_workspace_bounds(&fixture.runtime, size);
        assert_activity_title(&fixture.runtime, "Graph ready");
        assert!(fixture.runtime.elements().iter().any(|e| {
            e.key
                .as_deref()
                .is_some_and(|key| key.starts_with("explorer-icon-"))
        }));
        fixture.save(&output, name, size);
        click(&mut fixture.runtime, "graph-filters");
        fixture.settle();
        fixture.save(&output, &format!("{name}-filters"), size);
        fixture.escape();
        fixture.hover_source();
        fixture.save(&output, &format!("{name}-source"), size);
        fixture.escape();
        fixture.open_source();
        fixture.save(&output, &format!("{name}-pinned-source"), size);
        fixture.escape();
        capture_activity_states(&mut fixture, &output, name, size);
    }
}

fn capture_activity_states(fixture: &mut Fixture, output: &Path, name: &str, size: (f32, f32)) {
    fixture.runtime.update(|state, _| {
        state.analyzing = true;
        state.status = format!(
            "Compiling Java source sets for {}",
            "long-module-name/".repeat(18)
        );
    });
    fixture.settle();
    assert_workspace_bounds(&fixture.runtime, size);
    assert_activity_title(&fixture.runtime, "Updating graph");
    assert_inside(
        rect(&fixture.runtime, "activity-cancel"),
        rect(&fixture.runtime, "status-bar"),
        "Cancel analysis",
    );
    fixture.save(output, &format!("{name}-busy-long-message"), size);
    click(&mut fixture.runtime, "activity-details");
    fixture.settle();
    assert_eq!(
        fixture.runtime.state.ui.overlay,
        Some(crate::presentation::Overlay::Activity)
    );
    fixture.save(output, &format!("{name}-busy-details"), size);
    fixture.escape();
    fixture.runtime.update(|state, _| {
        state.analyzing = false;
        state.warning = "Analysis failed · the previous successful graph is preserved".into();
    });
    fixture.settle();
    assert_activity_title(&fixture.runtime, "Graph needs attention");
    fixture.save(output, &format!("{name}-failure"), size);
    fixture.runtime.update(|state, _| {
        state.warning.clear();
        state.freshness.status = freshness_workspace::Status::Observed(InputStatus::Stale);
    });
    fixture.settle();
    assert_activity_title(&fixture.runtime, "Graph out of date");
    fixture.save(output, &format!("{name}-stale"), size);
    click(&mut fixture.runtime, "activity-details");
    fixture.settle();
    assert_eq!(
        fixture.runtime.state.ui.overlay,
        Some(crate::presentation::Overlay::Activity)
    );
    fixture.save(output, &format!("{name}-details"), size);
}
