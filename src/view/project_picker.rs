//! Project selection presentation; discovery owns filesystem validation and failures.
use super::{build_picker, workspace_toolbar};
use crate::{app::State, presentation::Overlay, theme};
use rune::{Align, El, Side, col, el, icon, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit, TextInput, glyphs, popover};

pub(super) fn button(state: &State, k: &Kit) -> El {
    let path = state
        .project_open
        .folder
        .as_deref()
        .or_else(|| {
            state
                .query_context
                .as_ref()
                .and_then(|context| context.descriptor.project.to_str())
        })
        .unwrap_or_else(|| {
            if state.uses_fixture_queries() {
                state
                    .graph
                    .as_ref()
                    .and_then(|graph| graph.data.roots.first())
                    .map_or(state.root.text.trim(), String::as_str)
            } else {
                ""
            }
        });
    let title = std::path::Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Open project…");
    let visible_title = title.to_owned();
    Button::new(title)
        .icon(&glyphs::FOLDER)
        .control_size(ControlSize::ExtraLarge)
        .label_with(move |font, ink| {
            row()
                .center_y()
                .gap(12.0)
                .child(
                    text(visible_title)
                        .key("workspace-identity")
                        .font(font)
                        .color(ink)
                        .max_w(160.0)
                        .truncate(),
                )
                .child(icon(&glyphs::CHEVRON_DOWN, 14.0).color(ink))
        })
        .variant(ButtonVariant::Outline)
        .expanded(state.ui.overlay == Some(Overlay::Projects))
        .on_press(|state: &mut State| {
            state.ui.overlay = if state.ui.overlay == Some(Overlay::Projects) {
                None
            } else {
                Some(Overlay::Projects)
            };
        })
        .build(k)
        .key("project-picker-trigger")
        .help(path)
}

pub(super) fn layer(state: &State, k: &Kit, layers: &mut Vec<El>) {
    if state.ui.overlay != Some(Overlay::Projects) {
        return;
    }
    let mut contents = col()
        .w(400.0_f32.min((state.source_popup.interaction.window[0] - 24.0).max(1.0)))
        .padding(16.0)
        .max_h((state.source_popup.interaction.window[1] - 100.0).max(1.0))
        .scroll(rune::ScrollSpec {
            stick_bottom: false,
        })
        .gap(12.0)
        .child(
            text("Recent projects")
                .font(k.ty.ui_medium)
                .color(k.c.text_muted),
        );
    for root in &state.recent_projects {
        contents = contents.child(recent_project(state, k, root));
    }
    contents = contents.child(divider(k)).child(actions(state, k));
    layers.push(popover::raised(
        k,
        contents
            .key("project-picker")
            .role("dialog")
            .label("Choose project")
            .focus_return("project-picker-trigger")
            .modal(true, State::close_overlay)
            .anchored_fit(
                "project-picker-trigger",
                Side::Bottom,
                Align::Start,
                theme::SPACE,
            ),
        k.radius.md,
    ));
}

fn recent_project(state: &State, k: &Kit, root: &str) -> El {
    let selected = root.to_owned();
    let name = std::path::Path::new(root)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.to_owned());
    let path = root.to_owned();
    Button::new(name.clone())
        .label_with(move |font, ink| {
            row()
                .w_full()
                .gap(12.0)
                .center_y()
                .child(icon(&glyphs::FOLDER, 20.0).color(ink).a11y_hidden())
                .child(
                    col()
                        .grow(1.0)
                        .min_w(0.0)
                        .gap(2.0)
                        .child(text(name).font(font).color(ink).truncate())
                        .child(text(path).font(font).color(ink.alpha(0.7)).truncate()),
                )
        })
        .variant(ButtonVariant::Ghost)
        .on_press(move |state: &mut State| state.select_recent_project(selected.clone()))
        .build(k)
        // Keep project identity readable while analysis disables navigation.
        .disabled(state.project_job_busy())
        .focusable(!state.project_job_busy())
        .help(root)
        .h(56.0)
        .w_full()
}

fn repository(state: &State, k: &Kit) -> El {
    col()
        .w_full()
        .gap(8.0)
        .child(
            text("Project folder")
                .font(k.ty.ui_medium)
                .color(k.c.text_muted),
        )
        .child(
            TextInput::new(&state.root)
                .width(None)
                .label("Project folder")
                .placeholder("Project folder")
                .on_edit(|s: &mut State| &mut s.root)
                .on_change(|s: &mut State, _| {
                    s.root_edited();
                    s.ui.overlay = Some(Overlay::Projects);
                })
                .on_submit(|s: &mut State, _| {
                    s.open_project(s.root.text.clone());
                })
                .build(k)
                .key("root-path"),
        )
        .child(note(analysis_help(state), k))
}

fn analysis_help(state: &State) -> &'static str {
    if state.jobs.boot.is_pending() {
        "Opening the saved workspace…"
    } else if state.project_open.loading() {
        "Opening the saved graph…"
    } else if state.analyzing && state.graph.is_some() {
        "Analyzing selected builds; the previous graph remains available."
    } else if state.targets.iter().all(|t| !t.selected) {
        "Select one or more builds, then generate or refresh the graph."
    } else {
        "Analysis runs locally and may restore dependencies or compile project code."
    }
}

fn note(value: &str, k: &Kit) -> El {
    text(value)
        .font(k.ty.ui)
        .color(k.c.text_muted)
        .w_full()
        .wrap_anywhere()
}

fn actions(state: &State, k: &Kit) -> El {
    col()
        .w_full()
        .gap(8.0)
        .child(repository(state, k))
        .child(divider(k))
        .child(
            workspace_toolbar::clear_cache_action(state, k)
                .min_h(36.0)
                .w_full(),
        )
        .child(
            workspace_toolbar::analysis_action(state, k)
                .min_h(36.0)
                .w_full(),
        )
        .child(
            Button::new("Open folder…")
                .variant(ButtonVariant::Outline)
                .disabled(state.project_job_busy())
                .on_press_tx(|state: &mut State, tx| {
                    state.close_overlay();
                    state.choose_folder(tx);
                })
                .build(k)
                .key("choose-folder")
                .min_h(36.0)
                .w_full(),
        )
        .child(build_picker::button(state, k).min_h(36.0).w_full())
}

fn divider(k: &Kit) -> El {
    el().w_full().h(1.0).fill(k.c.border_muted).a11y_hidden()
}
