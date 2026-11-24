//! Both revision pickers render the shared local commit history.
use crate::{app::State, flow_comparison::SnapshotSide};
use rune::{El, ScrollSpec, col, el, row, text};
use rune_kit::{Button, ButtonVariant, Kit};

pub(super) fn picker_button(state: &State, side: SnapshotSide, k: &Kit) -> El {
    let id = selected(state, side);
    let label = match id {
        Some("WORKTREE") => "Worktree".to_owned(),
        Some(id) => state
            .flow_comparison
            .revisions
            .rows
            .iter()
            .find(|commit| commit.id == id && !commit.subject.is_empty())
            .map_or_else(
                || short_id(id).to_owned(),
                |commit| format!("{} — {}", short_id(id), commit.subject),
            ),
        None => "Choose commit".to_owned(),
    };
    Button::new(label)
        .icon(&rune_kit::glyphs::CHEVRON_DOWN)
        .variant(ButtonVariant::Outline)
        .on_press(move |state: &mut State| state.flow_comparison.show_revision_picker(side))
        .build(k)
        .w_full()
        .min_w(0.0)
        .on_key(move |state: &mut State, event, _| {
            if event.key != rune::Key::Down {
                return false;
            }
            state.flow_comparison.show_revision_picker(side);
            true
        })
        .key(match side {
            SnapshotSide::Base => "comparison-base",
            SnapshotSide::Head => "comparison-head",
        })
        .expanded(state.flow_comparison.revisions.picker == Some(side))
}

fn selected(state: &State, side: SnapshotSide) -> Option<&str> {
    match side {
        SnapshotSide::Base => state.flow_comparison.revisions.base(),
        SnapshotSide::Head => Some(state.flow_comparison.revisions.after()),
    }
}

fn short_id(id: &str) -> &str {
    id.get(..10).unwrap_or(id)
}

pub(super) fn layers(state: &State, k: &Kit) -> Vec<El> {
    let Some(side) = state.flow_comparison.revisions.picker else {
        return Vec::new();
    };
    let panel = col()
        .key("comparison-revisions")
        .label(format!("{} revision", side.label()))
        .role("dialog")
        .w(640.0_f32.min((state.source_popup.interaction.window[0] - 24.0).max(320.0)))
        .h(430.0)
        .min_h(0.0)
        .padding(12.0)
        .gap(8.0)
        .radius(10.0)
        .border(1.0, k.c.std_border)
        .fill(k.c.layer_01)
        .child(header(side, k));
    let panel = if side == SnapshotSide::Head {
        panel.child(choice(
            side,
            selected(state, side) == Some("WORKTREE"),
            ("WORKTREE", "Worktree · Uncommitted changes".into()),
            k,
        ))
    } else {
        panel
    };
    let panel = panel
        .child(history(state, side, k))
        .child(message(state, k))
        .focus_return(match side {
            SnapshotSide::Base => "comparison-base",
            SnapshotSide::Head => "comparison-head",
        })
        .modal(true, close_picker)
        .modal_click_outside(true);
    vec![
        el().absolute(0.0, 0.0)
            .w_full()
            .h_full()
            .fill(k.c.scrim)
            .a11y_hidden(),
        col()
            .absolute(0.0, 0.0)
            .w_full()
            .h_full()
            .pt(105.0)
            .items_center()
            .child(panel),
    ]
}

fn header(side: SnapshotSide, k: &Kit) -> El {
    row()
        .center_y()
        .gap(8.0)
        .child(
            text(format!("{} revision", side.label()))
                .font(k.ty.ui_medium)
                .grow(1.0),
        )
        .child(
            Button::new("Done")
                .on_press(close_picker)
                .build(k)
                .on_key(|state: &mut State, event, _| {
                    if event.key != rune::Key::Enter {
                        return false;
                    }
                    close_picker(state);
                    true
                })
                .key("comparison-revisions-done"),
        )
}

fn close_picker(state: &mut State) {
    state.flow_comparison.revisions.close_picker();
}

fn history(state: &State, side: SnapshotSide, k: &Kit) -> El {
    let revisions = &state.flow_comparison.revisions;
    let mut list = col()
        .key("comparison-revision-history")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .gap(3.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        });

    for commit in &revisions.rows {
        let subject = if commit.subject.is_empty() {
            "(No commit message)"
        } else {
            &commit.subject
        };
        list = list.child(choice(
            side,
            selected(state, side) == Some(commit.id.as_str()),
            (&commit.id, format!("{}  {}", short_id(&commit.id), subject)),
            k,
        ));
    }
    if let Some(error) = &revisions.error {
        list = list
            .child(text(error).font(k.ty.small).truncate().help(error))
            .child(
                Button::new("Retry")
                    .on_press(|state: &mut State| state.flow_comparison.more_comparison_commits())
                    .build(k),
            );
    } else if revisions.loading() {
        list = list.child(text("Loading commits…").font(k.ty.small));
    } else if revisions.rows.is_empty() {
        list = list.child(text("No commits in this repository").font(k.ty.small));
    } else if revisions.more.is_some() {
        list = list.child(
            Button::new("Load older commits")
                .on_press(|state: &mut State| state.flow_comparison.more_comparison_commits())
                .build(k)
                .key("comparison-revisions-more"),
        );
    }
    list
}

fn choice(side: SnapshotSide, selected: bool, row: (&str, String), k: &Kit) -> El {
    let (id, label) = row;
    let key = format!(
        "comparison-revision-{}",
        if id == "WORKTREE" { "worktree" } else { id }
    );
    let id = id.to_owned();
    let key_id = id.clone();
    Button::new(label)
        .variant(if selected {
            ButtonVariant::Outline
        } else {
            ButtonVariant::Ghost
        })
        .on_press(move |state: &mut State| state.choose_comparison_revision(side, &id))
        .build(k)
        .on_key(move |state: &mut State, event, _| {
            if event.key != rune::Key::Enter {
                return false;
            }
            state.choose_comparison_revision(side, &key_id);
            true
        })
        .key(key)
        .w_full()
        .min_w(0.0)
        .checked(selected)
}

fn message(state: &State, k: &Kit) -> El {
    let revisions = &state.flow_comparison.revisions;
    let message = if revisions.reading_message() {
        "Loading commit message…"
    } else {
        revisions.message.as_deref().unwrap_or("")
    };
    col()
        .key("comparison-revision-message")
        .selection_group(true)
        .w_full()
        .h(100.0)
        .padding(6.0)
        .fill(k.c.bg_base)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(
            text(message)
                .font(k.ty.small)
                .color(k.c.text_base)
                .w_full()
                .wrap_anywhere(),
        )
}
