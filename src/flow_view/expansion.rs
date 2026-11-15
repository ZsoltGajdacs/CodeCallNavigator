//! Explicit controls consume CLI-owned structural identities and stop explanations.
use crate::{app::State, flow_workspace::Session};
use rune::{El, row, text};
use rune_kit::{Button, ButtonVariant, Kit};
pub(super) fn controls(state: &State, k: &Kit) -> El {
    let mode = state
        .flow_workspace
        .current
        .as_ref()
        .is_some_and(Session::expansion_mode);
    let pending = state.flow_workspace.expansion_pending();
    let mut controls = row().gap(6.0).center_y();
    if mode {
        controls = controls.child(
            Button::new("Collapse all")
                .variant(ButtonVariant::Outline)
                .on_press(State::collapse_all_flow_expansion)
                .build(k)
                .key("flow-collapse-all"),
        );
        if pending {
            controls = controls.child(
                Button::new("Cancel expansion")
                    .on_press(State::cancel_flow_expansion)
                    .build(k)
                    .key("flow-cancel-expansion"),
            );
        }
    } else {
        controls = controls.child(
            Button::new("Expand calls…")
                .variant(ButtonVariant::Outline)
                .disabled(
                    pending
                        || !state.flow_expansion_supported()
                        || state
                            .flow_workspace
                            .current
                            .as_ref()
                            .is_none_or(|session| session.ready.is_none()),
                )
                .help("Show selected callees inside this chart. Requires a compatible engine.")
                .on_press(State::start_flow_expansion)
                .build(k)
                .key("flow-expand-mode"),
        );
        if pending {
            controls = controls.child(
                Button::new("Cancelling expansion…")
                    .disabled(true)
                    .build(k)
                    .key("flow-cancel-expansion"),
            );
        }
    }
    controls
}
pub(super) fn notice(session: &Session, k: &Kit) -> El {
    let mut notice = rune::col().w_full().gap(3.0);
    if let Some(result) = session.expanded_flow() {
        notice = notice.child(
            text(format!(
                "Expanded call view · {} occurrences",
                result.occurrences.len()
            ))
            .key("flow-expansion-ready")
            .font(k.ty.small)
            .color(k.c.text_muted),
        );
    }
    if !session.message.is_empty() {
        notice = notice.child(
            text(&session.message)
                .key("flow-expansion-status")
                .font(k.ty.small)
                .color(k.c.warning.fg)
                .w_full()
                .wrap(),
        );
    }
    notice
}
pub(super) fn details(state: &State, session: &Session, k: &Kit) -> El {
    let mut content = rune::col().w_full().gap(4.0);
    let Some(result) = session.expanded_flow() else {
        return content;
    };
    for occurrence in result
        .occurrences
        .iter()
        .filter(|occurrence| occurrence.boundaries.marker == session.selected)
    {
        if let codecallnavigator::flow::OccurrenceOutcome::Stopped { message, .. } =
            &occurrence.outcome
        {
            content = content.child(
                text(message)
                    .font(k.ty.small)
                    .color(k.c.warning.fg)
                    .w_full()
                    .wrap(),
            );
        }
    }
    if let Some(action) = state.present_flow_occurrence() {
        content = content.child(
            Button::new("Collapse this occurrence")
                .variant(ButtonVariant::Outline)
                .disabled(state.flow_workspace.expansion_pending())
                .on_press(move |state: &mut State| state.apply_presented_expansion(&action))
                .build(k)
                .key("flow-collapse-occurrence"),
        );
    }
    content
}
pub(super) fn target(state: &State, call: usize, target: usize, k: &Kit) -> El {
    let Some(action) = state.present_flow_call(call, target) else {
        return rune::el();
    };
    let selected = action.is_collapse();
    let (label, key) = if selected {
        (
            "Collapse here",
            format!("flow-collapse-call-{call}-{target}"),
        )
    } else {
        ("Expand here", format!("flow-expand-{call}-{target}"))
    };
    Button::new(label).variant(ButtonVariant::Outline).disabled(state.flow_workspace.expansion_pending()).help("Expand or collapse this call and its selected descendants. Unsupported calls, cycles and size limits remain visible.").on_press(move|state:&mut State|state.apply_presented_expansion(&action)).build(k).key(key)
}
