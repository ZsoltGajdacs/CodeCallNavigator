//! Real Rune layout catches width allocation and clipping that string/model
//! tests cannot. This explicit presentation fixture never runs an analyzer.
use super::{click, project_explorer::capture};
use crate::{
    app::State,
    engine_client::{Descriptor, QueryContext},
    freshness_workspace, scenes, view,
};
use codecallnavigator::{
    artifact::verification::InputStatus,
    query::Error,
    refresh::{Activity, Phase},
};
use rune::{Env, Runtime};

const OWNERSHIP_WARNING: &str = "Compiler-output ownership is incomplete or unsupported; use manual refresh. Recorded input: /workspace/recurring-billing/spec/target/classes. The saved input scope cannot safely distinguish compiler outputs from other consumed inputs until the project is refreshed manually.";

fn details_runtime(size: (f32, f32)) -> Runtime<State> {
    let mut state = State::fixture();
    scenes::install(&mut state);
    let descriptor = Descriptor {
        project: "/explicit-layout-fixture".into(),
        ..Default::default()
    };
    let backend = crate::runtime::Runtime {
        engine: "/unused".into(),
        resources: "/".into(),
        dot: "/unused".into(),
    };
    state.query_context = Some(QueryContext::new(&backend, descriptor.clone()));
    state.freshness.automatic.toggle(&descriptor);
    state.freshness.automatic.activity = Some(Activity {
        version: 1,
        runs_started: 0,
        phase: Phase::Unavailable {
            error: Error::new("selection_unavailable", OWNERSHIP_WARNING),
        },
    });
    state.freshness.accept(freshness_workspace::fixture(
        InputStatus::Stale,
        &"a".repeat(64),
    ));
    let mut runtime = Runtime::new(state, view::view, size, Env::new(true, 1.0));
    settle(&mut runtime);
    click(&mut runtime, "activity-details");
    settle(&mut runtime);
    runtime
}

fn settle(runtime: &mut Runtime<State>) {
    for _ in 0..60 {
        runtime.frame(1.0 / 60.0);
    }
}

fn element(runtime: &Runtime<State>, key: &str) -> rune::ElementInfo {
    runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("Missing {key}"))
}

fn reveal(runtime: &mut Runtime<State>, key: &'static str) {
    runtime.update(move |_, tx| tx.scroll_into_view_in("scroll", key));
    settle(runtime);
}

fn assert_full_width(runtime: &Runtime<State>, key: &str) {
    let text = element(runtime, key);
    let body = element(runtime, "freshness-status");
    assert!(
        text.rect[2] >= body.rect[2] - 1.0,
        "{key} squeezed beside controls: {:?}, body {:?}",
        text.rect,
        body.rect
    );
    assert!(
        text.visible[2] >= text.rect[2] - 1.0 && text.visible[3] >= text.rect[3] - 1.0,
        "{key} clipped: {:?}, visible {:?}",
        text.rect,
        text.visible
    );
}

