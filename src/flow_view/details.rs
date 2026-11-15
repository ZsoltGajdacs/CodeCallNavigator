//! Selected step diagnostics and compiler-attributed call navigation.
use super::address::Address;
use crate::{app::State, flow_workspace::Session};
use rune::{El, col, text};
use rune_kit::{Button, ButtonVariant, Kit};
pub(crate) fn view(state: &State, session: &Session, address: Address, k: &Kit) -> El {
    let mut content = col()
        .key(address.key("details"))
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
    content = content.child(selected_label(session, step, address, k));
    if address == Address::Single && session.expanded_flow().is_some() {
        content = content.child(super::expansion::details(state, session, k));
    }
    for (index, call) in step.calls.iter().enumerate() {
        content = content.child(call_details(state, Call { site: call, index }, address, k));
    }
    content
}

fn selected_label(
    session: &Session,
    step: &codecallnavigator::flow::FlowStep,
    address: Address,
    k: &Kit,
) -> El {
    let mut content = col().w_full().gap(4.0);
    content = content.child(
        text(&step.label)
            .font(k.ty.small_medium)
            .color(k.c.text_base)
            .w_full()
            .wrap(),
    );
    if session.selected_source_span().is_none() {
        content = content.child(
            text("Derived or unlocated step: no source location is available. Source position is unchanged.")
                .key(address.key("source-unlocated"))
                .font(k.ty.small)
                .color(k.c.text_muted)
                .w_full()
                .wrap(),
        );
    }
    content
}

struct Call<'a> {
    site: &'a codecallnavigator::flow::CallSite,
    index: usize,
}
fn call_details(state: &State, selected: Call<'_>, address: Address, k: &Kit) -> El {
    let Call {
        site: call,
        index: call_index,
    } = selected;
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
        content = content.child(target_button(
            state,
            Target {
                call_index,
                target_index,
                target,
            },
            address,
            k,
        ));
        if address == Address::Single
            && state
                .flow_workspace
                .current
                .as_ref()
                .is_some_and(|session| session.expanded_flow().is_some())
        {
            content = content.child(super::expansion::target(state, call_index, target_index, k));
        }
    }
    content
}

struct Target<'a> {
    call_index: usize,
    target_index: usize,
    target: &'a codecallnavigator::flow::CallTarget,
}
fn target_button(state: &State, selected: Target<'_>, address: Address, k: &Kit) -> El {
    let Target {
        call_index,
        target_index,
        target,
    } = selected;
    let symbol = target.symbol.clone();
    let summary = address.target(state, &symbol);
    let mut help = format!("{}\n{}", target.symbol, target.provenance);
    let explanation = address.target_help(&summary);
    if !explanation.is_empty() {
        help.push('\n');
        help.push_str(explanation);
    }
    let label = format!(
        "{} · {}",
        summary.name,
        if target.kind == "call" {
            target.certainty.clone()
        } else {
            format!("{} · {}", target.certainty, target.kind)
        }
    );
    Button::new(label.clone())
        .label_with(move |font, ink| {
            text(label)
                .key("label")
                .font(font)
                .color(ink)
                .w_full()
                .truncate_middle()
        })
        .variant(ButtonVariant::Outline)
        .help(help)
        .disabled(address.target_disabled(&summary))
        .on_press(move |state: &mut State| address.follow(state, symbol.clone()))
        .build(k)
        .w_full()
        .key(address.key(&format!("target-{call_index}-{target_index}")))
}
