//! Real filesystem boundaries cannot be proved by pure path classification tests.
use super::*;

#[test]
fn hidden_directories_are_incidental_but_required_roots_and_hidden_files_are_observed() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join(".claude/worktrees/other")).unwrap();
    fs::create_dir_all(root.path().join(".generated")).unwrap();
    let generated = root.path().join(".generated/Generated.java");
    fs::write(&generated, "class Generated {}").unwrap();
    let hidden_file = root.path().join(".env");
    fs::write(&hidden_file, "value=one").unwrap();
    let incidental = input_paths_excluding(&[root.path().into()], &[]).unwrap();
    assert!(!incidental.contains(&generated));
    assert!(incidental.contains(&hidden_file));
    let before = capture_excluding(incidental, &[]).unwrap();
    fs::write(
        root.path().join(".claude/worktrees/other/New.java"),
        "ignored",
    )
    .unwrap();
    fs::create_dir(root.path().join(".new-hidden")).unwrap();
    assert!(!is_stale(&before).unwrap());
    let required = collect_inputs(
        &[
            ScanRoot::Broad(root.path().into()),
            ScanRoot::Required(root.path().join(".generated")),
        ],
        &[],
        false,
    )
    .unwrap();
    assert!(required.contains(&generated));
    assert!(
        collect_inputs(
            &[ScanRoot::Broad(root.path().join(".generated"))],
            &[],
            false
        )
        .unwrap()
        .is_empty()
    );
    let before = capture_excluding(required, &[]).unwrap();
    fs::write(generated, "class Changed {}").unwrap();
    assert!(is_stale(&before).unwrap());
}

#[cfg(unix)]
#[test]
fn incidental_links_are_skipped_but_required_link_retargeting_and_cycles_are_detected() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::create_dir(outside.path().join("one")).unwrap();
    fs::create_dir(outside.path().join("two")).unwrap();
    for name in ["one", "two"] {
        fs::write(outside.path().join(name).join("App.java"), "identical").unwrap();
    }
    let alias = root.path().join("linked");
    symlink(outside.path().join("one"), &alias).unwrap();
    symlink(root.path().join("missing"), root.path().join("broken")).unwrap();
    symlink(root.path().join("cycle"), root.path().join("cycle")).unwrap();
    let paths = input_paths_excluding(&[root.path().into()], &[]).unwrap();
    assert_eq!(paths, vec![root.path().to_path_buf()]);
    assert!(
        collect_inputs(
            &[ScanRoot::Required(root.path().join("broken"))],
            &[],
            false
        )
        .and_then(|paths| capture_excluding(paths, &[]))
        .is_err()
    );
    let before = capture_excluding(paths, &[]).unwrap();
    let required = collect_inputs(&[ScanRoot::Required(alias.clone())], &[], false).unwrap();
    let required_before = capture_excluding(required, &[]).unwrap();
    fs::remove_file(&alias).unwrap();
    symlink(outside.path().join("two"), &alias).unwrap();
    assert!(!is_stale(&before).unwrap());
    assert!(is_stale(&required_before).unwrap());
    // The first link spelling can stay unchanged while an intermediate link moves.
    let chain = root.path().join("chain");
    symlink(&alias, &chain).unwrap();
    let chain_before = capture_excluding(
        collect_inputs(&[ScanRoot::Required(chain)], &[], false).unwrap(),
        &[],
    )
    .unwrap();
    fs::remove_file(&alias).unwrap();
    symlink(outside.path().join("one"), &alias).unwrap();
    assert!(is_stale(&chain_before).unwrap());

    assert!(
        collect_inputs(&[ScanRoot::Required(root.path().join("cycle"))], &[], false)
            .and_then(|paths| capture_excluding(paths, &[]))
            .is_err()
    );
}

#[test]
fn legacy_directory_hashes_keep_hidden_membership_semantics() {
    let root = tempfile::tempdir().unwrap();
    let before = observe_content_with_policy(root.path(), &[], InputPolicy::Legacy).unwrap();
    let current = observe_content_with_policy(root.path(), &[], InputPolicy::ScopedV1).unwrap();
    fs::create_dir(root.path().join(".authored")).unwrap();
    assert_ne!(
        before,
        observe_content_with_policy(root.path(), &[], InputPolicy::Legacy).unwrap()
    );
    assert_eq!(
        current,
        observe_content_with_policy(root.path(), &[], InputPolicy::ScopedV1).unwrap()
    );
}
