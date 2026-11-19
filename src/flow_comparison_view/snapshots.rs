//! Render local snapshot controls and one page of the engine's graph differences.
use super::provenance;
use crate::{
    app::State,
    engine_client::flow_comparison::{GraphChange, GraphPage, SnapshotStatus},
    flow_comparison::{GraphNavigation, SnapshotSide},
};
use rune::{El, ScrollSpec, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit, TextInput};

pub(super) fn controls(state: &State, k: &Kit) -> El {
    let snapshots = &state.flow_comparison.snapshots;
    let mut body = col()
        .key("comparison-snapshots-controls")
        .w_full()
        .gap(5.0)
        .px(10.0)
        .pb(5.0)
        .child(
            row()
                .gap(8.0)
                .child(
                    Button::new(if snapshots.panel_open {
                        "Hide local snapshots"
                    } else {
                        "Local snapshots"
                    })
                    .variant(ButtonVariant::Outline)
                    .on_press(|state: &mut State| {
                        let controls = &mut state.flow_comparison.controls;
                        controls.catalog_open = false;
                        let snapshots = &mut state.flow_comparison.snapshots;
                        snapshots.panel_open = !snapshots.panel_open;
                    })
                    .build(k)
                    .key("comparison-snapshots"),
                )
                .child(
                    Button::new("Compare graph differences")
                        .variant(ButtonVariant::Outline)
                        .disabled(state.runtime.is_none() || snapshots.pending())
                        .on_press(|state: &mut State| {
                            state.compare_snapshot_graphs(GraphNavigation::First)
                        })
                        .build(k)
                        .key("comparison-graph-differences"),
                ),
        );
    if !snapshots.message.is_empty() {
        body = body.child(
            text(&snapshots.message)
                .font(k.ty.small)
                .color(k.c.text_muted)
                .truncate()
                .help(&snapshots.message)
                .key("comparison-snapshot-status"),
        );
    }
    if !snapshots.panel_open {
        return body;
    }
    let refs = state.flow_comparison.control_refs();
    body.child(build_targets(state, k)).child(
        row()
            .w_full()
            .gap(10.0)
            .child(side(state, SnapshotSide::Base, &refs.base, k))
            .child(side(state, SnapshotSide::Head, &refs.head, k)),
    )
}

