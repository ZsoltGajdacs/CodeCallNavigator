use super::Contents;
use crate::{app::State, theme};
use rune::{El, GrowthSpec, col, el, row, text};
use rune_kit::{Button, ButtonVariant, Kit};

pub fn view(state: &State, k: &Kit) -> Option<El> {
    let preview = state.source_popup.active.as_ref()?;
    let [x, y, w, h] = state.source_popup.bounds(state);
    Some(
        col()
            .key("source-popup")
            .absolute(x, y)
            .size(w, h)
            .min_h(0.0)
            .fill(k.c.surface_raised)
            .border(1.0, k.c.std_border)
            .radius(8.0)
            .shadow(6.0, 24.0, k.c.bg_deep.alpha(0.5))
            .clip_rounded(8.0)
            .on_press_tx(|_: &mut State, tx| tx.live_only())
            .role("group")
            .label("Source preview")
            .on_layout(|s: &mut State, rect| s.source_popup.rect = rect)
            .on_hover_change(|s: &mut State, inside, tx| {
                s.source_popup.inside = inside;
                s.source_popup.close_at = if inside {
                    None
                } else {
                    Some(s.source_popup.now + 0.3)
                };
                if inside {
                    s.source_popup.hover = None;
                }
                tx.wake_after(std::time::Duration::from_millis(300));
            })
            .child(header(state, k))
            .child(
                text(preview.path.as_deref().unwrap_or(&preview.node.label))
                    .key("source-path")
                    .w_full()
                    .px(12.0)
                    .font(k.ty.small)
                    .color(k.c.text_muted)
                    .truncate()
                    .help(preview.path.as_deref().unwrap_or(&preview.node.label)),
            )
            .child(notice(state, k))
            .child(body(state, k))
            .child(footer(preview.path.is_some(), k)),
    )
}

fn footer(source: bool, k: &Kit) -> El {
    let label = if source {
        "Current disk contents"
    } else {
        "Files represented in this graph"
    };
    row()
        .w_full()
        .h(24.0)
        .center_y()
        .child(
            text(label)
                .grow(1.0)
                .px(12.0)
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
        .child(
            el().key("source-resize")
                .size(24.0, 24.0)
                .cursor(rune::Cursor::Crosshair)
                .role("button")
                .label("Resize source preview")
                .on_drag(|s: &mut State, drag, _| s.resize_source(drag))
                .child(text("◢").font(k.ty.small).color(k.c.text_muted)),
        )
}

fn header(state: &State, k: &Kit) -> El {
    let preview = state.source_popup.active.as_ref().unwrap();
    let mut bar = row().w_full().gap(6.0).padding(8.0).center_y();
    if preview.path.is_some() && preview.node.member.is_none() {
        bar = bar.child(
            Button::new("Back")
                .variant(ButtonVariant::Ghost)
                .on_press(State::source_files_back)
                .build(k)
                .key("source-back"),
        );
    }
    bar.child(
        text(if preview.path.is_some() {
            "Source"
        } else {
            "Source files"
        })
        .font(k.ty.ui_medium)
        .color(k.c.text_base)
        .grow(1.0),
    )
    .child(
        Button::new(if preview.pinned { "Unpin" } else { "Pin" })
            .variant(ButtonVariant::Ghost)
            .on_press(State::pin_source)
            .build(k)
            .key("source-pin"),
    )
    .child(
        Button::new("Close")
            .variant(ButtonVariant::Ghost)
            .on_press(State::close_source)
            .build(k)
            .key("source-close"),
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
                            .font(theme::mono(12.0, 400.0))
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
