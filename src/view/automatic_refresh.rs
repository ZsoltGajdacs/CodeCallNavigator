//! Explicit session opt-in and volatile engine activity controls.
use crate::app::State;
use rune::{El, text};
use rune_kit::{Button, Kit};

pub(super) fn append(state: &State, k: &Kit, mut body: El, mut actions: El) -> El {
    let enabled = state.query_context.as_ref().is_some_and(|context| {
        state
            .freshness
            .automatic
            .enabled(&context.descriptor.project)
    });
    actions = actions.child(
        Button::new(if enabled {
            "Disable automatic refresh"
        } else {
            "Enable automatic refresh"
        })
        .disabled(state.query_context.is_none() || state.closing)
        .on_press(State::toggle_automatic_refresh)
        .build(k)
        .key("freshness-auto-toggle"),
    );
    if enabled && let Some(activity) = state.freshness.automatic.message() {
        body = body.child(
            text(&activity)
                .help(state.freshness.automatic.details().unwrap_or(&activity))
                .w_full()
                .min_w(0.0)
                .font(k.ty.ui)
                .color(k.c.text_muted)
                .wrap()
                .key("automatic-refresh-activity"),
        );
    }
    if state.freshness.manual_pending() {
        actions = actions.child(
            Button::new("Cancel manual analysis")
                .on_press(State::cancel_analysis)
                .build(k)
                .key("freshness-manual-cancel"),
        );
    }
    if enabled && state.freshness.automatic.can_cancel() {
        actions = actions.child(
            Button::new("Cancel automatic refresh")
                .on_press(State::cancel_automatic_refresh)
                .build(k)
                .key("freshness-auto-cancel"),
        );
    }
    cleanup_warning(state, k, body).child(actions)
}

fn cleanup_warning(state: &State, k: &Kit, mut bar: El) -> El {
    if let Some(warning) = &state.freshness.cleanup_warning {
        bar = bar.child(
            text(warning)
                .w_full()
                .min_w(0.0)
                .font(k.ty.ui)
                .color(k.c.text_muted)
                .wrap()
                .key("freshness-cleanup-warning"),
        );
    }
    bar
}
