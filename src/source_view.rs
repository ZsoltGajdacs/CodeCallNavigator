//! Selectable highlighted source rows shared by previews and generation-bound flowcharts.
use crate::{source::SourceFile, theme};
use rune::{El, row, text};
use rune_kit::{Kit, syntax::SyntaxTheme};

pub struct SourceRow {
    pub index: usize,
    pub selected: bool,
    pub key: String,
}

pub fn line(file: &SourceFile, source_row: SourceRow, k: &Kit) -> El {
    let SourceRow {
        index,
        selected,
        key,
    } = source_row;
    let line = &file.lines[index];
    row()
        .h(20.0)
        .w_full()
        .fill(if selected {
            k.c.text_accent.alpha(0.14)
        } else {
            rune::Color::TRANSPARENT
        })
        .child(
            text(format!("{:>5}", index + 1))
                .w(64.0)
                .font(theme::mono(12.0, 400.0).lh(20.0))
                .color(k.c.text_muted),
        )
        .child(
            text(file.text[line.range.clone()].trim_end_matches('\r'))
                .key(key)
                .font(theme::mono(12.0, 400.0).lh(20.0))
                .color(k.c.text_base)
                .spans(SyntaxTheme::from_kit(k).spans(&line.highlights, 0))
                .selectable(true)
                .selection_color(k.selection()),
        )
}
