//! Exercise desktop comparison with the real engine, Rune, and an isolated user cache.
use super::{
    SnapshotSide,
    tests::{git, isolate_cli_cache, settle, state},
};
use crate::app::{State, TargetChoice};
use codecallnavigator::query::Query;
use std::sync::atomic::AtomicBool;
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

    state.show_flow_comparison();
    select_test_revisions(&mut state);
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
    for side in [SnapshotSide::Base, SnapshotSide::Head] {
        assert_prepares_picked_target(&mut state, side, &cache);
        if matches!(side, SnapshotSide::Base) {
            assert_impact_without_head_snapshot(&state);
        }
    }
    assert!(!root.join(".codecallnavigator").exists());

    assert_graph_pagination(&state);
    state.load_flow_comparison();
    settle(&mut state);
    let header = state.flow_comparison.header().unwrap();
    assert!(header.base.snapshot.is_some() && header.head.snapshot.is_some());
    assert!(state.flow_comparison.catalog.as_ref().unwrap().complete);
    assert!(state.graph.is_none());

    assert_preparation_failure(&mut state);
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

fn assert_impact_without_head_snapshot(state: &State) {
    let context = state.library_comparison_context().unwrap();
    let refs = state.flow_comparison.control_refs();
    let result: serde_json::Value = context
        .request(
            Query::ImpactPreview {
                base: refs.base,
                head: refs.head,
                base_snapshot: refs.base_snapshot,
            },
            &AtomicBool::new(false),
        )
        .unwrap();
    assert!(result["base"]["snapshot"].is_object());
    assert_eq!(result["coverage"]["tracked_only"], true);
    assert!(result["coverage"]["mapped_ranges"].as_u64().unwrap() > 0);
    let records = result["records"].as_array().unwrap();
    let symbols: Vec<_> = records
        .iter()
        .flat_map(|record| record["symbols"].as_array().unwrap())
        .collect();
    assert!(symbols.iter().any(|symbol| {
        symbol["label"]
            .as_str()
            .is_some_and(|label| label.contains("value"))
    }));
    assert!(symbols.iter().any(|symbol| {
        symbol["callers"].as_array().unwrap().iter().any(|caller| {
            caller["label"]
                .as_str()
                .is_some_and(|label| label.contains("caller"))
        })
    }));
}

fn assert_graph_pagination(state: &State) {
    let context = state.library_comparison_context().unwrap();
    let refs = state.flow_comparison.control_refs();
    let cancel = AtomicBool::new(false);
    let request = |offset, expected_comparison| Query::Compare {
        base: refs.base.clone(),
        head: refs.head.clone(),
        base_snapshot: refs.base_snapshot.clone(),
        head_snapshot: refs.head_snapshot.clone(),
        offset,
        limit: 50,
        expected_comparison,
    };
    let first: serde_json::Value = context.request(request(0, None), &cancel).unwrap();
    assert_eq!(first["offset"], 0);
    assert_eq!(first["graph_equal"], false);
    assert!(first["base"]["snapshot"].is_object() && first["head"]["snapshot"].is_object());
    let following = first["next_offset"].as_u64().unwrap() as usize;
    let binding = first["comparison_id"].as_str().unwrap();
    let next: serde_json::Value = context
        .request(request(following, Some(binding.into())), &cancel)
        .unwrap();
    assert_eq!(next["offset"], following);
    assert_eq!(next["comparison_id"], binding);
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
