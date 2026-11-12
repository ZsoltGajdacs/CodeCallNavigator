use super::*;
use std::{fs, sync::atomic::AtomicBool};

fn workspace(path: &Path) -> Workspace {
    let mut workspace = Workspace {
        root: crate::source_path::identity(&path.to_string_lossy()),
        ..Default::default()
    };
    workspace.refresh();
    workspace
}
fn load_next(workspace: &mut Workspace) {
    let path = workspace.queue.pop_front().expect("directory request");
    let result = loader::read_fixture(&path, &workspace.cancelled);
    workspace.accept(Output {
        generation: workspace.generation,
        canonical_root: (path == workspace.root)
            .then(|| fs::canonicalize(&path).ok())
            .flatten()
            .map(|root| crate::source_path::identity(&root.to_string_lossy())),
        path,
        result,
    });
}

#[test]
fn lists_every_entry_folders_first_without_following_directory_links() {
    let temporary = tempfile::tempdir().unwrap();
    fs::create_dir(temporary.path().join("z-folder")).unwrap();
    for name in [".hidden", ".gitignore", "ignored.bin", "a.txt"] {
        fs::write(temporary.path().join(name), "ignored.bin").unwrap();
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(temporary.path(), temporary.path().join("link")).unwrap();
    let entries =
        loader::read_fixture(&temporary.path().to_string_lossy(), &AtomicBool::new(false)).unwrap();
    assert_eq!(entries[0].label, "z-folder");
    assert_eq!(entries[0].kind, Kind::Directory);
    assert!(entries.iter().any(|entry| entry.label == "ignored.bin"));
    assert!(entries.iter().any(|entry| entry.label == ".hidden"));
    #[cfg(unix)]
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.label == "link")
            .unwrap()
            .kind,
        Kind::Symlink
    );
}

#[test]
fn nested_reveal_loads_only_expanded_ancestors_and_keeps_duplicate_names_distinct() {
    let temporary = tempfile::tempdir().unwrap();
    fs::create_dir_all(temporary.path().join("a/nested")).unwrap();
    fs::create_dir(temporary.path().join("b")).unwrap();
    fs::write(temporary.path().join("a/nested/same.txt"), "a").unwrap();
    fs::write(temporary.path().join("b/same.txt"), "b").unwrap();
    let mut tree = workspace(temporary.path());
    load_next(&mut tree);
    assert_eq!(tree.rows.len(), 2);
    let selected =
        crate::source_path::identity(&temporary.path().join("a/nested/same.txt").to_string_lossy());
    tree.highlight(BTreeSet::from([
        selected.clone(),
        "/elsewhere/outside.txt".into(),
    ]));
    load_next(&mut tree);
    load_next(&mut tree);
    assert_eq!(tree.selected, BTreeSet::from([selected.clone()]));
    assert!(tree.rows.iter().any(|row| row.path == selected));
    assert!(
        !tree
            .directories
            .contains_key(&temporary.path().join("b").to_string_lossy().into_owned())
    );
}

#[test]
fn refresh_retains_existing_selection_and_expansion_but_prunes_removed_entries() {
    let temporary = tempfile::tempdir().unwrap();
    fs::create_dir(temporary.path().join("folder")).unwrap();
    let file = temporary.path().join("folder/file.txt");
    fs::write(&file, "content").unwrap();
    let mut tree = workspace(temporary.path());
    load_next(&mut tree);
    let selected = crate::source_path::identity(&file.to_string_lossy());
    tree.highlight(BTreeSet::from([selected.clone()]));
    load_next(&mut tree);
    tree.refresh();
    load_next(&mut tree);
    load_next(&mut tree);
    assert!(tree.selected.contains(&selected));
    assert!(
        tree.expanded
            .contains(file.parent().unwrap().to_str().unwrap())
    );
    fs::remove_dir_all(file.parent().unwrap()).unwrap();
    tree.refresh();
    load_next(&mut tree);
    assert!(tree.selected.is_empty());
    assert!(tree.expanded.is_empty());
}

#[test]
fn empty_error_retry_cancel_and_stale_completion_are_observable() {
    let temporary = tempfile::tempdir().unwrap();
    let mut tree = workspace(temporary.path());
    assert!(matches!(tree.rows[0].kind, RowKind::Loading));
    load_next(&mut tree);
    assert!(matches!(tree.rows[0].kind, RowKind::Empty));
    let generation = tree.generation;
    tree.refresh();
    tree.accept(Output {
        generation,
        canonical_root: None,
        path: tree.root.clone(),
        result: Err("stale".into()),
    });
    assert!(matches!(tree.rows[0].kind, RowKind::Loading));
    let missing = temporary.path().join("missing");
    let mut failed = workspace(&missing);
    load_next(&mut failed);
    assert!(matches!(failed.rows[0].kind, RowKind::Error));
    fs::create_dir(&missing).unwrap();
    failed.retry(failed.root.clone());
    load_next(&mut failed);
    assert!(matches!(failed.rows[0].kind, RowKind::Empty));
    fs::write(temporary.path().join("file"), "x").unwrap();
    assert!(
        loader::read_fixture(temporary.path().to_str().unwrap(), &AtomicBool::new(true)).is_err()
    );
}

