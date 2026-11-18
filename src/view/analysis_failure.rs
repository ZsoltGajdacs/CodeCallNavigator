//! Presentation shared by the empty canvas and the activity details.
use crate::{app::State, presentation::Overlay};
use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit};

pub(super) fn actions(state: &State, k: &Kit, on_canvas: bool) -> El {
    let action = if let Some(retry) = state.retry_analysis() {
        Button::new(retry.label)
            .on_press(move |state: &mut State| state.retry_without_build(&retry.target))
            .build(k)
            .key("retry-remaining-builds")
    } else {
        Button::new("Edit builds")
            .disabled(state.analysis_job_busy())
            .on_press(State::open_builds)
            .build(k)
            .key("edit-builds-after-failure")
    };
    let actions = row().gap(8.0).center().flex_wrap().child(action);
    if on_canvas {
        actions.child(
            Button::new("Analysis details")
                .variant(ButtonVariant::Outline)
                .on_press(|state: &mut State| state.ui.overlay = Some(Overlay::Activity))
                .build(k)
                .key("failed-analysis-details"),
        )
    } else {
        actions
    }
}

pub(super) fn details(state: &State, k: &Kit) -> Option<El> {
    let failure = state.feedback()?;
    let mut panel = col().w_full().min_w(0.0).gap(8.0).child(diagnostic(
        failure.summary(),
        "analysis-failure-summary",
        k,
    ));
    if state.graph.is_some() {
        panel = panel.child(actions(state, k, false));
    }
    panel = panel.child(diagnostic(
        crate::analysis_feedback::bounded(failure.diagnostic, 4096),
        "analysis-failure-diagnostic",
        k,
    ));
    if let Some(logs) = failure.target.and_then(|target| target.logs.as_ref()) {
        panel = panel.child(diagnostic(
            format!("Logs · {}", logs.display()),
            "analysis-failure-logs",
            k,
        ));
    }
    Some(panel)
}

fn diagnostic(message: String, key: &'static str, k: &Kit) -> El {
    text(message)
        .key(key)
        .w_full()
        .min_w(0.0)
        .font(k.ty.ui)
        .color(k.c.warning.fg)
        .wrap_anywhere()
}
