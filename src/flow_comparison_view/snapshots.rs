//! One endpoint's snapshot status, actions, and retained commit variants.
mod failure;

use crate::{
    app::State,
    engine_client::flow_comparison::{EndpointStatus, SnapshotStatus, WorktreeFreshness},
    flow_comparison::{SnapshotSide, WorktreeSnapshot},
    freshness_workspace::{Status as FreshnessStatus, diagnostics::only_unmapped_inputs},
};
use codecallnavigator::artifact::{
    local_snapshots::SnapshotRecord, selection::Selection, verification::InputStatus,
};
use rune::{El, ScrollSpec, col, el, row, text};
use rune_kit::{Button, ButtonVariant, Kit};
use serde::Deserialize;

pub(super) fn view(state: &State, side: SnapshotSide, k: &Kit) -> El {
    let workspace = &state.flow_comparison;
    let snapshots = &workspace.snapshots;
    let refs = workspace.control_refs();
    let revision = side.reference(&refs);
    let worktree = revision == "WORKTREE";
    let scope = match side {
        SnapshotSide::Base => "comparison-snapshot-base",
        SnapshotSide::Head => "comparison-snapshot-after",
    };
    let status = status(state, side, revision);
    let buttons = SnapshotButtons {
        state,
        side,
        revision,
        scope,
    };
    let actions = buttons.actions(status.warning, k);
    let status = status.element(
        if worktree {
            "comparison-worktree-freshness"
        } else {
            scope
        },
        k,
    );
    let mut view = col()
        .gap(6.0)
        .w_full()
        .min_w(0.0)
        .child(buttons.bar(status, actions, k));
    if let Some(error) = snapshots
        .preparation_failure
        .as_ref()
        .filter(|_| snapshots.side == Some(side))
    {
        view = view.child(failure::details(error, k));
    }
    view
}

struct SnapshotButtons<'a> {
    state: &'a State,
    side: SnapshotSide,
    revision: &'a str,
    scope: &'static str,
}

impl SnapshotButtons<'_> {
    fn actions(&self, warning: bool, k: &Kit) -> El {
        let Self {
            state,
            side,
            revision,
            scope,
        } = *self;
        let workspace = &state.flow_comparison;
        let disabled = state.runtime.is_none()
            || workspace.snapshots.pending()
            || workspace.worktree_refresh.pending()
            || revision.is_empty();
        let can_prepare = revision != "WORKTREE" || state.can_refresh_comparison_worktree();
        row()
            .gap(6.0)
            .center_y()
            .child(
                Button::new("Find snapshot")
                    .disabled(disabled)
                    .on_press(move |state: &mut State| state.lookup_comparison_snapshot(side))
                    .build(k)
                    .key(format!("{scope}-find")),
            )
            .child(
                Button::new("Prepare snapshot")
                    .variant(if warning {
                        ButtonVariant::Contrast
                    } else {
                        ButtonVariant::Outline
                    })
                    .disabled(disabled || !can_prepare)
                    .on_press(move |state: &mut State| state.prepare_comparison_snapshot(side))
                    .build(k)
                    .key(format!("{scope}-prepare")),
            )
    }

    fn bar(&self, status: El, actions: El, k: &Kit) -> El {
        let workspace = &self.state.flow_comparison;
        let sidebar_width = if workspace.changes.sidebar.collapsed {
            38.0
        } else {
            280.0
        };
        let compact = workspace.previews.interaction.window[0] - sidebar_width < 800.0;
        let mut line = row().w_full().gap(6.0).center_y().child(status);
        let Some(cancel_side) = cancel_side(workspace) else {
            return if compact {
                col().gap(6.0).child(line).child(actions)
            } else {
                line.child(actions)
            };
        };
        if cancel_side == self.side {
            line = line.child(
                Button::new("Cancel")
                    .on_press(State::cancel_comparison_action)
                    .build(k)
                    .key("comparison-cancel"),
            );
        }
        col().gap(6.0).child(line.min_h(28.0)).child(actions)
    }
}

