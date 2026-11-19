use super::*;
use crate::{
    artifact::local_snapshots::{Retention, SnapshotIdentity, Store},
    model::GraphData,
};

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn init(repo: &Path) {
    git(repo, &["init", "-q"]);
    git(repo, &["config", "user.name", "Tests"]);
    git(repo, &["config", "user.email", "test@example.invalid"]);
}

#[test]
fn verified_nested_snapshots_bind_exact_bytes_without_a_working_graph() {
    let repo = tempfile::tempdir().unwrap();
    let library = tempfile::tempdir().unwrap();
    let store = Store::at(library.path().into());
    let project = repo.path().join("nested project");
    fs::create_dir(&project).unwrap();
    fs::write(project.join("build.gradle"), "").unwrap();
    init(repo.path());
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "baseline"]);
    git(repo.path(), &["tag", "baseline"]);
    let repository = Repository::open(&project).unwrap();
    let commit = repository.resolve("baseline").unwrap();
    let artifact =
        super::super::canonical(&GraphData::default(), &[("repo".into(), project.clone())])
            .unwrap();
    let encoded = artifact.encode().unwrap();
    let record = store
        .retain(
            &repository.common_dir,
            Retention {
                identity: SnapshotIdentity {
                    commit: commit.clone(),
                    project: repository.project.clone(),
                    analyzer: "fixture".into(),
                    selection: "{}".into(),
                },
                encoded: &encoded,
            },
            &AtomicBool::new(false),
        )
        .unwrap();
    let read = inspection::read_cached(&store, &repository, commit.clone(), None).unwrap();
    assert_eq!(read.kind, EndpointKind::Commit);
    assert_eq!(read.bytes, encoded.as_ref());
    assert_eq!(read.snapshot.unwrap(), record);
    assert_eq!(read.commit.as_deref(), Some(commit.as_str()));
    assert!(Path::new(&read.path).starts_with(library.path()));
    assert!(
        Path::new(&read.path)
            .components()
            .any(|part| part.as_os_str() == commit.as_str())
    );
    assert!(read.decoded.unwrap().validate().is_ok());

    git(
        repo.path(),
        &["commit", "--allow-empty", "-qm", "another commit"],
    );
    let other = repository.resolve("HEAD").unwrap();
    let error = inspection::read_cached(&store, &repository, other, Some(&record.id))
        .err()
        .unwrap();
    assert_eq!(error_code(&error), "snapshot_missing");
    assert!(!project.join(".codecallnavigator").exists());
    assert!(!repo.path().join(".git/FETCH_HEAD").exists());
}

#[test]
fn malformed_revisions_and_legacy_working_paths_fail_explicitly() {
    let directory = tempfile::tempdir().unwrap();
    for revision in ["", "--help", "HEAD:file", "HEAD\nHEAD"] {
        assert_eq!(
            error_code(&load(directory.path(), revision).err().unwrap()),
            "invalid_request"
        );
    }
    fs::create_dir(directory.path().join(".codecallnavigator")).unwrap();
    fs::write(
        directory.path().join(".codecallnavigator/graph.json"),
        "legacy",
    )
    .unwrap();
    assert_eq!(
        error_code(&load(directory.path(), "WORKTREE").err().unwrap()),
        "graph_missing"
    );
}

#[test]
fn legacy_committed_graphs_are_not_loaded() {
    let repo = tempfile::tempdir().unwrap();
    init(repo.path());
    fs::create_dir(repo.path().join(".codecallnavigator")).unwrap();
    fs::write(repo.path().join(".codecallnavigator/graph.json"), "legacy").unwrap();
    git(repo.path(), &["add", "-f", ".codecallnavigator/graph.json"]);
    git(repo.path(), &["commit", "-qm", "legacy graph"]);
    assert_eq!(
        error_code(&load(repo.path(), "HEAD").err().unwrap()),
        "graph_missing"
    );
}

