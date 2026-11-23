//! Exercise desktop comparison with the real engine, Rune, and an isolated user cache.
use super::{
    GraphNavigation, SnapshotSide,
    tests::{git, isolate_cli_cache, settle, state},
};
use crate::app::{State, TargetChoice};
use crate::engine_client::flow_comparison::{ImpactPreview, ImpactRow};
use rune::{ElementInfo, Env, Runtime};
use std::{fs, path::Path, sync::Arc};

#[test]
#[ignore = "requires installed TypeScript and Node"]
fn desktop_prepares_selects_and_pages_cached_graphs_and_flows() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap().join("repo");
    commit_revisions(&root);
    std::os::unix::fs::symlink(
        std::env::var_os("CODECALLNAVIGATOR_TS_NODE_MODULES")
            .expect("TypeScript fixture dependencies"),
        root.join("selected/node_modules"),
    )
    .unwrap();

    let mut state = state(&root);
    let cache = root.with_file_name("cache");
    isolate_cli_cache(Arc::make_mut(state.runtime.as_mut().unwrap()), &cache);

    check_impact_support(&mut state);
    state.lookup_comparison_snapshot(SnapshotSide::Base);
    settle(&mut state);
    assert!(
        state
            .flow_comparison
            .snapshots
            .lookup(SnapshotSide::Base, "refs/tags/base")
            .is_some()
    );
    state.prepare_comparison_snapshot(SnapshotSide::Base);
    assert_eq!(
        state.flow_comparison.snapshots.message,
        super::SELECT_BUILDS
    );
    assert!(!state.flow_comparison.snapshots.pending());
    state.targets = crate::discovery::discover(&root)
        .unwrap()
        .into_iter()
        .map(|target| TargetChoice {
            selected: target.path == root.join("selected/tsconfig.json"),
            target,
        })
        .collect();
    assert_eq!(state.targets.len(), 2);
    let mut runtime = Runtime::new(
        state,
        crate::view::view,
        (1200.0, 800.0),
        Env::new(true, 1.0),
    );
    for side in [SnapshotSide::Base, SnapshotSide::Head] {
        assert_prepares_picked_target(&mut runtime.state, side, &cache);
        if matches!(side, SnapshotSide::Base) {
            assert_impact_without_head_snapshot(&mut runtime);
        }
    }
    assert!(!root.join(".codecallnavigator").exists());

    assert_graph_pagination(&mut runtime);

    let state = &mut runtime.state;
    state
        .flow_comparison
        .revisions
        .set_for_test(SnapshotSide::Base, "refs/tags/base");
    state.load_flow_comparison();
    settle(state);
    let header = state.flow_comparison.header().unwrap();
    assert!(header.base.snapshot.is_some() && header.head.snapshot.is_some());
    assert!(state.flow_comparison.catalog.as_ref().unwrap().complete);
    assert!(state.graph.is_none());

    assert_preparation_failure(state);
}

fn select_test_revisions(state: &mut State) {
    let revisions = &mut state.flow_comparison.revisions;
    revisions.set_for_test(SnapshotSide::Base, "refs/tags/base");
    revisions.set_for_test(SnapshotSide::Head, "refs/tags/head");
}

fn assert_prepares_picked_target(state: &mut State, side: SnapshotSide, cache: &Path) {
    state.lookup_comparison_snapshot(side);
    settle(state);
    let refs = state.flow_comparison.control_refs();
    let lookup = state
        .flow_comparison
        .snapshots
        .lookup(side, side.reference(&refs))
        .unwrap();
    assert!(lookup.status.snapshots.is_empty());
    assert!(lookup.status.cache_path.starts_with(cache));
    state.prepare_comparison_snapshot(side);
    settle(state);
    assert!(
        state
            .flow_comparison
            .snapshots
            .message
            .starts_with("Prepared snapshot for "),
        "{}",
        state.flow_comparison.snapshots.message
    );
    let lookup = state
        .flow_comparison
        .snapshots
        .lookup(side, side.reference(&refs))
        .unwrap();
    assert_eq!(
        lookup.selected.as_ref(),
        Some(&lookup.status.snapshots[0].id)
    );
    let selection: serde_json::Value =
        serde_json::from_str(&lookup.status.snapshots[0].identity.selection).unwrap();
    assert_eq!(
        selection["builds"]["targets"],
        serde_json::json!([{
            "path": "@repo/selected/tsconfig.json",
            "kind": "typescript",
            "language": "typescript"
        }])
    );
}

