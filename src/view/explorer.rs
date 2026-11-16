//! Virtualized file tree; navigation decisions stay in workspace operations.
use crate::{
    app::State,
    explorer::{ROW_HEIGHT, Row, RowKind},
};
use rune::{El, GrowthSpec, col, el, icon, row, text};
use rune_kit::{Kit, ScrollBar, TextInput, file_icon, glyphs};

pub fn view(state: &State, k: &Kit) -> El {
    let kit = k.clone();
    let explorer = &state.explorer;
    col()
        .key("project-explorer")
        .w_full()
        .grow(1.0)
        .min_h(90.0)
        .gap(16.0)
        .child(
            TextInput::new(&state.explorer.filter)
                .label("Filter loaded files")
                .placeholder("Filter files…")
                .leading_icon(k.glyph.search)
                .width(None)
                .on_edit(|state: &mut State| &mut state.explorer.filter)
                .on_change(|state: &mut State, _| state.explorer.filter_changed())
                .build(k)
                .key("explorer-filter"),
        )
        .child(
            col()
                .key("explorer-scroll")
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .scroll_xy()
                .child(if explorer.root.is_empty() {
                    text(if state.root.text.trim().is_empty() {
                        "Choose a project folder."
                    } else {
                        "Loading…"
                    })
                    .font(k.ty.small)
                    .color(k.c.text_muted)
                    .h(ROW_HEIGHT)
                } else {
                    col()
                        .w_full()
                        .growth(GrowthSpec::default())
                        .virtual_rows_version(explorer.serial)
                        .virtual_rows(
                            explorer.rows.len(),
                            ROW_HEIGHT,
                            |s: &State, index| s.explorer.rows[index].key().into(),
                            move |s: &State, index| file_row(s, index, &kit),
                        )
                })
                .scroll_bar(k),
        )
}

fn file_row(state: &State, index: usize, k: &Kit) -> El {
    let entry = &state.explorer.rows[index];
    let path = entry.path.clone();
    let double_path = path.clone();
    let kind = entry.kind.clone();
    let reveal = entry.is_file();
    let selected = state.explorer.selected.contains(&path);
    row()
        .center_y()
        .gap(8.0)
        .key(entry.key())
        .h(ROW_HEIGHT)
        .w_full()
        .pl(4.0 + entry.depth as f32 * 14.0)
        .pr(8.0)
        .radius(6.0)
        .fill(if selected {
            k.c.text_accent.alpha(0.15)
        } else {
            k.c.bg_base
        })
        .hover(k.c.layer_01)
        .role("button")
        .label(&entry.path)
        .help(&entry.label)
        // Rows mirror graph selection; pointer presses must retain graph keyboard focus.
        .focusable(false)
        .preserve_focus_on_press(true)
        .on_press(move |s: &mut State| press(s, &path, &kind))
        .child(disclosure(state, entry, k))
        .child(entry_icon(state, entry, k))
        .child(
            text(&entry.label)
                .font(if selected { k.ty.ui_medium } else { k.ty.ui })
                .color(k.c.text_base)
                .truncate()
                .grow(1.0)
                .min_w(0.0),
        )
        .on_double_press_tx(move |s: &mut State, tx| {
            if reveal && s.reveal_file_classes(double_path.clone()) {
                tx.focus("graph-canvas", false);
            }
        })
}

fn disclosure(state: &State, entry: &Row, k: &Kit) -> El {
    if !matches!(entry.kind, RowKind::Directory) {
        return el().w(12.0);
    }
    let glyph = if state.explorer.expanded.contains(&entry.path) {
        k.glyph.chevron_down
    } else {
        &crate::icons::OUTLINE_CHEVRON_RIGHT
    };
    icon(glyph, 12.0).color(k.c.text_muted).a11y_hidden()
}

fn entry_icon(state: &State, entry: &Row, k: &Kit) -> El {
    let glyph = match entry.kind {
        RowKind::Directory if state.explorer.expanded.contains(&entry.path) => &glyphs::FOLDER_OPEN,
        RowKind::Directory => &glyphs::FOLDER,
        RowKind::File | RowKind::Symlink => file_icon(&entry.label),
        _ => return el().w(16.0),
    };
    icon(glyph, 16.0)
        .key(format!("explorer-icon-{}", entry.key()))
        .color(k.c.text_base)
        .a11y_hidden()
}
fn press(state: &mut State, path: &str, kind: &RowKind) {
    match kind {
        RowKind::Directory => state.explorer.toggle(path.into()),
        RowKind::File | RowKind::Symlink => state.explorer.select(path.into()),
        RowKind::Error => state.explorer.retry(path.into()),
        _ => {}
    }
}
