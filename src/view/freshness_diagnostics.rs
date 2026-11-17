//! Selectable, bounded-height diagnostic details and an explicit copy action.
use crate::{app::State, freshness_workspace::diagnostics::Details};
use rune::{El, ScrollSpec, col, text};
use rune_kit::{Button, Kit};

pub(super) fn append(state: &State, k: &Kit, body: El) -> El {
    let Some(context) = &state.query_context else {
        return body;
    };
    let Some(details) = Details::from_workspace(
        &state.freshness,
        &context.descriptor,
        state
            .runtime
            .as_ref()
            .map(|runtime| runtime.engine.as_path()),
    ) else {
        return body;
    };
    let copy = details.copy_text();
    let mut entries = col().w_full().gap(12.0);
    for (index, entry) in details.entries.iter().enumerate() {
        entries = entries.child(detail(entry, k).key(format!("freshness-diagnostic-{index}")));
    }
    if details.omitted > 0 {
        entries = entries.child(
            detail(
                format!(
                    "{} additional diagnostics omitted. Use the full verification report below.",
                    details.omitted
                ),
                k,
            )
            .key("freshness-diagnostics-omitted"),
        );
    }
    body.child(detail(details.summary, k).key("freshness-diagnostics-summary"))
        .child(col().key("freshness-diagnostics-scroll").w_full().h(200.0).min_h(0.0)
            .scroll(ScrollSpec { stick_bottom: false }).child(entries))
        .child(detail(format!("Full read-only verification report:\n{}", details.command), k)
            .key("freshness-verification-command"))
        .child(detail("Retry freshness checks the inputs again; it does not rebuild the graph. Refresh reruns analysis.", k)
            .key("freshness-retry-explanation"))
        .child(Button::new("Copy details")
            .on_press_tx(move |_: &mut State, tx| tx.copy(copy.clone()))
            .build(k).key("freshness-copy-details"))
}

fn detail(value: impl Into<String>, k: &Kit) -> El {
    text(value)
        .w_full()
        .min_w(0.0)
        .font(k.ty.ui)
        .color(k.c.text_muted)
        .wrap_anywhere()
        .selectable(true)
        .selection_color(k.selection())
}
