use super::{Content, Contents, Heading, Owner, Panel};
use crate::app::State;
use rune::{El, GrowthSpec, col, el, text};
use rune_kit::{Button, ButtonVariant, Kit};

pub fn view(state: &State, k: &Kit) -> Option<El> {
    let popup = &state.source_popup;
    let preview = popup.active.as_ref()?;
    let has_path = preview.path.is_some();
    let back = (has_path && preview.node.member.is_none()).then(|| {
        Button::new("Back")
            .variant(ButtonVariant::Ghost)
            .on_press(State::source_files_back)
            .build(k)
            .key("source-back")
    });
    let panel = Panel {
        owner: Owner::Graph,
        bounds: popup.bounds(state),
        interaction: &popup.interaction,
    };
    Some(
        panel.view(
            Content {
                heading: Heading {
                    title: if has_path {
                        &preview.node.label
                    } else {
                        "Source files"
                    },
                    lead: back,
                },
                description: preview.path.as_deref().unwrap_or(&preview.node.label),
                body: col()
                    .w_full()
                    .grow(1.0)
                    .min_h(0.0)
                    .child(notice(state, k))
                    .child(body(state, k)),
                provenance: if has_path {
                    "Current disk contents"
                } else {
                    "Files represented in this graph"
                },
            },
            k,
        ),
    )
}

fn notice(state: &State, k: &Kit) -> El {
    let preview = state.source_popup.active.as_ref().unwrap();
    let message = [
        state.freshness.attention(state.generation.as_deref()),
        preview.location_warning.clone(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ");
    if message.is_empty() {
        el()
    } else {
        text(message)
            .key("source-notice")
            .w_full()
            .px(12.0)
            .font(k.ty.small)
            .color(k.c.warning.fg)
            .wrap()
    }
}

fn body(state: &State, k: &Kit) -> El {
    match &state.source_popup.active.as_ref().unwrap().contents {
        Contents::Files => file_list(state, k),
        Contents::Source(_) => source_lines(state, k),
        Contents::Loading => message("Loading source…", k),
        Contents::Error(error) => message(error, k),
    }
}

fn message(value: &str, k: &Kit) -> El {
    col()
        .key("source-message")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .padding(12.0)
        .child(
            text(value)
                .w_full()
                .wrap()
                .font(k.ty.ui)
                .color(k.c.text_muted),
        )
}

fn file_list(state: &State, k: &Kit) -> El {
    let count = state.source_popup.active.as_ref().unwrap().node.files.len();
    if count == 0 {
        return message("No source files are represented in this group.", k);
    }
    let kit = k.clone();
    col()
        .key("source-files-scroll")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll_xy()
        .child(
            col()
                .growth(GrowthSpec::default())
                .virtual_rows_version(state.source_popup.serial)
                .virtual_rows(
                    count,
                    32.0,
                    |s: &State, i| format!("source-file-{}-{i}", s.source_popup.serial).into(),
                    move |s: &State, i| {
                        let file = &s.source_popup.active.as_ref().unwrap().node.files[i];
                        let path = file.path.clone();
                        text(&file.label)
                            .key(format!("source-file-row-{i}"))
                            .h(32.0)
                            .px(12.0)
                            .font(kit.ty.ui)
                            .color(kit.c.text_base)
                            .hover(kit.c.layer_01)
                            .role("button")
                            .label(&file.path)
                            .focusable(true)
                            .on_press(move |s: &mut State| s.open_source_file(path.clone(), 1))
                    },
                ),
        )
}

fn source_lines(state: &State, k: &Kit) -> El {
    let preview = state.source_popup.active.as_ref().unwrap();
    let Contents::Source(file) = &preview.contents else {
        unreachable!()
    };
    let width = (file.max_columns as f32 * 8.0 + 90.0).max(240.0);
    let kit = k.clone();
    col()
        .key("source-scroll")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll_xy()
        .selection_group(true)
        .child(
            col()
                .w(width)
                .growth(GrowthSpec::default())
                .virtual_rows_version(state.source_popup.serial)
                .virtual_rows(
                    file.lines.len(),
                    20.0,
                    |_: &State, i| format!("source-line-{i}").into(),
                    move |s: &State, i| code_line(s, i, &kit),
                ),
        )
}

fn code_line(state: &State, i: usize, k: &Kit) -> El {
    let preview = state.source_popup.active.as_ref().unwrap();
    let Contents::Source(file) = &preview.contents else {
        return el();
    };
    let selected = preview.location_warning.is_none() && preview.target == i;
    crate::source_view::line(
        file,
        crate::source_view::SourceRow {
            index: i,
            selected,
            key: format!("source-text-{i}"),
        },
        k,
    )
}
