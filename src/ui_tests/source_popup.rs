//! Source previews exercised through the real graph cards, worker, and Rune runtime.
use super::{click, runtime};
use crate::{
    app::State,
    projection::{self, Grouping},
    scenes, source_popup,
    source_popup::Contents,
    view,
};
use codecallnavigator::graph::GraphIndex;
use rune::{Key, KeyEvent, MagnifyEvent, Mods, Runtime};
use std::{fs, sync::Arc, time::Duration};

fn frames(runtime: &mut Runtime<State>, count: usize) {
    for _ in 0..count {
        runtime.frame(1.0 / 60.0);
    }
}

fn source_runtime(line: u32) -> (Runtime<State>, tempfile::TempDir) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("Orders.cs");
    let content: String = (1..=240)
        .map(|n| format!("// line {n} {}\n", "wide source column ".repeat(30)))
        .collect();
    fs::write(&path, content).unwrap();
    let mut runtime = runtime();
    runtime.on_key = Some(view::key);
    runtime.update(|state, _| {
        let mut data = state.graph.as_ref().unwrap().data.as_ref().clone();
        for node in &mut data.nodes {
            node.file = path.to_string_lossy().into_owned();
            node.line = line;
        }
        state.graph = Some(Arc::new(GraphIndex::new(Arc::new(data)).unwrap()));
    });
    frames(&mut runtime, 2);
    (runtime, directory)
}

fn rect(runtime: &Runtime<State>, key: &str) -> [f32; 4] {
    runtime
        .elements()
        .iter()
        .find(|element| element.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("Missing element {key}"))
        .rect
}

fn hover(runtime: &mut Runtime<State>, key: &str) {
    let (x, y) = runtime.find(key).unwrap();
    runtime.pointer_move(x, y);
    frames(runtime, 1);
}

fn open(runtime: &mut Runtime<State>) {
    hover(runtime, "graph-node-0");
    frames(runtime, 27);
    assert!(runtime.state.source_popup.active.is_some());
}