fn assert_preparation_failure(state: &mut State) {
    state
        .flow_comparison
        .revisions
        .set_for_test(SnapshotSide::Base, "no-such-revision");
    state.prepare_comparison_snapshot(SnapshotSide::Base);
    settle(state);
    assert!(
        state
            .flow_comparison
            .snapshots
            .message
            .starts_with("Snapshot preparation failed"),
        "{}",
        state.flow_comparison.snapshots.message
    );
    assert!(
        state
            .flow_comparison
            .snapshots
            .preparation_failure
            .is_some()
    );
    state
        .flow_comparison
        .revisions
        .set_for_test(SnapshotSide::Base, "refs/tags/base");
    state.lookup_comparison_snapshot(SnapshotSide::Base);
    assert!(
        state
            .flow_comparison
            .snapshots
            .preparation_failure
            .is_none()
    );
    settle(state);
    let lookup = state
        .flow_comparison
        .snapshots
        .lookup(SnapshotSide::Base, "refs/tags/base")
        .unwrap();
    assert_eq!(
        lookup.selected.as_ref(),
        Some(&lookup.status.snapshots[0].id)
    );
}

fn check_impact_support(state: &mut State) {
    state.show_flow_comparison();
    select_test_revisions(state);
    let start = std::time::Instant::now();
    while state.flow_comparison.impact.checking_support() {
        assert!(start.elapsed() < std::time::Duration::from_secs(15));
        state.flow_comparison.poll();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    state.flow_comparison.poll();
    assert_eq!(state.flow_comparison.impact.supported, Some(true));
    state.preview_changed_symbol_impact();
    settle(state);
    assert!(state.flow_comparison.impact.report.is_none());
    assert!(state.flow_comparison.impact.message.contains("snapshot"));
}

fn assert_impact_without_head_snapshot(runtime: &mut Runtime<State>) {
    runtime.invalidate();
    runtime.frame(0.0);
    click(runtime, "comparison-impact-preview");
    settle(&mut runtime.state);
    runtime.invalidate();
    runtime.frame(1.0 / 60.0);

    let report = runtime
        .state
        .flow_comparison
        .impact
        .report
        .as_ref()
        .unwrap_or_else(|| {
            panic!("{}", runtime.state.flow_comparison.impact.message);
        });
    assert!(report.base.snapshot.is_some());
    assert!(report.coverage.tracked_only && report.coverage.mapped_ranges > 0);
    assert_impact_rows(report, &runtime.elements());

    let state = &mut runtime.state;
    state
        .flow_comparison
        .revisions
        .set_for_test(SnapshotSide::Head, "HEAD");
    state.flow_comparison.poll();
    assert!(state.flow_comparison.impact.report.is_none());
    state
        .flow_comparison
        .revisions
        .set_for_test(SnapshotSide::Head, "refs/tags/head");
}

fn assert_impact_rows(report: &ImpactPreview, elements: &[ElementInfo]) {
    let symbol = report.rows.iter().find_map(|row| match row {
        ImpactRow::Symbol { label, .. } if label.contains("value") => Some(label.as_str()),
        _ => None,
    });
    let caller = report.rows.iter().find_map(|row| match row {
        ImpactRow::Caller {
            label, possible, ..
        } if label.contains("caller") => {
            let prefix = if *possible {
                "Possible caller"
            } else {
                "Caller"
            };
            Some(format!("{prefix} {label}"))
        }
        _ => None,
    });
    for expected in [
        symbol.expect("changed symbol"),
        caller.as_deref().expect("baseline caller"),
    ] {
        assert_visible_text(elements, "/comparison-impact-records/", expected);
    }
}

fn assert_graph_pagination(runtime: &mut Runtime<State>) {
    runtime
        .state
        .compare_snapshot_graphs(GraphNavigation::First);
    settle(&mut runtime.state);
    assert_graph_change_detail(runtime);

    let state = &mut runtime.state;
    let first = state.flow_comparison.snapshots.graph.as_ref().unwrap();
    assert_eq!(first.offset, 0);
    assert!(!first.graph_equal);
    assert!(first.header.base.snapshot.is_some() && first.header.head.snapshot.is_some());
    let following = first.next_offset.unwrap();
    let binding = first.header.comparison_id.clone();
    state.compare_snapshot_graphs(GraphNavigation::Next);
    settle(state);
    let next = state.flow_comparison.snapshots.graph.as_ref().unwrap();
    assert_eq!(next.offset, following);
    assert_eq!(next.header.comparison_id, binding);

    state
        .flow_comparison
        .revisions
        .set_for_test(SnapshotSide::Base, "no-such-revision");
    state.compare_snapshot_graphs(GraphNavigation::Reload);
    settle(state);
    let reloaded = state.flow_comparison.snapshots.graph.as_ref().unwrap();
    assert_eq!(reloaded.offset, 0);
    assert_eq!(reloaded.header.comparison_id, binding);
}

fn assert_graph_change_detail(runtime: &mut Runtime<State>) {
    runtime.invalidate();
    runtime.frame(1.0 / 60.0);
    click(runtime, "comparison-graph-change-0");

    let snapshots = &runtime.state.flow_comparison.snapshots;
    assert_eq!(snapshots.selected_change, Some(0));
    let first = &snapshots.graph.as_ref().expect("first graph page").changes[0];
    let json = first.line(1).expect("first JSON property");
    assert_visible_text(
        &runtime.elements(),
        "/comparison-graph-detail-scroll-",
        json,
    );

    click(runtime, "comparison-graph-change-back");
    assert!(
        runtime
            .state
            .flow_comparison
            .snapshots
            .selected_change
            .is_none()
    );
    assert!(runtime.find("comparison-graph-change-0").is_some());
}

fn assert_visible_text(elements: &[ElementInfo], path: &str, text: &str) {
    assert!(
        elements.iter().any(|element| {
            element.path.contains(path)
                && element.text.as_deref() == Some(text)
                && element.visible[2] > 0.0
                && element.visible[3] > 0.0
        }),
        "Expected visible text within {path}: {text}"
    );
}

fn click(runtime: &mut Runtime<State>, key: &str) {
    let [x, y, width, height] = runtime
        .elements()
        .iter()
        .find(|element| element.key.as_deref() == Some(key) && !element.exiting)
        .unwrap_or_else(|| panic!("Missing control {key}"))
        .visible;
    assert!(width > 0.0 && height > 0.0, "Control is clipped: {key}");
    let (x, y) = (x + width / 2.0, y + height / 2.0);
    runtime.pointer_down(x, y);
    runtime.frame(1.0 / 60.0);
    runtime.pointer_up(x, y);
    runtime.frame(1.0 / 60.0);
}

fn commit_revisions(root: &Path) {
    fs::create_dir(root).unwrap();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.name", "Desktop snapshot test"]);
    git(root, &["config", "user.email", "snapshot@example.invalid"]);
    fs::write(root.join("tsconfig.json"), r#"{"files":["main.ts"]}"#).unwrap();
    fs::create_dir(root.join("selected")).unwrap();
    fs::write(
        root.join("selected/tsconfig.json"),
        r#"{"files":["main.ts"]}"#,
    )
    .unwrap();
    let before =
        "export function value() { return 1; }\nexport function caller() { return value(); }\n";
    fs::write(root.join("main.ts"), "export function deselected() {}\n").unwrap();
    fs::write(root.join("selected/main.ts"), before).unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "baseline"]);
    git(root, &["tag", "base"]);
    let additions: String = (0..65)
        .map(|id| format!("export function added{id}() {{ return {id}; }}\n"))
        .collect();
    let after = before.replace("return 1", "return 2");
    fs::write(
        root.join("selected/main.ts"),
        format!("{after}\n{additions}"),
    )
    .unwrap();
    git(root, &["add", "selected/main.ts"]);
    git(root, &["commit", "-qm", "head"]);
    git(root, &["tag", "head"]);
}
