//! Exercise the desktop controller with the real engine and an isolated user cache.
use super::{
    GraphNavigation, SnapshotSide,
    tests::{git, isolate_cli_cache, settle, state},
};
use crate::app::State;
use crate::engine_client::flow_comparison::ImpactRow;
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
        root.join("node_modules"),
    )
    .unwrap();

    let mut state = state(&root);
    let cache = root.with_file_name("cache");
    isolate_cli_cache(Arc::make_mut(state.runtime.as_mut().unwrap()), &cache);

    state.flow_comparison.controls.base = rune::TextEdit::new("refs/tags/base");
    state.flow_comparison.controls.head = rune::TextEdit::new("refs/tags/head");
    state.flow_comparison.snapshots.target = rune::TextEdit::new("tsconfig.json");
    check_impact_support(&mut state);
    for side in [SnapshotSide::Base, SnapshotSide::Head] {
        state.lookup_comparison_snapshot(side);
        settle(&mut state);
        let refs = state.flow_comparison.control_refs();
        let lookup = state
            .flow_comparison
            .snapshots
            .lookup(side, side.reference(&refs))
            .unwrap();
        assert!(lookup.status.snapshots.is_empty());
        assert!(lookup.status.cache_path.starts_with(&cache));
        state.prepare_comparison_snapshot(side);
        settle(&mut state);
        assert!(
            state.flow_comparison.snapshots.message.is_empty(),
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
        if matches!(side, SnapshotSide::Base) {
            assert_impact_without_head_snapshot(&mut state);
        }
    }
    assert!(!root.join(codecallnavigator::artifact::GRAPH_PATH).exists());

    assert_graph_pagination(&mut state);

    state.flow_comparison.controls.base = rune::TextEdit::new("refs/tags/base");
    state.load_flow_comparison();
    settle(&mut state);
    let header = state.flow_comparison.header().unwrap();
    assert!(header.base.snapshot.is_some() && header.head.snapshot.is_some());
    assert!(state.flow_comparison.catalog.as_ref().unwrap().complete);
    assert!(state.graph.is_none());
}

fn check_impact_support(state: &mut State) {
    state.show_flow_comparison();
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

fn assert_impact_without_head_snapshot(state: &mut State) {
    state.preview_changed_symbol_impact();
    settle(state);
    let report = state
        .flow_comparison
        .impact
        .report
        .as_ref()
        .unwrap_or_else(|| {
            panic!("{}", state.flow_comparison.impact.message);
        });
    assert!(report.base.snapshot.is_some());
    assert!(report.coverage.tracked_only && report.coverage.mapped_ranges > 0);
    assert!(
        report.rows.iter().any(|row| {
            matches!(row, ImpactRow::Symbol { label, .. } if label.contains("value"))
        })
    );
    assert!(
        report.rows.iter().any(|row| {
            matches!(row, ImpactRow::Caller { label, .. } if label.contains("caller"))
        })
    );
    state.flow_comparison.controls.head = rune::TextEdit::new("HEAD");
    state.flow_comparison.poll();
    assert!(state.flow_comparison.impact.report.is_none());
    state.flow_comparison.controls.head = rune::TextEdit::new("refs/tags/head");
}

fn assert_graph_pagination(state: &mut State) {
    state.compare_snapshot_graphs(GraphNavigation::First);
    settle(state);
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

    state.flow_comparison.controls.base = rune::TextEdit::new("no-such-revision");
    state.compare_snapshot_graphs(GraphNavigation::Reload);
    settle(state);
    let reloaded = state.flow_comparison.snapshots.graph.as_ref().unwrap();
    assert_eq!(reloaded.offset, 0);
    assert_eq!(reloaded.header.comparison_id, binding);
}

fn commit_revisions(root: &Path) {
    fs::create_dir(root).unwrap();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.name", "Desktop snapshot test"]);
    git(root, &["config", "user.email", "snapshot@example.invalid"]);
    fs::write(root.join("tsconfig.json"), r#"{"files":["main.ts"]}"#).unwrap();
    let before =
        "export function value() { return 1; }\nexport function caller() { return value(); }\n";
    fs::write(root.join("main.ts"), before).unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "baseline"]);
    git(root, &["tag", "base"]);
    let additions: String = (0..65)
        .map(|id| format!("export function added{id}() {{ return {id}; }}\n"))
        .collect();
    let after = before.replace("return 1", "return 2");
    fs::write(root.join("main.ts"), format!("{after}\n{additions}")).unwrap();
    git(root, &["add", "main.ts"]);
    git(root, &["commit", "-qm", "head"]);
    git(root, &["tag", "head"]);
}