fn wait_source(runtime: &mut Runtime<State>) {
    for _ in 0..200 {
        frames(runtime, 1);
        if matches!(
            runtime
                .state
                .source_popup
                .active
                .as_ref()
                .map(|p| &p.contents),
            Some(Contents::Source(_))
        ) {
            return;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!("Source worker did not deliver the temporary file");
}

#[test]
fn graph_hover_waits_then_allows_crossing_gap_and_closes_after_leaving() {
    let (mut runtime, _directory) = source_runtime(1);
    hover(&mut runtime, "graph-node-0");
    frames(&mut runtime, 20);
    assert!(
        runtime.state.source_popup.active.is_none(),
        "hover={:?} inside={} close={:?}",
        runtime
            .state
            .source_popup
            .hover
            .as_ref()
            .map(|h| &h.node.key),
        runtime.state.source_popup.inside,
        runtime.state.source_popup.close_at
    );
    frames(&mut runtime, 7);
    assert!(runtime.state.source_popup.active.is_some());
    let popup = rect(&runtime, "source-popup");
    runtime.pointer_move(10.0, 10.0);
    frames(&mut runtime, 10);
    assert!(
        runtime.state.source_popup.active.is_some(),
        "grace expired early"
    );
    runtime.pointer_move(popup[0] + 100.0, popup[1] + 20.0);
    frames(&mut runtime, 25);
    assert!(
        runtime.state.source_popup.active.is_some(),
        "popup hover did not retain preview"
    );
    runtime.pointer_move(10.0, 10.0);
    frames(&mut runtime, 30);
    assert!(
        runtime.state.source_popup.active.is_none(),
        "hover={:?} inside={} close={:?}",
        runtime
            .state
            .source_popup
            .hover
            .as_ref()
            .map(|h| &h.node.key),
        runtime.state.source_popup.inside,
        runtime.state.source_popup.close_at
    );
}

#[test]
fn pin_locks_target_and_escape_and_close_dismiss_without_graph_navigation() {
    let (mut runtime, _directory) = source_runtime(1);
    open(&mut runtime);
    click(&mut runtime, "source-pin");
    let key = runtime
        .state
        .source_popup
        .active
        .as_ref()
        .unwrap()
        .node
        .key
        .clone();
    hover(&mut runtime, "graph-node-1");
    frames(&mut runtime, 40);
    assert_eq!(
        runtime.state.source_popup.active.as_ref().unwrap().node.key,
        key
    );
    let focus = runtime.state.nav.focus.clone();
    runtime.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    frames(&mut runtime, 2);
    assert!(runtime.state.source_popup.active.is_none());
    assert_eq!(runtime.state.nav.focus, focus);
    runtime.pointer_move(10.0, 10.0);
    frames(&mut runtime, 1);
    open(&mut runtime);
    click(&mut runtime, "source-close");
    assert!(runtime.state.source_popup.active.is_none());
}

#[test]
fn method_source_reveals_recorded_line_and_scrolls_both_axes_without_zooming_graph() {
    let (mut runtime, _directory) = source_runtime(150);
    open(&mut runtime);
    wait_source(&mut runtime);
    frames(&mut runtime, 4);
    let source = rect(&runtime, "source-scroll");
    let target = rect(&runtime, "source-text-149");
    assert!(target[1] >= source[1] && target[1] < source[1] + source[3]);
    assert_eq!(
        runtime.state.source_popup.active.as_ref().unwrap().target,
        149
    );
    click(&mut runtime, "source-pin");
    let pan = runtime.state.nav.camera.pan;
    let zoom = runtime.state.nav.camera.zoom;
    runtime.wheel_xy(source[0] + 100.0, source[1] + 50.0, -180.0, 0.0);
    frames(&mut runtime, 4);
    let shifted = rect(&runtime, "source-text-149");
    assert!(shifted[0] < target[0], "horizontal source scrolling failed");
    runtime.frame(0.2);
    runtime.wheel(source[0] + 100.0, source[1] + 50.0, -100.0);
    frames(&mut runtime, 2);
    let shifted_y = rect(&runtime, "source-text-149");
    assert!(
        shifted_y[1] < shifted[1],
        "vertical source scrolling failed"
    );
    runtime.magnify_event(MagnifyEvent::new(0.4).at(source[0] + 100.0, source[1] + 50.0));
    frames(&mut runtime, 2);
    runtime.pointer_down(source[0] + 40.0, source[1] + 30.0);
    runtime.pointer_move(source[0] + 70.0, source[1] + 60.0);
    runtime.pointer_up(source[0] + 70.0, source[1] + 60.0);
    frames(&mut runtime, 2);
    assert_eq!(runtime.state.nav.camera.pan, pan);
    assert_eq!(runtime.state.nav.camera.zoom, zoom);
    assert!(runtime.state.nav.selected.is_none());
}

#[test]
fn directory_group_opens_files_and_back_returns_to_listing() {
    let (mut runtime, _directory) = source_runtime(1);
    runtime.update(|state, _| {
        state.nav.grouping = Grouping::Folder;
        state.nav.focus = None;
        state.replace_graph_view(projection::project(
            state.graph.as_ref().unwrap(),
            &state.request().unwrap(),
        ));
        scenes::position(&mut state.graph_view);
        state.nav.camera.fit(state.graph_view.size, state.viewport);
    });
    frames(&mut runtime, 3);
    open(&mut runtime);
    assert!(matches!(
        runtime.state.source_popup.active.as_ref().unwrap().contents,
        Contents::Files
    ));
    assert_eq!(
        runtime
            .state
            .source_popup
            .active
            .as_ref()
            .unwrap()
            .node
            .files
            .len(),
        1
    );
    let row_key = format!("source-file-{}-0", runtime.state.source_popup.serial);
    let (x, y) = runtime.find(&row_key).unwrap();
    runtime.pointer_move(x, y);
    click(&mut runtime, &row_key);
    wait_source(&mut runtime);
    click(&mut runtime, "source-back");
    assert!(matches!(
        runtime.state.source_popup.active.as_ref().unwrap().contents,
        Contents::Files
    ));
    let row_key = format!("source-file-{}-0", runtime.state.source_popup.serial);
    assert!(runtime.find(&row_key).is_some());
    assert!(
        runtime
            .state
            .source_popup
            .active
            .as_ref()
            .unwrap()
            .path
            .is_none()
    );
}

#[test]
fn dragging_resize_handle_is_bounded_and_window_resize_keeps_pinned_preview_visible() {
    let (mut runtime, _directory) = source_runtime(1);
    open(&mut runtime);
    click(&mut runtime, "source-pin");
    let pan = runtime.state.nav.camera.pan;
    let (x, y) = runtime.find("source-resize").unwrap();
    runtime.pointer_down(x, y);
    runtime.pointer_move(x - 600.0, y - 600.0);
    runtime.pointer_up(x - 600.0, y - 600.0);
    frames(&mut runtime, 4);
    assert_eq!(runtime.state.source_popup.size, [360.0, 240.0]);
    assert_eq!(runtime.state.nav.camera.pan, pan);
    let (x, y) = runtime.find("source-resize").unwrap();
    runtime.pointer_down(x, y);
    runtime.pointer_move(x + 2000.0, y + 2000.0);
    runtime.pointer_up(x + 2000.0, y + 2000.0);
    frames(&mut runtime, 4);
    runtime.set_viewport((1050.0, 680.0), 1.0);
    frames(&mut runtime, 60);
    let popup = rect(&runtime, "source-popup");
    assert!(popup[0] >= 16.0 && popup[1] >= 16.0);
    assert!(
        popup[0] + popup[2] <= 1034.0 && popup[1] + popup[3] <= 664.0,
        "popup bounds: {popup:?}"
    );
    assert!(runtime.state.source_popup.active.as_ref().unwrap().pinned);
}

#[test]
fn invalid_line_notice_and_stale_graph_are_local_and_navigation_clears_preview() {
    let (mut runtime, _directory) = source_runtime(999);
    open(&mut runtime);
    wait_source(&mut runtime);
    assert_eq!(
        runtime.state.source_popup.active.as_ref().unwrap().target,
        0
    );
    assert!(
        runtime
            .state
            .source_popup
            .active
            .as_ref()
            .unwrap()
            .location_warning
            .is_some()
    );
    runtime.update(|state, _| state.stale = true);
    frames(&mut runtime, 2);
    let notice = runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("source-notice"))
        .unwrap();
    assert!(notice.text.unwrap().contains("Source changed"));
    runtime.update(|state, _| state.changed());
    frames(&mut runtime, 1);
    assert!(runtime.state.source_popup.active.is_none());
}

#[test]
fn obsolete_worker_completion_cannot_replace_current_preview() {
    let (mut runtime, _directory) = source_runtime(1);
    open(&mut runtime);
    wait_source(&mut runtime);
    click(&mut runtime, "source-pin");
    let serial = runtime.state.source_popup.serial;
    runtime.update(|state, _| {
        state
            .jobs
            .source
            .request((), move |_| source_popup::SourceOutput {
                serial: serial - 1,
                result: Err("Obsolete failure".into()),
            });
    });
    for _ in 0..100 {
        frames(&mut runtime, 1);
        if !runtime.state.jobs.source.is_pending() {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(matches!(
        runtime.state.source_popup.active.as_ref().unwrap().contents,
        Contents::Source(_)
    ));
    runtime.update(|state, _| state.close_source());
    frames(&mut runtime, 2);
    assert!(runtime.state.source_popup.active.is_none());
}

#[test]
fn unpin_restores_leave_dismissal_and_pane_resizing_preserves_open_preview() {
    let (mut runtime, _directory) = source_runtime(1);
    open(&mut runtime);
    wait_source(&mut runtime);
    click(&mut runtime, "source-pin");
    let serial = runtime.state.source_popup.serial;
    runtime.update(|state, _| state.ui.sidebar.set_width(340.0));
    frames(&mut runtime, 30);
    assert_eq!(runtime.state.source_popup.serial, serial);
    assert!(matches!(
        runtime.state.source_popup.active.as_ref().unwrap().contents,
        Contents::Source(_)
    ));
    click(&mut runtime, "source-pin");
    assert!(!runtime.state.source_popup.active.as_ref().unwrap().pinned);
    runtime.pointer_move(10.0, 10.0);
    frames(&mut runtime, 30);
    assert!(runtime.state.source_popup.active.is_none());
}

#[test]
fn hover_after_idle_still_waits_the_full_open_delay() {
    let (mut runtime, _directory) = source_runtime(1);
    frames(&mut runtime, 180);
    hover(&mut runtime, "graph-node-0");
    frames(&mut runtime, 20);
    assert!(runtime.state.source_popup.active.is_none());
    frames(&mut runtime, 7);
    assert!(runtime.state.source_popup.active.is_some());
}

#[test]
fn popup_occludes_graph_hover_and_blank_presses_and_drags() {
    let (mut runtime, _directory) = source_runtime(1);
    open(&mut runtime);
    wait_source(&mut runtime);
    let popup = rect(&runtime, "source-popup");
    let point = [popup[0] + 120.0, popup[1] + popup[3] - 12.0];
    runtime.update(|state, _| {
        // Place another actual graph card beneath the popup footer, without navigation.
        let camera = &state.nav.camera;
        state.graph_view.nodes[1].position = [
            (point[0] - state.source_popup.canvas[0] - camera.pan[0]) / camera.zoom,
            (point[1] - state.source_popup.canvas[1] - camera.pan[1]) / camera.zoom,
        ];
    });
    frames(&mut runtime, 2);
    let key = runtime
        .state
        .source_popup
        .active
        .as_ref()
        .unwrap()
        .node
        .key
        .clone();
    let pan = runtime.state.nav.camera.pan;
    let serial = runtime.state.source_popup.serial;
    runtime.pointer_move(point[0], point[1]);
    frames(&mut runtime, 40);
    assert_eq!(
        runtime.state.source_popup.active.as_ref().unwrap().node.key,
        key
    );
    assert_eq!(runtime.state.source_popup.serial, serial);
    runtime.pointer_down(point[0], point[1]);
    runtime.pointer_up(point[0], point[1]);
    frames(&mut runtime, 2);
    assert!(
        runtime.state.nav.selected.is_none(),
        "blank popup press selected graph card"
    );
    runtime.pointer_down(point[0], point[1]);
    runtime.pointer_move(point[0] + 25.0, point[1] + 15.0);
    runtime.pointer_up(point[0] + 25.0, point[1] + 15.0);
    frames(&mut runtime, 2);
    assert_eq!(
        runtime.state.nav.camera.pan, pan,
        "blank popup drag panned graph"
    );
}

#[test]
fn resizing_a_window_constrained_popup_starts_from_its_visible_size() {
    let (mut runtime, _directory) = source_runtime(1);
    open(&mut runtime);
    click(&mut runtime, "source-pin");
    runtime.update(|state, _| state.source_popup.size = [1388.0, 868.0]);
    frames(&mut runtime, 60);
    runtime.set_viewport((1050.0, 680.0), 1.0);
    frames(&mut runtime, 60);
    let before = rect(&runtime, "source-popup");
    assert!((before[2] - 1018.0).abs() < 1.0);
    assert_eq!(runtime.state.source_popup.size[0], 1388.0);
    let (x, y) = runtime.find("source-resize").unwrap();
    runtime.pointer_down(x, y);
    runtime.pointer_move(x - 100.0, y);
    frames(&mut runtime, 1);
    assert!(
        (runtime.state.source_popup.size[0] - (before[2] - 100.0)).abs() < 1.0,
        "resize used preferred rather than visible width: {:?}",
        runtime.state.source_popup.size
    );
    runtime.pointer_up(x - 100.0, y);
    frames(&mut runtime, 60);
    assert!(rect(&runtime, "source-popup")[2] < before[2] - 90.0);
}

#[test]
fn moving_from_popup_overlap_to_exposed_trigger_keeps_preview_open() {
    let (mut runtime, _directory) = source_runtime(1);
    runtime.update(|state, _| {
        state.nav.camera.zoom = 1.0;
        state.nav.camera.pan = [0.0, 0.0];
        state.graph_view.nodes[0].position = [
            650.0 - state.source_popup.canvas[0],
            200.0 - state.source_popup.canvas[1],
        ];
        state.graph_view.nodes[0].size = [235.0, 68.0];
    });
    frames(&mut runtime, 60);
    open(&mut runtime);
    wait_source(&mut runtime);
    frames(&mut runtime, 20);
    let popup = rect(&runtime, "source-popup");
    let trigger = rect(&runtime, "graph-node-0");
    let overlap = [trigger[0] + 20.0, trigger[1] + 35.0];
    let exposed = [trigger[0] + trigger[2] - 20.0, trigger[1] + 35.0];
    assert!(overlap[0] < popup[0] + popup[2]);
    assert!(exposed[0] > popup[0] + popup[2]);
    runtime.pointer_move(overlap[0], overlap[1]);
    frames(&mut runtime, 4);
    assert!(runtime.state.source_popup.inside);
    assert!(runtime.state.source_popup.trigger_inside);
    runtime.pointer_move(exposed[0], exposed[1]);
    frames(&mut runtime, 30);
    assert!(!runtime.state.source_popup.inside);
    assert!(runtime.state.source_popup.trigger_inside);
    assert!(runtime.state.source_popup.active.is_some());
}

#[test]
fn virtual_source_selection_copies_code_without_gutters_and_unicode_tail_is_reachable() {
    let (mut runtime, directory) = source_runtime(150);
    let unicode_line = format!("// {}UNICODE_END", "猫😀".repeat(150));
    let expected = (1..=240)
        .map(|line| {
            if line == 150 {
                unicode_line.clone()
            } else {
                format!("// source line {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(directory.path().join("Orders.cs"), &expected).unwrap();
    let clipboard = rune::clipboard::memory();
    runtime.use_clipboard(&clipboard);
    open(&mut runtime);
    wait_source(&mut runtime);
    click(&mut runtime, "source-pin");
    runtime.update(|_, tx| tx.select_all_text("source-scroll"));
    frames(&mut runtime, 2);
    assert_eq!(runtime.selected_text().as_deref(), Some(expected.as_str()));
    runtime.key(KeyEvent {
        key: Key::Command('c'),
        mods: Mods {
            cmd: true,
            ..Mods::default()
        },
    });
    frames(&mut runtime, 2);
    assert_eq!(
        clipboard.text().as_deref().map(String::as_str),
        Some(expected.as_str())
    );
    runtime.update(|_, tx| tx.clear_text_selection());
    frames(&mut runtime, 2);
    let viewport = rect(&runtime, "source-scroll");
    let text = rect(&runtime, "source-text-149");
    let scroll_to_tail = text[0] + text[2] - (viewport[0] + viewport[2]) + 20.0;
    assert!(
        scroll_to_tail > viewport[2],
        "Unicode test line must overflow substantially"
    );
    runtime.update(|_, tx| tx.scroll_to_x("source-scroll", scroll_to_tail));
    frames(&mut runtime, 4);
    let after = rect(&runtime, "source-text-149");
    let right = after[0] + after[2];
    assert!(
        right <= viewport[0] + viewport[2] && right > viewport[0] + 20.0,
        "Unicode suffix is outside horizontal scroll range: text={after:?}, viewport={viewport:?}"
    );
}
