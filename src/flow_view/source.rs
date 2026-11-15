//! Virtualized analyzed source with explicit single/before/after widget identity.
use super::address::Address;
use crate::{app::State, flow_workspace::Session};
use rune::{El, GrowthSpec, col};
use rune_kit::Kit;
pub(crate) fn view(state: &State, session: &Session, address: Address, k: &Kit) -> El {
    let pane = col()
        .key(address.key("source-pane"))
        .w_full()
        .h_full()
        .min_h(0.0)
        .fill(k.c.bg_base);
    let Some(source) = session.displayed_source() else {
        return pane;
    };
    let kit = k.clone();
    let keys = address.keys(session);
    let offset = session.scroll.clone();
    let mounted_key = session.scroll_key.clone();
    let width = (source.max_columns as f32 * 8.0 + 90.0).max(300.0);
    pane.child(
        col()
            .key(session.scroll_key.clone())
            .on_layout_tx(move |s: &mut State, _, tx| address.mounted(s, &mounted_key, tx))
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .scroll_xy()
            .selection_group(true)
            .on_scroll_offset(move |at| offset.set(at.offset))
            .child(
                col()
                    .w(width)
                    .growth(GrowthSpec::default())
                    .virtual_rows_version(address.revision(state))
                    .virtual_rows(
                        source.lines.len(),
                        20.0,
                        move |_: &State, i| keys.line(i).into(),
                        move |s: &State, i| source_line(s, i, address, &kit),
                    ),
            ),
    )
}
fn source_line(state: &State, index: usize, address: Address, k: &Kit) -> El {
    let Some(session) = address.session(state) else {
        return rune::el().h(20.0);
    };
    let Some(ready) = &session.ready else {
        return rune::el().h(20.0);
    };
    let Some(source) = session.displayed_source() else {
        return rune::el().h(20.0);
    };
    let Some(line) = source.lines.get(index) else {
        return rune::el().h(20.0);
    };
    let line_end = source
        .lines
        .get(index + 1)
        .map(|next| next.range.start)
        .unwrap_or(line.range.end);
    let selected = session
        .selected
        .and_then(|i| ready.chart.steps.get(i))
        .is_some_and(|step| line.range.start < step.span.end && line_end > step.span.start);
    crate::source_view::line(
        source,
        crate::source_view::SourceRow {
            index,
            selected,
            key: address.keys(session).text(index),
        },
        k,
    )
}