#[test]
fn method_and_group_files_resolve_without_navigation_mutation() {
    use crate::projection::{SourcePath, ViewNode};
    use codecallnavigator::{
        graph::GraphIndex,
        model::{GraphData, Node},
    };
    let mut state = State::fixture();
    state.graph = Some(Arc::new(
        GraphIndex::new(Arc::new(GraphData {
            nodes: vec![Node {
                id: "method".into(),
                file: "/repo/one.cs".into(),
                ..Default::default()
            }],
            ..Default::default()
        }))
        .unwrap(),
    ));
    state.graph_view.nodes.push(ViewNode {
        key: "method".into(),
        member: Some(0),
        ..Default::default()
    });
    state.visual_selection = Some("method".into());
    assert_eq!(
        selected_files(&state),
        BTreeSet::from(["/repo/one.cs".into()])
    );
    state.graph_view.nodes.push(ViewNode {
        key: "group".into(),
        files: vec![
            SourcePath {
                label: "one.cs".into(),
                path: "/repo/one.cs".into(),
                language: "C#".into(),
            },
            SourcePath {
                label: "two.cs".into(),
                path: "/repo/two.cs".into(),
                language: "C#".into(),
            },
        ],
        ..Default::default()
    });
    state.visual_selection = Some("group".into());
    assert_eq!(
        selected_files(&state),
        BTreeSet::from(["/repo/one.cs".into(), "/repo/two.cs".into()])
    );
    assert!(state.nav.selected.is_none());
    state.visual_selection = None;
    state.nav.focus_target = Some(crate::projection::FocusTarget::FileClasses(
        "/repo/two.cs".into(),
    ));
    assert_eq!(
        selected_files(&state),
        BTreeSet::from(["/repo/two.cs".into()])
    );
}

#[test]
fn layout_reconciliation_does_not_overwrite_explorer_only_selection() {
    use crate::projection::{FocusTarget, SourcePath, ViewNode};
    let mut state = State::fixture();
    state.explorer.root = "/repo".into();
    state.graph_view.nodes.push(ViewNode {
        key: "group".into(),
        files: vec![SourcePath {
            label: "a.cs".into(),
            path: "/repo/a.cs".into(),
            language: "C#".into(),
        }],
        ..Default::default()
    });
    state.visual_selection = Some("group".into());
    assert!(selection::sync(&mut state));
    state.explorer.select("/repo/manual.txt".into());
    state.visible_revision = Some((state.epoch, state.revision));
    assert!(!selection::sync(&mut state));
    assert!(state.explorer.selected.contains("/repo/manual.txt"));
    state.visual_selection = None;
    state.nav.focus_target = Some(FocusTarget::Group("group".into()));
    state.graph_view.focus_keys.push("group".into());
    assert!(selection::sync(&mut state));
    assert_eq!(
        state.explorer.selected,
        BTreeSet::from(["/repo/a.cs".into()])
    );
}

#[test]
fn reveal_waits_until_selected_ancestor_chains_are_loaded() {
    let temporary = tempfile::tempdir().unwrap();
    fs::create_dir_all(temporary.path().join("a/deep")).unwrap();
    for file in ["b.cs", "a/deep/a.cs"] {
        fs::write(temporary.path().join(file), "class").unwrap();
    }
    let mut tree = workspace(temporary.path());
    load_next(&mut tree);
    let first =
        crate::source_path::identity(&temporary.path().join("a/deep/a.cs").to_string_lossy());
    let second = crate::source_path::identity(&temporary.path().join("b.cs").to_string_lossy());
    tree.highlight(BTreeSet::from([first.clone(), second]));
    assert!(tree.path_pending(&first));
    load_next(&mut tree);
    assert!(tree.path_pending(&first));
    load_next(&mut tree);
    assert!(!tree.path_pending(&first));
    let first_visible = tree
        .rows
        .iter()
        .find(|row| row.is_file() && tree.selected.contains(&row.path))
        .unwrap();
    assert_eq!(first_visible.path, first);
}

#[cfg(unix)]
#[test]
fn selected_root_alias_maps_canonical_graph_files_without_following_interior_links() {
    let temporary = tempfile::tempdir().unwrap();
    let real = temporary.path().join("actual");
    let alias = temporary.path().join("chosen-root");
    fs::create_dir(&real).unwrap();
    fs::write(real.join("file.cs"), "class Example {}").unwrap();
    std::os::unix::fs::symlink(&real, &alias).unwrap();
    std::os::unix::fs::symlink(&real, real.join("interior-link")).unwrap();
    let tree = workspace(&alias);
    let mut state = State::fixture();
    state.explorer = tree;
    selection::sync(&mut state);
    let mut tree = std::mem::take(&mut state.explorer);
    load_next(&mut tree);
    assert!(
        tree.selection_token.is_none(),
        "new root mapping must resynchronize graph selection"
    );
    let graph_file = fs::canonicalize(real.join("file.cs"))
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let logical_file = crate::source_path::identity(&alias.join("file.cs").to_string_lossy());
    tree.highlight(BTreeSet::from([graph_file.clone()]));
    assert_eq!(tree.selected, BTreeSet::from([logical_file.clone()]));
    assert_eq!(tree.graph_path(&logical_file), graph_file);
    tree.highlight(BTreeSet::from([logical_file.clone()]));
    assert!(
        tree.selected.contains(&logical_file),
        "logical graph paths remain supported"
    );
    let link = tree
        .rows
        .iter()
        .find(|row| row.label == "interior-link")
        .unwrap();
    assert!(matches!(link.kind, RowKind::Symlink));
    assert!(tree.queue.is_empty());
    tree.observe_root("/another-root", 1.0);
    assert!(tree.canonical_root.is_none());
}