fn assert_legend_fits(runtime: &Runtime<State>) {
    let legend = element(runtime, "relationship-legend");
    let entries = runtime
        .elements()
        .into_iter()
        .filter(|e| {
            e.key
                .as_deref()
                .is_some_and(|key| key.starts_with("legend-"))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        entries.len(),
        4,
        "All relationship legend entries must remain present"
    );
    for entry in entries {
        assert!(
            entry.rect[2] <= legend.rect[2] + 0.5,
            "Legend extends outside detail surface: {:?}, legend {:?}",
            entry.rect,
            legend.rect
        );
    }
}

#[test]
fn analysis_detail_warnings_use_full_width_above_actions() {
    for size in [(600.0, 800.0), (1050.0, 680.0)] {
        let mut runtime = details_runtime(size);
        reveal(&mut runtime, "automatic-refresh-activity");
        assert_full_width(&runtime, "automatic-refresh-activity");
        let activity = element(&runtime, "automatic-refresh-activity");
        let toggle = element(&runtime, "freshness-auto-toggle");
        assert!(
            activity
                .text
                .as_deref()
                .unwrap()
                .ends_with(OWNERSHIP_WARNING)
        );
        assert!(
            activity.rect[3] > 40.0,
            "Long warning must render more than two lines: {:?}",
            activity.rect
        );
        assert_legend_fits(&runtime);
        assert!(
            toggle.rect[1] >= activity.rect[1] + activity.rect[3] - 0.5,
            "Actions must follow complete warning text"
        );
        reveal(&mut runtime, "freshness-message");
        assert_full_width(&runtime, "freshness-message");
        for key in ["freshness-retry", "freshness-auto-toggle"] {
            reveal(&mut runtime, key);
            let action = element(&runtime, key);
            assert!(
                action.visible[2] >= action.rect[2] - 1.0
                    && action.visible[3] >= action.rect[3] - 1.0,
                "{key} clipped"
            );
            assert!(runtime.find(key).is_some(), "{key} must be hit-testable");
        }
        if let Some(output) = std::env::var_os("CODECALLNAVIGATOR_ACTIVITY_UI_CAPTURES") {
            reveal(&mut runtime, "automatic-refresh-activity");
            capture(
                &mut runtime,
                size,
                &std::path::PathBuf::from(output).join(format!("activity-warning-{}.png", size.0)),
            );
        }
        click(&mut runtime, "close");
        assert!(runtime.state.ui.overlay.is_none());
    }
}

#[test]
fn freshness_blocker_details_scroll_and_copy_complete_context() {
    let mut runtime = details_runtime((600.0, 800.0));
    let clipboard = rune::clipboard::memory();
    runtime.use_clipboard(&clipboard);
    runtime.update(|state, _| {
        let mut observation = freshness_workspace::fixture(InputStatus::Stale, &"a".repeat(64));
        observation.coverage.complete = false;
        for index in 0..8 {
            observation
                .reasons
                .push(codecallnavigator::artifact::verification::Reason::new(
                    "unreadable_input",
                    Some(format!(
                        "@repo/required/{}/Input{index}.java",
                        "long-directory/".repeat(8)
                    )),
                ));
        }
        observation.reasons_total = 11;
        observation.omitted_reasons = 3;
        state.freshness.accept(observation);
    });
    settle(&mut runtime);
    reveal(&mut runtime, "freshness-diagnostics-scroll");
    let viewport = element(&runtime, "freshness-diagnostics-scroll");
    assert!(
        viewport.rect[3] <= 201.0,
        "Diagnostics must have a bounded viewport"
    );
    let first = element(&runtime, "freshness-diagnostic-0");
    assert!(
        first.rect[2] <= viewport.rect[2] + 1.0,
        "Long paths must wrap within the viewport"
    );
    capture_blockers(&mut runtime, "freshness-blockers-first.png");
    runtime.update(|_, tx| {
        tx.scroll_into_view_in("freshness-diagnostics-scroll", "freshness-diagnostic-7");
    });
    settle(&mut runtime);
    let tail = element(&runtime, "freshness-diagnostic-7");
    assert!(
        tail.visible[3] > 20.0,
        "The last required-input failure must be reachable by scrolling"
    );
    capture_blockers(&mut runtime, "freshness-blockers-tail.png");
    reveal(&mut runtime, "freshness-copy-details");
    assert!(runtime.find("freshness-copy-details").is_some());
    click(&mut runtime, "freshness-copy-details");
    settle(&mut runtime);
    let copied = clipboard.text().unwrap();
    assert!(copied.contains("Input0.java"));
    assert!(
        copied.contains("Input7.java"),
        "Copy must include details below the viewport"
    );
    assert!(copied.contains("3 additional diagnostics omitted"));
    assert!(copied.contains("ccnavigator verify --project '/explicit-layout-fixture'"));
    assert!(copied.contains("does not rebuild the graph"));
    assert!(
        copied.contains("permissions"),
        "Recovery instructions must accompany the path"
    );
    assert!(
        copied.contains(OWNERSHIP_WARNING),
        "Coordinator failures must remain available"
    );
    capture_blockers(&mut runtime, "freshness-blockers-actions.png");
}

fn capture_blockers(runtime: &mut Runtime<State>, filename: &str) {
    if let Some(output) = std::env::var_os("CODECALLNAVIGATOR_ACTIVITY_UI_CAPTURES") {
        capture(
            runtime,
            (600.0, 800.0),
            &std::path::PathBuf::from(output).join(filename),
        );
    }
}