#[test]
fn revision_proof_reads_the_original_tree_when_git_replace_is_configured() {
    let repo = tempfile::tempdir().unwrap();
    init(repo.path());
    let file = repo.path().join("source");
    fs::write(&file, "original").unwrap();
    git(repo.path(), &["add", "source"]);
    git(repo.path(), &["commit", "-qm", "original"]);
    let repository = Repository::open(repo.path()).unwrap();
    let original = repository.resolve("HEAD").unwrap();
    let cancel = AtomicBool::new(false);
    let original_tree = repository.tree(&original, &cancel).unwrap();
    fs::write(&file, "replacement").unwrap();
    git(repo.path(), &["commit", "-qam", "replacement"]);
    git(repo.path(), &["replace", &original, "HEAD"]);

    let proven_tree = repository.tree(&original, &cancel).unwrap();
    assert_eq!(
        proven_tree["source"].regular_blob(),
        original_tree["source"].regular_blob()
    );
}

#[test]
fn inherited_git_environment_cannot_redirect_an_explicit_project() {
    const PROJECT: &str = "CODECALLNAVIGATOR_GIT_ENV_TEST_PROJECT";
    const COMMIT: &str = "CODECALLNAVIGATOR_GIT_ENV_TEST_COMMIT";
    if let Some(project) = std::env::var_os(PROJECT) {
        let project = std::path::PathBuf::from(project).canonicalize().unwrap();
        let repository = Repository::detect(&project).unwrap().unwrap();
        assert_eq!(repository.root, project.parent().unwrap());
        assert_eq!(repository.common_dir, repository.root.join(".git"));
        assert_eq!(repository.project, "nested");
        assert_eq!(
            repository.resolve("HEAD").unwrap(),
            std::env::var(COMMIT).unwrap()
        );
        return;
    }

    let selected = tempfile::tempdir().unwrap();
    let project = selected.path().join("nested");
    fs::create_dir(&project).unwrap();
    fs::write(project.join("source"), "selected source").unwrap();
    init(selected.path());
    git(selected.path(), &["add", "nested"]);
    git(selected.path(), &["commit", "-qm", "selected"]);
    let expected = resolve_commit(selected.path(), "HEAD").unwrap();

    let foreign = tempfile::tempdir().unwrap();
    init(foreign.path());
    git(
        foreign.path(),
        &["commit", "--allow-empty", "-qm", "foreign"],
    );
    let foreign_git = foreign.path().join(".git");
    let result = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "artifact::revision::tests::inherited_git_environment_cannot_redirect_an_explicit_project",
        ])
        .env(PROJECT, project)
        .env(COMMIT, expected)
        .env("GIT_DIR", &foreign_git)
        .env("GIT_COMMON_DIR", &foreign_git)
        .env("GIT_WORK_TREE", foreign.path())
        .env("GIT_INDEX_FILE", foreign_git.join("index"))
        .env("GIT_OBJECT_DIRECTORY", foreign_git.join("objects"))
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "core.worktree")
        .env("GIT_CONFIG_VALUE_0", foreign.path())
        .output()
        .unwrap();
    assert!(
        result.status.success() && String::from_utf8_lossy(&result.stdout).contains("1 passed"),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr),
    );
}

#[test]
fn missing_promisor_blob_never_starts_lazy_fetch() {
    let repo = tempfile::tempdir().unwrap();
    init(repo.path());
    let source = b"tracked source";
    fs::write(repo.path().join("source"), source).unwrap();
    git(repo.path(), &["add", "source"]);
    git(repo.path(), &["commit", "-qm", "baseline"]);

    let repository = Repository::open(repo.path()).unwrap();
    let commit = repository.resolve("HEAD").unwrap();
    let cancel = AtomicBool::new(false);
    let tree = repository.tree(&commit, &cancel).unwrap();
    let blob = tree["source"].regular_blob().unwrap();
    fs::remove_file(
        repo.path()
            .join(".git/objects")
            .join(&blob[..2])
            .join(&blob[2..]),
    )
    .unwrap();
    let marker = repo.path().join("fetch-attempted");
    git(
        repo.path(),
        &["config", "extensions.partialClone", "origin"],
    );
    git(repo.path(), &["config", "remote.origin.promisor", "true"]);
    git(repo.path(), &["config", "protocol.ext.allow", "always"]);
    git(
        repo.path(),
        &[
            "config",
            "remote.origin.url",
            &format!("ext::/usr/bin/touch {}", marker.display()),
        ],
    );
    let hashes = [(blob.into(), blake3::hash(source).to_hex().to_string())].into();
    assert!(repository.verify_blobs(&hashes, &cancel).is_err());
    assert!(
        !marker.exists(),
        "Reading a missing blob started a fetch helper"
    );
    assert!(!repo.path().join(".git/FETCH_HEAD").exists());
}
