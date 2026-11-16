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
            e.text.as_deref().is_some_and(|text| {
                text.starts_with("Solid ·") || text.starts_with("Purple dashes")
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        entries.len(),
        2,
        "Both relationship legend entries must remain present"
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
