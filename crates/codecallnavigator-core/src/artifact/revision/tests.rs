use super::*;
use crate::model::GraphData;

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

#[test]
fn committed_nested_snapshots_bind_exact_bytes_without_a_working_graph() {
    let repo = tempfile::tempdir().unwrap();
    let project = repo.path().join("nested project");
    fs::create_dir_all(project.join(".codecallnavigator")).unwrap();
    let artifact =
        super::super::canonical(&GraphData::default(), &[("repo".into(), project.clone())])
            .unwrap();
    let bytes = serde_json::to_vec_pretty(&artifact).unwrap();
    fs::write(project.join(GRAPH_PATH), &bytes).unwrap();
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.name", "Tests"]);
    git(
        repo.path(),
        &["config", "user.email", "test@example.invalid"],
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "baseline"]);
    git(repo.path(), &["tag", "baseline"]);
    let worktree = load(&project, "WORKTREE").unwrap();
    fs::remove_file(project.join(GRAPH_PATH)).unwrap();
    let snapshot = load(&project, "baseline").unwrap();
    assert_eq!(snapshot.endpoint.kind, EndpointKind::Commit);
    assert_eq!(
        snapshot.endpoint.digest,
        blake3::hash(&bytes).to_hex().to_string()
    );
    assert_eq!(snapshot.endpoint.digest, worktree.endpoint.digest);
    assert_eq!(
        snapshot.endpoint.path,
        "nested project/.codecallnavigator/graph.json"
    );
    assert_eq!(snapshot.endpoint.path, worktree.endpoint.path);
    assert_eq!(snapshot.endpoint.commit.as_ref().unwrap().len(), 40);
    assert!(snapshot.artifact.validate().is_ok());
    assert_eq!(
        error_code(&load(&project, "WORKTREE").err().unwrap()),
        "graph_missing"
    );
    assert_eq!(
        error_code(&load(&project, "missing").err().unwrap()),
        "revision_unavailable"
    );
    assert_eq!(
        fs::read_dir(project.join(".codecallnavigator"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn malformed_inputs_and_revision_syntax_fail_explicitly() {
    let directory = tempfile::tempdir().unwrap();
    for revision in ["", "--help", "HEAD:file", "HEAD\nHEAD"] {
        assert_eq!(
            error_code(&load(directory.path(), revision).err().unwrap()),
            "invalid_request"
        );
    }
    fs::create_dir(directory.path().join(".codecallnavigator")).unwrap();
    fs::write(directory.path().join(GRAPH_PATH), "not json").unwrap();
    assert_eq!(
        error_code(&load(directory.path(), "WORKTREE").err().unwrap()),
        "invalid_graph"
    );
}

#[test]
fn working_paths_only_fall_back_for_nonrepositories() {
    let timeout = working_path(Err(failure(
        "git_timeout",
        "Git snapshot read exceeded 30 seconds",
    )))
    .unwrap_err();
    assert_eq!(error_code(&timeout), "git_timeout");
    let unavailable =
        working_path(Err(failure("revision_unavailable", "permission denied"))).unwrap_err();
    assert_eq!(error_code(&unavailable), "revision_unavailable");
    assert_eq!(
        working_path(Err(failure(
            "revision_unavailable",
            "fatal: not a git repository"
        )))
        .unwrap(),
        GRAPH_PATH
    );
}

#[test]
fn missing_promisor_blob_never_starts_lazy_fetch() {
    let repo = tempfile::tempdir().unwrap();
    fs::create_dir(repo.path().join(".codecallnavigator")).unwrap();
    let artifact = super::super::canonical(
        &GraphData::default(),
        &[("repo".into(), repo.path().into())],
    )
    .unwrap();
    fs::write(repo.path().join(GRAPH_PATH), artifact.bytes().unwrap()).unwrap();
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.name", "Tests"]);
    git(
        repo.path(),
        &["config", "user.email", "test@example.invalid"],
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "baseline"]);
    let output = Command::new("git")
        .arg("-C")
        .arg(repo.path())
        .args(["rev-parse", &format!("HEAD:{GRAPH_PATH}")])
        .output()
        .unwrap();
    assert!(output.status.success());
    let blob = String::from_utf8(output.stdout).unwrap();
    let blob = blob.trim();
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
    let error = load(repo.path(), "HEAD").err().unwrap();
    assert_eq!(error_code(&error), "graph_missing");
    assert!(
        !marker.exists(),
        "Reading a missing blob started a fetch helper"
    );
    assert!(!repo.path().join(".git/FETCH_HEAD").exists());
}
