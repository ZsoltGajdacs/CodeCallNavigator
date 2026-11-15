//! The overlay lists the CLI's exact identities and recorded availability without building charts.
use crate::{
    app::State,
    engine_client::flow_comparison::{self as wire, CatalogSide, Recorded},
};
use rune::{El, col, el, row, text};
use rune_kit::{Button, ButtonVariant, Kit, TextInput};
pub(super) fn layers(state: &State, k: &Kit) -> Vec<El> {
    if !state.flow_comparison.controls.catalog_open {
        return Vec::new();
    }
    let panel = col()
        .key("comparison-catalog")
        .w(720.0)
        .h(450.0)
        .min_h(0.0)
        .padding(12.0)
        .gap(8.0)
        .radius(10.0)
        .border(1.0, k.c.std_border)
        .fill(k.c.layer_01)
        .child(header(state, k))
        .child(search(state, k))
        .child(records(state, k))
        .child(pager(state, k))
        .modal(true, |s: &mut State| {
            s.flow_comparison.controls.catalog_open = false
        })
        .modal_click_outside(true);
    vec![
        el().key("comparison-catalog-scrim")
            .absolute(0.0, 0.0)
            .w_full()
            .h_full()
            .fill(k.c.scrim)
            .a11y_hidden(),
        col()
            .key("comparison-catalog-layer")
            .absolute(0.0, 0.0)
            .w_full()
            .h_full()
            .pt(110.0)
            .items_center()
            .child(panel),
    ]
}
fn header(state: &State, k: &Kit) -> El {
    row()
        .w_full()
        .gap(8.0)
        .center_y()
        .child(
            text(if state.flow_comparison.controls.choosing_after {
                "Choose an explicit after counterpart"
            } else {
                "Choose an exact function identity"
            })
            .font(k.ty.ui_medium)
            .color(k.c.text_base)
            .grow(1.0)
            .min_w(0.0)
            .truncate(),
        )
        .child(
            Button::new("Done")
                .variant(ButtonVariant::Outline)
                .on_press(|s: &mut State| s.flow_comparison.controls.catalog_open = false)
                .build(k)
                .key("comparison-catalog-close"),
        )
}
fn search(state: &State, k: &Kit) -> El {
    row()
        .w_full()
        .gap(8.0)
        .child(
            TextInput::new(&state.flow_comparison.controls.search)
                .width(None)
                .label("Search comparison identities")
                .placeholder("Name, signature, file or exact identity")
                .on_edit(|s: &mut State| &mut s.flow_comparison.controls.search)
                .on_key(|s: &mut State, event, _| {
                    if event.key == rune::Key::Enter {
                        s.flow_comparison.search_control();
                        true
                    } else {
                        false
                    }
                })
                .build(k)
                .key("comparison-search")
                .grow(1.0)
                .min_w(0.0),
        )
        .child(
            Button::new("Search")
                .disabled(state.flow_comparison.catalog_pending())
                .on_press(|s: &mut State| s.flow_comparison.search_control())
                .build(k)
                .key("comparison-search-submit"),
        )
}
fn records(state: &State, k: &Kit) -> El {
    let mut body = col()
        .key("comparison-records")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .gap(3.0)
        .scroll(rune::ScrollSpec {
            stick_bottom: false,
        });
    let Some(page) = &state.flow_comparison.catalog else {
        return body.child(
            text("Loading saved identities…")
                .font(k.ty.small)
                .color(k.c.text_muted),
        );
    };
    for record in &page.records {
        body = body.child(record_row(state, record, k));
    }
    if page.records.is_empty() {
        body = body.child(
            text("No identities match this search")
                .font(k.ty.small)
                .color(k.c.text_muted),
        );
    }
    body
}
fn record_row(state: &State, record: &wire::Record, k: &Kit) -> El {
    let name = record_name(record);
    let membership = match record.membership {
        wire::Membership::Both => "Both revisions",
        wire::Membership::Added => "Added",
        wire::Membership::Removed => "Removed",
        wire::Membership::Unknown => "Membership unknown",
    };
    let disabled = state.flow_comparison.catalog_pending() || counterpart_disabled(state, record);
    let symbol = record.symbol.clone();
    col()
        .key(format!("comparison-record-{symbol}"))
        .w_full()
        .min_w(0.0)
        .h(66.0)
        .padding(6.0)
        .gap(2.0)
        .radius(5.0)
        .fill(k.c.bg_base)
        .child(
            text(format!("{name} · {membership}"))
                .font(k.ty.small_medium)
                .color(if disabled {
                    k.c.text_muted
                } else {
                    k.c.text_base
                })
                .w_full()
                .truncate(),
        )
        .child(
            text(record_owner(record))
                .font(k.ty.small)
                .color(k.c.text_muted)
                .w_full()
                .truncate(),
        )
        .child(
            text(format!(
                "Before: {} · After: {}",
                availability(&record.before),
                availability(&record.after)
            ))
            .font(k.ty.small)
            .color(k.c.text_muted)
            .w_full()
            .truncate(),
        )
        .help(format!(
            "Identity: {}\nBefore: {}\nAfter: {}",
            record.symbol,
            side_description(&record.before),
            side_description(&record.after)
        ))
        .role("button")
        .label(name)
        .focusable(!disabled)
        .on_press(move |s: &mut State| {
            if !disabled {
                s.flow_comparison.choose_control(symbol.clone());
            }
        })
}
fn counterpart_disabled(state: &State, record: &wire::Record) -> bool {
    if !state.flow_comparison.controls.choosing_after {
        return false;
    }
    !matches!(record.after, CatalogSide::Present(_))
        || state
            .flow_comparison
            .current
            .as_ref()
            .is_none_or(|pair| pair.selection.symbol == record.symbol)
}
fn record_node(record: &wire::Record) -> Option<&wire::Symbol> {
    [&record.before, &record.after]
        .into_iter()
        .find_map(|side| match side {
            CatalogSide::Present(present) => present.node.as_ref(),
            _ => None,
        })
}
fn record_name(record: &wire::Record) -> String {
    record_node(record)
        .map(|node| {
            if node.signature.is_empty() {
                node.label.clone()
            } else {
                node.signature.clone()
            }
        })
        .unwrap_or_else(|| record.symbol.clone())
}
fn record_owner(record: &wire::Record) -> String {
    record_node(record)
        .map(|node| format!("{} · {}", node.owner.project, node.owner.file))
        .unwrap_or_else(|| "Ownership unavailable".into())
}
fn side_description(side: &CatalogSide) -> String {
    match side {
        CatalogSide::Present(present) => present
            .node
            .as_ref()
            .map(|node| {
                format!(
                    "{} · {} · {}:{}\nBuild: {} · {}\n{}",
                    node.signature,
                    node.owner.project,
                    node.owner.file,
                    node.line,
                    node.owner.build,
                    node.owner.language,
                    availability(side)
                )
            })
            .unwrap_or_else(|| availability(side)),
        _ => availability(side),
    }
}
fn availability(side: &CatalogSide) -> String {
    match side {
        CatalogSide::Missing => "missing".into(),
        CatalogSide::Unknown(failure) => format!("unknown · {}", failure.message),
        CatalogSide::Present(present) => match &present.flow {
            Recorded::Recorded => "flow recorded".into(),
            Recorded::Unavailable(failure) => format!("unavailable · {}", failure.message),
        },
    }
}
fn pager(state: &State, k: &Kit) -> El {
    let workspace = &state.flow_comparison;
    let pending = workspace.catalog_pending();
    let label = workspace
        .catalog
        .as_ref()
        .map(|page| {
            format!(
                "{} shown · {} known identities · offset {}",
                page.returned, page.total, page.offset
            )
        })
        .unwrap_or_else(|| "Loading…".into());
    row()
        .w_full()
        .gap(8.0)
        .center_y()
        .child(
            text(label)
                .font(k.ty.small)
                .color(k.c.text_muted)
                .grow(1.0)
                .min_w(0.0)
                .truncate(),
        )
        .child(
            Button::new("Previous page")
                .disabled(pending || !workspace.has_previous_page())
                .on_press(|s: &mut State| s.flow_comparison.previous_control())
                .build(k)
                .key("comparison-page-previous"),
        )
        .child(
            Button::new("Next page")
                .disabled(pending || workspace.catalog.as_ref().is_none_or(|page| !page.more))
                .on_press(|s: &mut State| s.flow_comparison.next_control())
                .build(k)
                .key("comparison-page-next"),
        )
}