fn cancel_side(workspace: &crate::flow_comparison::Workspace) -> Option<SnapshotSide> {
    if workspace.snapshots.pending() {
        return workspace.snapshots.side;
    }
    let loading = matches!(workspace.phase, crate::flow_comparison::Phase::Loading);
    (workspace.worktree_refresh.pending() || loading).then_some(SnapshotSide::Head)
}

struct Status {
    label: String,
    detail: String,
    warning: bool,
}

impl Status {
    fn new(label: impl Into<String>, detail: impl Into<String>, warning: bool) -> Self {
        Self {
            label: label.into(),
            detail: detail.into(),
            warning,
        }
    }

    fn message(label: &str, warning: bool) -> Self {
        Self::new(label, label, warning)
    }

    fn element(self, key: &str, k: &Kit) -> El {
        text(self.label)
            .font(k.ty.small)
            .color(if self.warning {
                k.c.warning.fg
            } else {
                k.c.text_muted
            })
            .grow(1.0)
            .min_w(0.0)
            .truncate()
            .help(self.detail)
            .key(key)
    }
}

fn status(state: &State, side: SnapshotSide, revision: &str) -> Status {
    let workspace = &state.flow_comparison;
    let snapshots = &workspace.snapshots;
    if let Some(progress) = action_message(state, side, revision) {
        return Status::message(progress, false);
    }
    if revision == "WORKTREE" {
        return workspace
            .worktree_snapshot()
            .or_else(|| snapshots.worktree.clone())
            .map_or_else(
                || bound_status(workspace, side),
                |snapshot| worktree_status(state, snapshot),
            );
    }
    let empty = snapshots
        .lookup(side, revision)
        .is_some_and(|lookup| lookup.status.snapshots.is_empty());
    if empty {
        Status::new("No local snapshots", revision, true)
    } else {
        bound_status(workspace, side)
    }
}

fn action_message<'a>(state: &'a State, side: SnapshotSide, revision: &str) -> Option<&'a str> {
    let workspace = &state.flow_comparison;
    let refresh = &workspace.worktree_refresh;
    let message = if refresh.showing_progress() {
        &state.status
    } else {
        &refresh.message
    };
    if revision == "WORKTREE" && !message.is_empty() {
        return Some(message);
    }
    let snapshots = &workspace.snapshots;
    (snapshots.side == Some(side) && !snapshots.message.is_empty())
        .then_some(snapshots.message.as_str())
}

fn worktree_status(state: &State, snapshot: WorktreeSnapshot) -> Status {
    let report = match snapshot {
        WorktreeSnapshot::Missing => return Status::message("No snapshot for Worktree", true),
        WorktreeSnapshot::Saved(None) => {
            return Status::message("Worktree snapshot freshness unknown", true);
        }
        WorktreeSnapshot::Saved(Some(report)) => report,
    };
    let observed = report
        .evidence
        .as_ref()
        .zip(state.flow_comparison.worktree_scope())
        .and_then(|(evidence, (project, roots))| {
            state
                .freshness
                .observed_display_status(project, roots, &evidence.digest)
        });
    reported_worktree_status(&report, observed)
}

fn reported_worktree_status(
    report: &WorktreeFreshness,
    observed: Option<&FreshnessStatus>,
) -> Status {
    let (label, warning) = match (report.input_status, observed) {
        (InputStatus::Stale, _) | (_, Some(FreshnessStatus::Observed(InputStatus::Stale))) => {
            ("Worktree snapshot stale", true)
        }
        (InputStatus::Match, Some(FreshnessStatus::Observed(InputStatus::Unverifiable))) => {
            ("Worktree snapshot freshness unknown", true)
        }
        (InputStatus::Match, _) => ("Snapshot ready", false),
        (InputStatus::Unverifiable, Some(FreshnessStatus::Idle))
            if report.evidence.as_ref().is_some_and(|evidence| {
                evidence.contract_valid
                    && only_unmapped_inputs(&evidence.reasons, evidence.omitted_reasons)
            }) =>
        {
            ("Snapshot ready", false)
        }
        _ => ("Worktree snapshot freshness unknown", true),
    };
    Status::message(label, warning)
}

