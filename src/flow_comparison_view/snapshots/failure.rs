//! Readable and copyable diagnostics for a failed snapshot preparation.
use crate::{analysis_feedback::phase_label, app::State, engine_client::AnalysisFailure};
use rune::{El, ScrollSpec, col, text};
use rune_kit::{Button, Kit};

pub(super) fn details(failure: &AnalysisFailure, k: &Kit) -> El {
    let diagnostic = diagnostic_text(failure);
    let copy = diagnostic.clone();
    col()
        .w_full()
        .gap(4.0)
        .child(
            col()
                .key("comparison-snapshot-diagnostics")
                .w_full()
                .max_h(144.0)
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(
                    text(diagnostic)
                        .key("comparison-snapshot-diagnostic-text")
                        .w_full()
                        .min_w(0.0)
                        .font(k.ty.small)
                        .color(k.c.text_base)
                        .wrap_anywhere()
                        .selectable(true)
                        .selection_color(k.selection()),
                ),
        )
        .child(
            Button::new("Copy error details")
                .on_press_tx(move |_: &mut State, tx| tx.copy(copy.clone()))
                .build(k)
                .key("comparison-snapshot-copy-error"),
        )
}

fn diagnostic_text(failure: &AnalysisFailure) -> String {
    let Some(target) = &failure.target else {
        return failure.message.clone();
    };
    let phase = phase_label(target.phase);
    let path = target.target.path.display();
    let kind = &target.target.kind;
    let mut text = format!("{phase} failed for {path} ({kind})\n\n{}", failure.message);
    if let Some(logs) = target.logs.as_ref().filter(|logs| logs.is_absolute()) {
        text.push_str(&format!("\n\nBuild logs · {}", logs.display()));
    }
    text
}
