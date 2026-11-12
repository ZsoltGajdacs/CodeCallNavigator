//! Build selection through the production Rune view, input and overlay runtime.
use super::{click, wait_for};
use crate::{
    app::{State, TargetChoice},
    build_discovery::{Output, Phase},
    discovery::BuildTarget,
    presentation::{Drawer, Overlay},
    scenes, view,
};
use rune::{Env, Key, KeyEvent, Mods, Runtime};
use std::sync::Arc;

fn runtime(size: (f32, f32), dark: bool) -> Runtime<State> {
    let mut state = State::new();
    scenes::install(&mut state);
    state.root = rune::TextEdit::new("/repo");
    state.runtime = Some(Arc::new(crate::runtime::Runtime {
        engine: "/nonexistent/build-picker-test-engine".into(),
        resources: "/nonexistent/build-picker-test-resources".into(),
        dot: "/nonexistent/build-picker-test-dot".into(),
    }));
    let mut runtime = Runtime::new(state, view::view, size, Env::new(dark, 1.0));
    runtime.on_key = Some(view::key);
    settle(&mut runtime);
    runtime
}

fn settle(runtime: &mut Runtime<State>) {
    for _ in 0..30 {
        runtime.frame(1.0 / 60.0);
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

fn choices(count: usize) -> Vec<TargetChoice> {
    (0..count)
        .map(|index| TargetChoice {
            target: BuildTarget {
                path: format!("/repo/module-{index}/App.csproj").into(),
                kind: "project".into(),
                language: "csharp".into(),
                label: format!("module-{index} / Application {index} (C# project)"),
            },
            selected: index % 2 == 0,
        })
        .collect()
}

fn ready(runtime: &mut Runtime<State>, count: usize) {
    runtime.update(move |state, _| {
        state.jobs.discovery.cancel();
        state.build_discovery.phase = Phase::Ready;
        state.targets = choices(count);
        state.ui.overlay = Some(Overlay::Builds);
    });
    settle(runtime);
}

fn press(runtime: &mut Runtime<State>, key: Key) {
    runtime.key(KeyEvent {
        key,
        mods: Mods::default(),
    });
    settle(runtime);
}

#[test]
fn clicking_find_builds_opens_loading_and_completion_does_not_reopen_dismissed_modal() {
    let mut runtime = runtime((1050.0, 680.0), true);
    assert!(runtime.find("builds-scroll").is_none());
    click(&mut runtime, "find-builds");
    assert_eq!(runtime.state.ui.overlay, Some(Overlay::Builds));
    assert!(matches!(
        runtime.state.build_discovery.phase,
        Phase::Loading
    ));
    assert!(
        runtime
            .elements()
            .iter()
            .any(|e| e.text.as_deref() == Some("Looking for build files…"))
    );
    press(&mut runtime, Key::Escape);
    assert_eq!(runtime.state.ui.overlay, None);
    runtime.state.jobs.discovery.request((), |_| Output {
        root: "/repo".into(),
        result: Ok(vec![]),
    });
    runtime.poll = Some(crate::jobs::poll);
    wait_for(&mut runtime, |state| {
        matches!(state.build_discovery.phase, Phase::Ready)
    });
    assert_eq!(runtime.state.ui.overlay, None);
    assert!(runtime.find("build-picker").is_none());
}

#[test]
fn modal_selection_is_immediate_and_escape_returns_focus_to_button() {
    let mut runtime = runtime((1050.0, 680.0), true);
    click(&mut runtime, "find-builds");
    ready(&mut runtime, 3);
    click(&mut runtime, "build-1");
    assert!(runtime.state.targets[1].selected);
    assert_eq!(runtime.state.ui.overlay, Some(Overlay::Builds));
    press(&mut runtime, Key::Text(" ".into()));
    assert!(
        !runtime.state.targets[1].selected,
        "focused checkbox must toggle with Space"
    );
    press(&mut runtime, Key::Tab);
    press(&mut runtime, Key::Text(" ".into()));
    assert!(
        !runtime.state.targets[2].selected,
        "Tab must navigate to the next build checkbox"
    );
    press(&mut runtime, Key::Escape);
    assert_eq!(runtime.state.ui.overlay, None);
    assert!(
        runtime
            .elements()
            .iter()
            .any(|e| e.key.as_deref() == Some("find-builds") && e.focused)
    );
}

#[test]
fn close_outside_search_and_sidebar_dismiss_the_picker() {
    let mut runtime = runtime((1050.0, 680.0), true);
    ready(&mut runtime, 2);
    click(&mut runtime, "close-builds");
    assert_eq!(runtime.state.ui.overlay, None);
    ready(&mut runtime, 2);
    runtime.pointer_down(1030.0, 660.0);
    runtime.pointer_up(1030.0, 660.0);
    settle(&mut runtime);
    assert_eq!(runtime.state.ui.overlay, None);
    ready(&mut runtime, 2);
    runtime.update(|state, _| state.open_search());
    settle(&mut runtime);
    assert_eq!(runtime.state.ui.overlay, None);
    assert!(runtime.find("build-picker").is_none());
    runtime.update(|state, _| state.close_search());
    ready(&mut runtime, 2);
    runtime.update(|state, _| state.toggle_drawer(Drawer::Repository));
    settle(&mut runtime);
    assert_eq!(runtime.state.ui.overlay, None);
    assert!(runtime.find("build-picker").is_none());
}

#[test]
fn hover_summary_lists_only_selected_compact_names_and_hides_when_open() {
    let mut runtime = runtime((1050.0, 680.0), true);
    runtime.update(|state, _| state.targets.clear());
    settle(&mut runtime);
    let help = |runtime: &Runtime<State>| {
        runtime
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some("find-builds"))
            .unwrap()
            .help
    };
    assert_eq!(help(&runtime).as_deref(), Some("No builds selected"));
    runtime.update(|state, _| state.targets = choices(3));
    settle(&mut runtime);
    assert_eq!(
        help(&runtime).as_deref(),
        Some(
            "Selected builds:\nmodule-0 / Application 0 (C# project)\nmodule-2 / Application 2 (C# project)"
        )
    );
    ready(&mut runtime, 3);
    assert!(help(&runtime).is_none());
}

#[test]
fn scrollable_modal_fits_windows_and_leaves_explorer_space() {
    let output = std::env::var_os("CODECALLNAVIGATOR_BUILD_CAPTURES").map(std::path::PathBuf::from);
    for (size, dark, name) in [
        ((1420.0, 900.0), true, "builds-dark-1420.png"),
        ((1050.0, 680.0), false, "builds-light-1050.png"),
    ] {
        let mut runtime = runtime(size, dark);
        let button = rect(&runtime, "find-builds");
        let explorer = rect(&runtime, "explorer-refresh");
        assert!(
            explorer[1] < button[1] + 130.0,
            "inline builds still reserve sidebar height"
        );
        ready(&mut runtime, 60);
        let panel = rect(&runtime, "build-picker");
        assert!((panel[0] - (button[0] + button[2] + 8.0)).abs() < 1.0);
        assert!((panel[1] - button[1]).abs() < 1.0);
        assert!(panel[2] <= 421.0 && panel[3] <= 481.0, "{panel:?}");
        assert!(
            panel[0] >= 0.0
                && panel[1] >= 0.0
                && panel[0] + panel[2] <= size.0
                && panel[1] + panel[3] <= size.1
        );
        let viewport = rect(&runtime, "builds-scroll");
        let first = rect(&runtime, "build-0");
        runtime.wheel(viewport[0] + 60.0, viewport[1] + 60.0, -400.0);
        settle(&mut runtime);
        assert!(rect(&runtime, "build-0")[1] < first[1]);
        assert_eq!(rect(&runtime, "builds-scroll"), viewport);
        assert_eq!(rect(&runtime, "build-picker"), panel);
        if let Some(output) = &output {
            super::project_explorer::capture(&mut runtime, size, &output.join(name));
        }
    }
}

#[test]
fn empty_and_failure_states_are_visible_inside_modal() {
    let mut runtime = runtime((1050.0, 680.0), true);
    ready(&mut runtime, 0);
    assert!(runtime.elements().iter().any(|e| {
        e.text
            .as_deref()
            .is_some_and(|t| t.starts_with("No builds found."))
    }));
    runtime.update(|state, _| {
        state.build_discovery.phase = Phase::Failed("Cannot inspect /repo".into())
    });
    settle(&mut runtime);
    assert!(
        runtime
            .elements()
            .iter()
            .any(|e| e.key.as_deref() == Some("build-picker-message")
                && e.text.as_deref() == Some("Cannot inspect /repo"))
    );
}

#[test]
fn resized_modal_stays_within_a_constrained_window() {
    let mut runtime = runtime((1050.0, 680.0), true);
    ready(&mut runtime, 50);
    runtime.set_viewport((600.0, 360.0), 1.0);
    settle(&mut runtime);
    let panel = rect(&runtime, "build-picker");
    assert!(panel[0] >= 0.0 && panel[1] >= 0.0, "{panel:?}");
    assert!(
        panel[0] + panel[2] <= 600.0 && panel[1] + panel[3] <= 360.0,
        "{panel:?}"
    );
    assert!(rect(&runtime, "builds-scroll")[3] > 40.0);
}

#[test]
fn keyboard_navigation_reveals_offscreen_builds_and_blocks_background_shortcuts() {
    let mut runtime = runtime((1050.0, 680.0), true);
    ready(&mut runtime, 60);
    click(&mut runtime, "build-0");
    for _ in 0..25 {
        press(&mut runtime, Key::Tab);
    }
    let focused = runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("build-25"))
        .unwrap();
    assert!(focused.focused, "Tab must reach offscreen builds");
    assert!(
        focused.visible[3] >= focused.rect[3] - 1.0,
        "focused build must scroll into view: {:?} {:?}",
        focused.rect,
        focused.visible
    );
    press(&mut runtime, Key::Text(" ".into()));
    assert!(runtime.state.targets[25].selected);
    let navigation = serde_json::to_value(&runtime.state.nav).unwrap();
    runtime.state.back.push(Default::default());
    runtime.key(KeyEvent {
        key: Key::Left,
        mods: Mods {
            alt: true,
            ..Default::default()
        },
    });
    runtime.key(KeyEvent {
        key: Key::Command('f'),
        mods: Mods {
            cmd: true,
            logo: true,
            ..Default::default()
        },
    });
    settle(&mut runtime);
    assert_eq!(runtime.state.back.len(), 1);
    assert_eq!(
        serde_json::to_value(&runtime.state.nav).unwrap(),
        navigation
    );
    assert!(!runtime.state.search.open);
    assert_eq!(runtime.state.ui.overlay, Some(Overlay::Builds));
}
