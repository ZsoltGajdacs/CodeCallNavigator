//! Selected step diagnostics and compiler-attributed call navigation.
use crate::{app::State, flow_workspace::Session};
use rune::{El, col, text};
use rune_kit::{Button, ButtonVariant, Kit};
pub(super) fn view(state: &State, session: &Session, k: &Kit) -> El {
    let mut content = col()
        .key("flow-details")
        .motion(rune::Spring::NONE)
        .h(
            if session.selected.is_some()
                || session
                    .ready
                    .as_ref()
                    .is_some_and(|r| !r.chart.diagnostics.is_empty())
            {
                160.0
            } else {
                36.0
            },
        )
        .w_full()
        .padding(8.0)
        .gap(4.0)
        .max_h(190.0)
        .scroll(rune::ScrollSpec {
            stick_bottom: false,
        });
    let Some(ready) = &session.ready else {
        return content;
    };
    for diagnostic in &ready.chart.diagnostics {
        content = content.child(
            text(diagnostic)
                .font(k.ty.small)
                .color(k.c.warning.fg)
                .w_full()
                .wrap(),
        );
    }
    let Some(step) = session.selected.and_then(|i| ready.chart.steps.get(i)) else {
        return content.child(
            text("Select a step to highlight its source and inspect call targets.")
                .font(k.ty.small)
                .color(k.c.text_muted),
        );
    };
    content = content.child(
        text(&step.label)
            .font(k.ty.small_medium)
            .color(k.c.text_base)
            .w_full()
            .wrap(),
    );
    for (index, call) in step.calls.iter().enumerate() {
        content = content.child(call_details(state, call, index, k));
    }
    content
}

fn target_name(state: &State, symbol: &str) -> String {
    state
        .graph
        .as_ref()
        .and_then(|graph| {
            graph
                .node_index(symbol)
                .map(|i| graph.data.nodes[i as usize].signature.clone())
        })
        .unwrap_or_else(|| symbol.to_owned())
}

fn call_details(state: &State, call: &codegraph::flow::CallSite, call_index: usize, k: &Kit) -> El {
    let mut content = col().w_full().gap(4.0);
    content = content.child(
        text(&call.label)
            .font(k.ty.small)
            .color(k.c.text_muted)
            .w_full()
            .wrap(),
    );
    if let Some(diagnostic) = &call.diagnostic {
        content = content.child(
            text(diagnostic)
                .font(k.ty.small)
                .color(k.c.warning.fg)
                .w_full()
                .wrap(),
        );
    }
    for (target_index, target) in call.targets.iter().enumerate() {
        let symbol = target.symbol.clone();
        content = content.child(
            Button::new(format!(
                "{} · {}",
                target_name(state, &target.symbol),
                if target.kind == "call" {
                    target.certainty.clone()
                } else {
                    format!("{} · {}", target.certainty, target.kind)
                }
            ))
            .variant(ButtonVariant::Outline)
            .help(format!("{}\n{}", target.symbol, target.provenance))
            .on_press(move |s: &mut State| s.open_flow(symbol.clone()))
            .build(k)
            .key(format!("flow-target-{call_index}-{target_index}")),
        );
    }
    content
}