fn bound_status(workspace: &crate::flow_comparison::Workspace, side: SnapshotSide) -> Status {
    let Some(catalog) = &workspace.catalog else {
        return unbound_status(workspace, side);
    };
    let (endpoint, status) = match side {
        SnapshotSide::Base => (&catalog.header.base, &catalog.endpoints.before),
        SnapshotSide::Head => (&catalog.header.head, &catalog.endpoints.after),
    };
    match status {
        EndpointStatus::Available => Status::new(
            "Snapshot ready",
            format!("Artifact {}\n{}", endpoint.digest, endpoint.path),
            false,
        ),
        EndpointStatus::Unavailable(error) => {
            Status::new("Snapshot unavailable", &error.message, true)
        }
    }
}

fn unbound_status(workspace: &crate::flow_comparison::Workspace, side: SnapshotSide) -> Status {
    use crate::flow_comparison::Phase;
    if side == SnapshotSide::Base && !workspace.controls.message.is_empty() {
        return Status::message(&workspace.controls.message, true);
    }
    match &workspace.phase {
        Phase::Loading => Status::message("Checking snapshots…", false),
        Phase::ReloadRequired(message) => Status::new("Use Find snapshot to update", message, true),
        Phase::Failed(error) => Status::new("Snapshot unavailable", error, true),
        _ => Status::message("", false),
    }
}

pub(super) fn layers(state: &State, k: &Kit) -> Vec<El> {
    let workspace = &state.flow_comparison;
    let refs = workspace.control_refs();
    let Some(side) = workspace.snapshots.variant_picker else {
        return Vec::new();
    };
    let Some(lookup) = workspace.snapshots.lookup(side, side.reference(&refs)) else {
        return Vec::new();
    };
    let close = |s: &mut State| s.flow_comparison.snapshots.variant_picker = None;
    let title = match side {
        SnapshotSide::Base => "Before snapshots",
        SnapshotSide::Head => "After snapshots",
    };
    let panel = col()
        .key("comparison-snapshot-variants")
        .label(title)
        .role("dialog")
        .w(620.0_f32.min((workspace.previews.interaction.window[0] - 24.0).max(320.0)))
        .padding(12.0)
        .gap(8.0)
        .radius(8.0)
        .fill(k.c.layer_01)
        .border(1.0, k.c.std_border)
        .child(
            row()
                .center_y()
                .gap(8.0)
                .child(
                    text(title)
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base)
                        .grow(1.0),
                )
                .child(
                    Button::new("Done")
                        .on_press(close)
                        .build(k)
                        .key("comparison-snapshot-variants-done"),
                ),
        )
        .child(variants(
            side,
            &lookup.status,
            lookup.selected.as_deref(),
            k,
        ))
        .focus_return(match side {
            SnapshotSide::Base => "comparison-snapshot-base-find",
            SnapshotSide::Head => "comparison-snapshot-after-find",
        })
        .modal(true, close)
        .modal_click_outside(true);
    vec![
        el().absolute(0.0, 0.0)
            .w_full()
            .h_full()
            .fill(k.c.scrim)
            .a11y_hidden(),
        col()
            .absolute(0.0, 0.0)
            .w_full()
            .h_full()
            .pt(105.0)
            .items_center()
            .child(panel),
    ]
}

