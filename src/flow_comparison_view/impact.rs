//! Show concrete changed ranges and the baseline call paths attributed to them.
use super::provenance;
use crate::{
    app::State,
    engine_client::flow_comparison::{ImpactPreview, ImpactRow},
};
use rune::{El, GrowthSpec, ScrollSpec, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit};

pub(super) fn button(state: &State, k: &Kit) -> El {
    let workspace = &state.flow_comparison;
    let help = if workspace.impact.checking_support() {
        "Checking CLI support"
    } else if workspace.impact.supported == Some(false) {
        "Update the configured CLI to preview changed-symbol impact"
    } else {
        "Trace potential impact from the saved base graph"
    };
    Button::new("Preview changed-symbol impact")
        .variant(ButtonVariant::Outline)
        .disabled(!workspace.can_preview_impact())
        .on_press(State::preview_changed_symbol_impact)
        .build(k)
        .help(help)
        .key("comparison-impact-preview")
}

pub(super) fn view(state: &State, k: &Kit) -> El {
    let impact = &state.flow_comparison.impact;
    let mut pane = col()
        .key("comparison-impact-panel")
        .grow(1.0)
        .min_h(0.0)
        .w_full()
        .px(10.0)
        .pb(8.0)
        .gap(6.0);
    if !impact.message.is_empty() {
        pane = pane.child(warning(&impact.message, k).key("comparison-impact-status"));
    }
    let Some(report) = &impact.report else {
        return pane;
    };
    pane = pane.child(endpoints(report, k)).child(coverage(report, k));
    if report.rows.is_empty() {
        return pane.child(line("No changed ranges in this preview", k));
    }
    pane.child(records(report, k))
}

fn endpoints(report: &ImpactPreview, k: &Kit) -> El {
    let head = &report.head;
    let after = if let Some(commit) = &head.commit {
        format!(
            "{} · commit {}",
            head.requested_ref,
            commit.get(..12).unwrap_or(commit)
        )
    } else {
        "Tracked working tree".into()
    };
    row()
        .gap(12.0)
        .w_full()
        .child(provenance::endpoint("Baseline", &report.base, k))
        .child(
            text(after)
                .font(k.ty.small_medium)
                .color(k.c.text_base)
                .grow(1.0)
                .min_w(0.0)
                .truncate()
                .help(head.commit.as_deref().unwrap_or(&head.requested_ref)),
        )
}

fn coverage(report: &ImpactPreview, k: &Kit) -> El {
    let coverage = &report.coverage;
    let mut description = format!(
        "Potential impact from the base graph · {} changed paths · {} mapped ranges · {} unmapped ranges",
        coverage.changed_paths, coverage.mapped_ranges, coverage.unmapped_ranges
    );
    if coverage.tracked_only {
        description.push_str(" · tracked changes only");
    }
    let mut summary = col().gap(3.0).child(line(&description, k));
    if coverage.truncated {
        summary = summary.child(warning(
            "Preview limit reached. Some ranges or call paths are omitted.",
            k,
        ));
    }
    summary
}

fn records(report: &ImpactPreview, k: &Kit) -> El {
    let kit = k.clone();
    let rows = col().growth(GrowthSpec::default()).virtual_rows(
        report.rows.len(),
        30.0,
        |_: &State, index| index.to_string().into(),
        move |state: &State, index| {
            state
                .flow_comparison
                .impact
                .report
                .as_ref()
                .and_then(|report| report.rows.get(index))
                .map_or_else(rune::el, |row| record(row, &kit))
        },
    );
    col()
        .key("comparison-impact-records")
        .grow(1.0)
        .min_h(0.0)
        .w_full()
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(rows)
}

fn record(record: &ImpactRow, k: &Kit) -> El {
    match record {
        ImpactRow::Range { label, unresolved } => {
            let header = line(label, k).font(k.ty.small_medium);
            if let Some(reason) = unresolved {
                col().child(header).child(warning(reason, k).pl(12.0))
            } else {
                header
            }
        }
        ImpactRow::Symbol { id, label } => line(label, k).pl(12.0).help(id),
        ImpactRow::Caller {
            label,
            path,
            evidence,
            possible,
        } => {
            let description = format!(
                "{} {label}",
                if *possible {
                    "Possible caller"
                } else {
                    "Caller"
                }
            );
            row()
                .gap(8.0)
                .pl(24.0)
                .w_full()
                .child(if *possible {
                    warning(&description, k)
                } else {
                    line(&description, k)
                })
                .child(line(path, k).grow(1.0).min_w(0.0).help(evidence))
        }
    }
}

fn line(value: &str, k: &Kit) -> El {
    text(value)
        .font(k.ty.small)
        .color(k.c.text_base)
        .truncate()
        .help(value)
}

fn warning(value: &str, k: &Kit) -> El {
    line(value, k).color(k.c.warning.fg)
}
