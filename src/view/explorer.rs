//! Virtualized file tree; navigation decisions stay in workspace operations.
use crate::{
    app::State,
    explorer::{ROW_HEIGHT, Row, RowKind},
    theme,
};
use rune::{El, GrowthSpec, col, row, text};
use rune_kit::{Button, Kit, ScrollBar};

pub fn view(state: &State, k: &Kit) -> El {
    let kit = k.clone();
    let explorer = &state.explorer;
    col()
        .key("project-explorer")
        .w_full()
        .grow(1.0)
        .min_h(90.0)
        .child(
            row()
                .w_full()
                .center_y()
                .gap(4.0)
                .child(
                    text("Project Explorer")
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base)
                        .grow(1.0),
                )
                .child(
                    Button::new("Refresh files")
                        .on_press(|s: &mut State| s.explorer.refresh())
                        .build(k)
                        .key("explorer-refresh"),
                ),
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
    let row = &state.explorer.rows[index];
    let path = row.path.clone();
    let double_path = path.clone();
    let kind = row.kind.clone();
    let reveal = row.is_file();
    let selected = state.explorer.selected.contains(&path);
    text(format!("{}{}", prefix(state, row), row.label))
        .key(row.key())
        .h(ROW_HEIGHT)
        .w_full()
        .pl(4.0 + row.depth as f32 * 14.0)
        .pr(8.0)
        .font(theme::mono(12.0, if selected { 600.0 } else { 400.0 }))
        .color(k.c.text_base)
        .fill(if selected { k.c.layer_02 } else { k.c.bg_base })
        .hover(k.c.layer_01)
        .truncate()
        .role("button")
        .label(&row.path)
        .help(&row.label)
        // Rows mirror graph selection; pointer presses must retain graph keyboard focus.
        .focusable(false)
        .preserve_focus_on_press(true)
        .on_press(move |s: &mut State| press(s, &path, &kind))
        .on_double_press_tx(move |s: &mut State, tx| {
            if reveal && s.reveal_file_classes(double_path.clone()) {
                tx.focus("graph-canvas", false);
            }
        })
}

fn prefix(state: &State, row: &Row) -> &'static str {
    match row.kind {
        RowKind::Directory if state.explorer.expanded.contains(&row.path) => "▾ ",
        RowKind::Directory => "▸ ",
        RowKind::Symlink => "↗ ",
        _ => "  ",
    }
}
fn press(state: &mut State, path: &str, kind: &RowKind) {
    match kind {
        RowKind::Directory => state.explorer.toggle(path.into()),
        RowKind::File | RowKind::Symlink => state.explorer.select(path.into()),
        RowKind::Error => state.explorer.retry(path.into()),
        _ => {}
    }
}