fn variants(side: SnapshotSide, status: &SnapshotStatus, selected: Option<&str>, k: &Kit) -> El {
    let list = col()
        .gap(3.0)
        .max_h(220.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .children(status.snapshots.iter().map(|record| {
            let id = record.id.clone();
            let is_selected = selected == Some(id.as_str());
            let (summary, details) = variant_settings(record);
            let prefix = if is_selected { "Selected · " } else { "" };
            Button::new(format!("{prefix}{summary} · {}", short(&id)))
                .disabled(is_selected)
                .on_press(move |state: &mut State| {
                    state.select_comparison_snapshot(side, Some(id.clone()))
                })
                .build(k)
                .w_full()
                .min_w(0.0)
                .help(format!(
                    "Snapshot {}\n{details}\nDigest {}\n{}",
                    record.id,
                    record.digest,
                    status.cache_path.display()
                ))
        }));
    col()
        .gap(3.0)
        .child(
            Button::new("Use automatic selection")
                .disabled(selected.is_none())
                .on_press(move |state: &mut State| state.select_comparison_snapshot(side, None))
                .build(k),
        )
        .child(list)
}

#[derive(Deserialize)]
struct VariantSettings {
    builds: Option<Selection>,
    #[serde(default)]
    analyzers: Vec<Analyzer>,
}

#[derive(Deserialize)]
struct Analyzer {
    path: String,
    kind: String,
    language: String,
    backend: String,
    version: String,
    mode: String,
}

fn variant_settings(record: &SnapshotRecord) -> (String, String) {
    let Ok(VariantSettings {
        builds: Some(builds),
        analyzers,
    }) = serde_json::from_str(&record.identity.selection)
    else {
        return ("Saved graph".into(), record.identity.selection.clone());
    };
    let mut lines = builds.targets.iter().map(|target| {
        let path = target.path.strip_prefix("@repo/").unwrap_or(&target.path);
        let path = if path.is_empty() { "." } else { path };
        let build = format!("{path} ({})", target.kind);
        let choices = analyzers
            .iter()
            .filter(|analyzer| {
                analyzer.path == target.path
                    && analyzer.kind == target.kind
                    && analyzer.language == target.language
            })
            .map(|analyzer| {
                format!(
                    "{} {} ({})",
                    analyzer.backend, analyzer.version, analyzer.mode
                )
            })
            .collect::<Vec<_>>();
        if choices.is_empty() {
            format!("{build} · {}", target.language)
        } else {
            format!("{build} · {}", choices.join(", "))
        }
    });
    let Some(first) = lines.next() else {
        return ("Saved graph".into(), record.identity.selection.clone());
    };
    let total = builds.targets.len();
    let summary = if total > 1 {
        format!(
            "{} + {}",
            crate::analysis_feedback::bounded(&first, 64),
            total - 1
        )
    } else {
        crate::analysis_feedback::bounded(&first, 64)
    };
    let details = std::iter::once(first)
        .chain(lines)
        .collect::<Vec<_>>()
        .join("\n");
    (summary, details)
}

fn short(digest: &str) -> &str {
    digest.get(..12).unwrap_or(digest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_client::flow_comparison::WorktreeFreshnessEvidence;
    use codecallnavigator::artifact::verification::Reason;

    #[test]
    fn worktree_status_shares_freshness_warnings_with_the_call_graph() {
        let mut report = WorktreeFreshness {
            input_status: InputStatus::Unverifiable,
            evidence: Some(WorktreeFreshnessEvidence {
                digest: "generation".into(),
                contract_valid: true,
                omitted_reasons: 0,
                reasons: vec![Reason::new("unmapped_input", None)],
            }),
        };
        let ready = reported_worktree_status(&report, Some(&FreshnessStatus::Idle));
        assert_eq!(ready.label, "Snapshot ready");
        assert!(!ready.warning);
        assert!(reported_worktree_status(&report, None).warning);

        report
            .evidence
            .as_mut()
            .unwrap()
            .reasons
            .push(Reason::new("unavailable_root", None));
        assert!(reported_worktree_status(&report, Some(&FreshnessStatus::Idle)).warning);
        report.input_status = InputStatus::Stale;
        assert!(reported_worktree_status(&report, Some(&FreshnessStatus::Idle)).warning);

        report.input_status = InputStatus::Match;
        let stale = FreshnessStatus::Observed(InputStatus::Stale);
        let status = reported_worktree_status(&report, Some(&stale));
        assert_eq!(status.label, "Worktree snapshot stale");
        assert!(status.warning);
    }
}