fn build_targets(state: &State, k: &Kit) -> El {
    let snapshots = &state.flow_comparison.snapshots;
    let targets = snapshots
        .targets
        .iter()
        .map(|target| target.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let mut body = col().w_full().gap(3.0).child(
        row()
            .gap(6.0)
            .child(
                TextInput::new(&snapshots.target)
                    .width(None)
                    .label("Snapshot build target")
                    .placeholder("Project-relative build target (empty uses defaults)")
                    .on_edit(|state: &mut State| &mut state.flow_comparison.snapshots.target)
                    .build(k)
                    .grow(1.0)
                    .min_w(0.0)
                    .key("comparison-snapshot-target"),
            )
            .child(
                Button::new("Add target")
                    .disabled(snapshots.target.text.trim().is_empty())
                    .on_press(|state: &mut State| state.flow_comparison.snapshots.add_target())
                    .build(k),
            )
            .child(
                Button::new("Clear targets")
                    .disabled(snapshots.targets.is_empty())
                    .on_press(|state: &mut State| state.flow_comparison.snapshots.targets.clear())
                    .build(k),
            ),
    );
    if !targets.is_empty() {
        body = body.child(
            text(&targets)
                .font(k.ty.small)
                .color(k.c.text_muted)
                .truncate()
                .help(&targets),
        );
    }
    body
}

fn side(state: &State, side: SnapshotSide, revision: &str, k: &Kit) -> El {
    let snapshots = &state.flow_comparison.snapshots;
    let disabled = state.runtime.is_none()
        || snapshots.pending()
        || revision.trim().is_empty()
        || revision == "WORKTREE";
    let scope = format!("comparison-snapshot-{}", side.label().to_lowercase());
    let mut panel = col().key(&scope).grow(1.0).min_w(0.0).gap(4.0).child(
        row()
            .gap(5.0)
            .center_y()
            .child(
                text(side.label())
                    .font(k.ty.small_medium)
                    .color(k.c.text_base),
            )
            .child(
                Button::new("Find snapshots")
                    .disabled(disabled)
                    .on_press(move |state: &mut State| state.lookup_comparison_snapshot(side))
                    .build(k)
                    .key(format!("{scope}-lookup")),
            )
            .child(
                Button::new("Prepare snapshot")
                    .disabled(disabled)
                    .on_press(move |state: &mut State| state.prepare_comparison_snapshot(side))
                    .build(k)
                    .key(format!("{scope}-prepare")),
            ),
    );
    if let Some(lookup) = snapshots.lookup(side, revision) {
        panel = panel.child(variants(
            side,
            &lookup.status,
            lookup.selected.as_deref(),
            k,
        ));
    }
    panel
}

fn variants(side: SnapshotSide, status: &SnapshotStatus, selected: Option<&str>, k: &Kit) -> El {
    let path = status.cache_path.display().to_string();
    let list = col()
        .gap(3.0)
        .max_h(105.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .children(status.snapshots.iter().map(|record| {
            let id = record.id.clone();
            let is_selected = selected == Some(id.as_str());
            let label = format!(
                "{}{} · {}",
                if is_selected { "Selected: " } else { "" },
                short(&id),
                record.identity.analyzer,
            );
            Button::new(label)
                .disabled(is_selected)
                .on_press(move |state: &mut State| {
                    state
                        .flow_comparison
                        .snapshots
                        .choose(side, Some(id.clone()));
                })
                .build(k)
                .help(format!(
                    "Snapshot {}\nBuild selection {}\nDigest {}",
                    record.id, record.identity.selection, record.digest
                ))
        }));
    let label = if status.snapshots.is_empty() {
        format!("No local snapshots for {}", short(&status.commit))
    } else {
        format!("Local snapshots for {}", short(&status.commit))
    };
    col()
        .gap(3.0)
        .child(
            text(label)
                .font(k.ty.small_medium)
                .color(k.c.text_base)
                .help(&status.commit),
        )
        .child(
            text(&path)
                .font(k.ty.small)
                .color(k.c.text_muted)
                .truncate()
                .help(&path),
        )
        .child(
            Button::new("Use automatic selection")
                .disabled(selected.is_none())
                .on_press(move |state: &mut State| {
                    state.flow_comparison.snapshots.choose(side, None)
                })
                .build(k),
        )
        .child(list)
}

pub(super) fn graph(state: &State, k: &Kit) -> El {
    let snapshots = &state.flow_comparison.snapshots;
    let body = col()
        .key("comparison-graph-panel")
        .grow(1.0)
        .min_h(0.0)
        .w_full()
        .gap(6.0)
        .px(10.0)
        .pb(8.0);
    let Some(page) = &snapshots.graph else {
        return body;
    };
    body.child(provenance::pair(&page.header, k))
        .child(graph_summary(page, k))
        .child(
            match snapshots
                .selected_change
                .and_then(|index| page.changes.get(index).map(|change| (index, change)))
            {
                Some((index, change)) => graph_detail(page, index, change, k),
                None => graph_records(page, k),
            },
        )
        .child(
            row()
                .gap(6.0)
                .center_y()
                .child(
                    Button::new("Previous")
                        .disabled(snapshots.pending() || !snapshots.previous_available())
                        .on_press(|state: &mut State| {
                            state.compare_snapshot_graphs(GraphNavigation::Previous)
                        })
                        .build(k)
                        .key("comparison-graph-previous"),
                )
                .child(
                    text(format!("{} of {}", page.offset + page.returned, page.total))
                        .font(k.ty.small)
                        .color(k.c.text_muted),
                )
                .child(
                    Button::new("Next")
                        .disabled(snapshots.pending() || page.next_offset.is_none())
                        .on_press(|state: &mut State| {
                            state.compare_snapshot_graphs(GraphNavigation::Next)
                        })
                        .build(k)
                        .key("comparison-graph-next"),
                ),
        )
}

fn graph_summary(page: &GraphPage, k: &Kit) -> El {
    let summary = page
        .summary
        .iter()
        .map(|(section, summary)| {
            format!(
                "{section} · {} entities, {} records · +{} −{} ~{}",
                summary.changed_entities,
                summary.records,
                summary.added,
                summary.removed,
                summary.replaced
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    text(format!(
        "{} · {}",
        if page.graph_equal {
            "Graphs match"
        } else {
            "Graphs differ"
        },
        if page.metadata_equal {
            "Metadata matches"
        } else {
            "Metadata differs"
        },
    ))
    .font(k.ty.small_medium)
    .color(k.c.text_base)
    .help(summary)
}

fn graph_records(page: &GraphPage, k: &Kit) -> El {
    let kit = k.clone();
    col()
        .min_h(0.0)
        .grow(1.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(
            col()
                .gap(4.0)
                .virtual_rows_version((&page.header.comparison_id, page.offset))
                .virtual_rows(
                    page.changes.len(),
                    35.0,
                    |_: &State, index| index.to_string().into(),
                    move |state: &State, index| {
                        let Some(page) = &state.flow_comparison.snapshots.graph else {
                            return rune::el();
                        };
                        graph_record(page, index, &kit)
                    },
                ),
        )
}

fn graph_record(page: &GraphPage, index: usize, k: &Kit) -> El {
    let Some(change) = page.changes.get(index) else {
        return rune::el();
    };
    let binding = page.header.comparison_id.clone();
    let offset = page.offset;
    row()
        .w_full()
        .gap(8.0)
        .center_y()
        .child(
            text(&change.summary)
                .font(k.ty.small)
                .color(k.c.text_base)
                .grow(1.0)
                .min_w(0.0)
                .truncate()
                .help(&change.summary),
        )
        .child(
            Button::new("View change")
                .variant(ButtonVariant::Outline)
                .on_press(move |state: &mut State| {
                    state
                        .flow_comparison
                        .snapshots
                        .select_graph_change(&binding, offset, index);
                })
                .build(k)
                .key(format!("comparison-graph-change-{}", offset + index)),
        )
}

fn graph_detail(page: &GraphPage, index: usize, change: &GraphChange, k: &Kit) -> El {
    let color = k.c.text_base;
    let binding = (&page.header.comparison_id, page.offset, index);
    col()
        .grow(1.0)
        .min_h(0.0)
        .gap(4.0)
        .key("comparison-graph-change-detail")
        .child(
            row()
                .gap(6.0)
                .child(
                    Button::new("Back to changes")
                        .variant(ButtonVariant::Outline)
                        .on_press(|state: &mut State| {
                            state.flow_comparison.snapshots.selected_change = None
                        })
                        .build(k)
                        .key("comparison-graph-change-back"),
                )
                .child(
                    Button::new("Copy JSON")
                        .on_press_tx(|state: &mut State, tx| {
                            let snapshots = &state.flow_comparison.snapshots;
                            if let Some(change) = snapshots
                                .selected_change
                                .and_then(|index| snapshots.graph.as_ref()?.changes.get(index))
                            {
                                tx.copy(change.document.clone());
                            }
                        })
                        .build(k)
                        .key("comparison-graph-change-copy"),
                ),
        )
        .child(
            col()
                .key(format!(
                    "comparison-graph-detail-scroll-{}-{}",
                    binding.0,
                    binding.1 + index
                ))
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .scroll_xy()
                .child(col().w(1250.0).virtual_rows_version(binding).virtual_rows(
                    change.line_count(),
                    20.0,
                    |_: &State, line| line.to_string().into(),
                    move |state: &State, line| {
                        state
                            .flow_comparison
                            .snapshots
                            .graph
                            .as_ref()
                            .and_then(|page| page.changes.get(index))
                            .and_then(|change| change.line(line))
                            .map_or_else(rune::el, |line| {
                                text(line)
                                    .font(crate::theme::mono(12.0, 400.0).lh(20.0))
                                    .color(color)
                                    .h(20.0)
                            })
                    },
                )),
        )
}

fn short(digest: &str) -> &str {
    digest.get(..12).unwrap_or(digest)
}
