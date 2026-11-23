use super::{Repository, SourceChanges, SourceFile, SourceSide, git_for_project};
use std::{fs, path::Path, sync::atomic::AtomicBool};

fn git(root: &Path, args: &[&str]) {
    let output = git_for_project(root).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn init(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.name", "Tests"]);
    git(root, &["config", "user.email", "test@example.invalid"]);
}

#[test]
fn source_capture_uses_net_worktree_paths_and_exact_commit_blobs() {
    let repo = tempfile::tempdir().unwrap();
    let root = repo.path();
    init(root);
    git(root, &["config", "core.autocrlf", "false"]);
    let project = root.join("selected project");
    fs::create_dir(&project).unwrap();
    let write = |path: &str, bytes: &[u8]| fs::write(project.join(path), bytes).unwrap();
    for path in ["staged", "reverted", "recreated", "deleted", "unchanged"] {
        write(path, b"base\n");
    }
    write(".gitignore", b"ignored\n");
    git(&project, &["add", "."]);
    git(&project, &["commit", "-qm", "base"]);
    let repository = Repository::open(&project).unwrap();
    let base = repository.resolve("HEAD").unwrap();
    let cancel = AtomicBool::new(false);

    write("staged", b"staged\n");
    write("reverted", b"index only\n");
    git(&project, &["add", "staged", "reverted"]);
    write("reverted", b"base\n");
    git(&project, &["rm", "-q", "recreated"]);
    write("recreated", b"new checkout content\n");
    fs::remove_file(project.join("deleted")).unwrap();
    write("new file\nname", b"text\n");
    write("binary", b"some\0bytes");
    write("ignored", b"ignored\n");
    let embedded = project.join("embedded");
    fs::create_dir(&embedded).unwrap();
    init(&embedded);
    git(&embedded, &["commit", "--allow-empty", "-qm", "embedded"]);

    let captured = repository.source_changes(&base, None, &cancel).unwrap();
    assert_effective_checkout(&captured);
    write("staged", b"edited after reading\n");
    assert_ne!(
        captured.digest,
        repository
            .source_changes(&base, None, &cancel)
            .unwrap()
            .digest
    );
    assert!(
        repository
            .source_changes(&base, Some(&base), &cancel)
            .unwrap()
            .files
            .is_empty()
    );
    git(&project, &["commit", "-qm", "second commit"]);
    let head = repository.resolve("HEAD").unwrap();
    let commits = repository
        .source_changes(&base, Some(&head), &cancel)
        .unwrap();
    let staged = commits
        .files
        .iter()
        .find(|file| file.path == "staged")
        .unwrap();
    assert!(matches!(&staged.after, SourceSide::Text(text) if text.as_ref() == "staged\n"));
}

#[cfg(unix)]
#[test]
fn source_capture_keeps_link_and_mode_changes_as_metadata() {
    use std::os::unix::fs::{PermissionsExt, symlink};

    let checkout = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let root = checkout.path();
    init(root);
    git(root, &["config", "core.filemode", "true"]);
    fs::create_dir(root.join("parent")).unwrap();
    fs::write(root.join("parent/text"), "tracked\n").unwrap();
    fs::write(external.path().join("text"), "outside checkout\n").unwrap();
    fs::write(root.join("mode"), "same\n").unwrap();
    symlink("mode", root.join("link")).unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "base"]);
    let repository = Repository::open(root).unwrap();
    let base = repository.resolve("HEAD").unwrap();
    let cancel = AtomicBool::new(false);

    fs::remove_file(root.join("parent/text")).unwrap();
    fs::remove_dir(root.join("parent")).unwrap();
    symlink(external.path(), root.join("parent")).unwrap();
    fs::set_permissions(root.join("mode"), fs::Permissions::from_mode(0o755)).unwrap();
    git(root, &["rm", "-q", "link"]);
    symlink("mode", root.join("link")).unwrap();
    #[cfg(not(target_os = "macos"))]
    {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
        fs::write(root.join(OsStr::from_bytes(b"raw\xff")), "untracked\n").unwrap();
    }

    let capture = repository.source_changes(&base, None, &cancel).unwrap();
    let paths = capture
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();
    #[cfg(target_os = "macos")]
    assert_eq!(paths, ["mode", "parent", "parent/text"]);
    #[cfg(not(target_os = "macos"))]
    assert_eq!(paths, ["mode", "parent", "parent/text", "raw\\xff"]);
    assert_eq!(capture.files[0].reason, Some("mode_only_change"));
    assert!(matches!(
        capture.files[1].after,
        SourceSide::Unavailable("symlink")
    ));
    assert!(matches!(capture.files[2].after, SourceSide::Absent));
    #[cfg(not(target_os = "macos"))]
    assert_eq!(capture.files[3].reason, Some("non_utf8_path"));
}

fn assert_effective_checkout(captured: &SourceChanges) {
    let paths: Vec<_> = captured
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    assert_eq!(
        paths,
        [
            "binary",
            "deleted",
            "embedded",
            "new file\nname",
            "recreated",
            "staged"
        ]
    );
    assert_recreated_file(captured);
    assert!(matches!(
        by_path(captured, "deleted").after,
        SourceSide::Absent
    ));
    assert!(matches!(
        by_path(captured, "embedded").after,
        SourceSide::Unavailable("embedded_repository")
    ));
    assert!(matches!(
        by_path(captured, "binary").after,
        SourceSide::Unavailable("binary_content")
    ));
}

fn by_path<'a>(capture: &'a SourceChanges, path: &str) -> &'a SourceFile {
    capture.files.iter().find(|file| file.path == path).unwrap()
}

fn assert_recreated_file(captured: &SourceChanges) {
    let recreated = by_path(captured, "recreated");
    assert!(matches!(&recreated.before, SourceSide::Text(text) if text.as_ref() == "base\n"));
    assert!(
        matches!(&recreated.after, SourceSide::Text(text) if text.as_ref() == "new checkout content\n")
    );
}
