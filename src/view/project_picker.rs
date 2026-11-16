//! Project selection presentation; discovery owns filesystem validation and failures.
use super::{build_picker, workspace_toolbar};
use crate::{app::State, presentation::Overlay, theme};
use rune::{Align, El, Side, col, text};
use rune_kit::{Button, ButtonVariant, Kit, TextInput, popover};

pub(super) fn button(state: &State, k: &Kit) -> El {
    let displayed = state
        .query_context
        .as_ref()
        .map(|context| context.descriptor.project.to_string_lossy().into_owned())
        .or_else(|| {
            state
                .graph
                .as_ref()
                .and_then(|graph| graph.data.roots.first().cloned())
        })
        .unwrap_or_else(|| state.root.text.trim().to_owned());
    let path = displayed.as_str();
    let title = std::path::Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Open project…");
    let visible_title = title.to_owned();
    Button::new(title)
        .label_with(move |font, ink| {
            text(visible_title)
                .key("workspace-identity")
                .font(font)
                .color(ink)
                .max_w(220.0)
                .truncate()
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
        .w(320.0_f32.min((state.source_popup.window[0] - 24.0).max(1.0)))
        .padding(12.0)
        .max_h((state.source_popup.window[1] - 100.0).max(1.0))
        .scroll(rune::ScrollSpec {
            stick_bottom: false,
        })
        .gap(6.0)
        .child(
            text("Recent projects")
                .font(k.ty.small_medium)
                .color(k.c.text_muted),
        );
    for root in &state.recent_projects {
        let selected = root.clone();
        let name = std::path::Path::new(root)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.clone());
        contents = contents.child(
            Button::new(name)
                .variant(ButtonVariant::Ghost)
                .disabled(state.project_job_busy())
                .on_press(move |state: &mut State| state.select_recent_project(selected.clone()))
                .build(k)
                .help(root)
                .w_full(),
        );
    }
    contents = contents.child(actions(state, k));
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

fn repository(state: &State, k: &Kit) -> El {
    col()
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
                .on_submit(|s: &mut State, _| s.discover_root())
                .build(k)
                .key("root-path"),
        )
        .child(note(analysis_help(state), k))
}

fn analysis_help(state: &State) -> &'static str {
    if state.jobs.boot.is_pending() {
        "Opening the saved workspace…"
    } else if state.analyzing {
        "Analyzing selected builds; the previous graph remains available."
    } else if state.targets.iter().all(|t| !t.selected) {
        "Select one or more builds, then generate or refresh the graph."
    } else {
        "Analysis runs locally and may restore dependencies or compile project code."
    }
}

fn note(value: &str, k: &Kit) -> El {
    text(value)
        .font(k.ty.text_xsmall)
        .color(k.c.text_muted)
        .w_full()
        .wrap_anywhere()
}

fn actions(state: &State, k: &Kit) -> El {
    col()
        .child(repository(state, k))
        .child(
            Button::new("Compare flows")
                .variant(ButtonVariant::Outline)
                .on_press(|state: &mut State| {
                    state.close_overlay();
                    state.show_flow_comparison();
                })
                .build(k)
                .key("compare-flows"),
        )
        .child(workspace_toolbar::clear_cache_action(state, k))
        .child(workspace_toolbar::analysis_action(state, k))
        .child(
            Button::new("Open folder…")
                .disabled(state.project_job_busy())
                .on_press_tx(|state: &mut State, tx| {
                    state.close_overlay();
                    state.choose_folder(tx);
                })
                .build(k)
                .key("choose-folder")
                .w_full(),
        )
        .child(build_picker::button(state, k))
}
