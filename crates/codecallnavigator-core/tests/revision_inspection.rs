//! Real Git inspection retains endpoint evidence without weakening artifact validation.
use codecallnavigator_core::{
    artifact::{
        self,
        revision::{self, ArtifactResult, ObservedSchema},
    },
    model::GraphData,
};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn git(root: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}
struct Repository {
    directory: tempfile::TempDir,
    project: PathBuf,
    bytes: Vec<u8>,
}
impl Repository {
    fn new(mut bytes: Vec<u8>) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let project = directory.path().join("nested project");
        fs::create_dir_all(project.join(".codecallnavigator")).unwrap();
        if let Ok(document) = artifact::Artifact::from_bytes(&bytes) {
            artifact::publish(
                &project,
                &document,
                &std::sync::atomic::AtomicBool::new(false),
            )
            .unwrap();
            bytes = fs::read(project.join(artifact::GRAPH_PATH)).unwrap();
        } else {
            fs::write(project.join(artifact::GRAPH_PATH), &bytes).unwrap();
        }
        git(directory.path(), &["init", "-q"]);
        git(
            directory.path(),
            &["config", "user.name", "Inspection tests"],
        );
        git(
            directory.path(),
            &["config", "user.email", "test@example.invalid"],
        );
        git(directory.path(), &["add", "."]);
        git(directory.path(), &["commit", "-qm", "saved graph"]);
        Self {
            directory,
            project,
            bytes,
        }
    }
}
fn artifact() -> artifact::Artifact {
    artifact::canonical(
        &GraphData::default(),
        &[("repo".into(), PathBuf::from("/snapshot"))],
    )
    .unwrap()
}
fn valid_bytes() -> Vec<u8> {
    serde_json::to_vec_pretty(&artifact()).unwrap()
}

#[test]
fn valid_inspection_and_legacy_endpoint_have_identical_serialization() {
    let repo = Repository::new(valid_bytes());
    let old = revision::load(&repo.project, "HEAD").unwrap();
    let inspected = revision::inspect(&repo.project, "HEAD").unwrap();
    assert_eq!(
        serde_json::to_vec(&old.endpoint).unwrap(),
        serde_json::to_vec(&inspected.endpoint).unwrap()
    );
    assert_eq!(inspected.endpoint.requested_ref, "HEAD");
    assert_eq!(
        inspected.endpoint.path,
        "nested project/.codecallnavigator/graph.json"
    );
    assert_eq!(
        inspected.endpoint.digest,
        blake3::hash(&repo.bytes).to_hex().to_string()
    );
    assert_eq!(
        inspected.endpoint.schema,
        ObservedSchema::Known(artifact::SCHEMA.into())
    );
    assert!(matches!(inspected.artifact, ArtifactResult::Available(_)));
}

#[test]
fn unsupported_monolithic_schema_or_inventory_retains_failure_and_bound_metadata() {
    for incompatible_schema in [true, false] {
        let mut document = artifact();
        if incompatible_schema {
            document.schema = "future.graph.v99".into();
        } else {
            document.graph["flow_inventory"] = json!({"version":99,"functions":[]});
        }
        let repo = Repository::new(serde_json::to_vec(&document).unwrap());
        let old = revision::load(&repo.project, "HEAD").err().unwrap();
        let expected = format!(
            "{:#}",
            artifact::storage::Manifest::from_bytes(&repo.bytes).unwrap_err()
        );
        assert_eq!(format!("{old:#}"), expected);
        let inspected = revision::inspect(&repo.project, "HEAD").unwrap();
        assert_eq!(
            inspected.endpoint.schema,
            ObservedSchema::Known(document.schema)
        );
        assert!(inspected.endpoint.commit.is_some());
        assert_eq!(
            inspected.endpoint.digest,
            blake3::hash(&repo.bytes).to_hex().to_string()
        );
        let ArtifactResult::Unavailable(failure) = inspected.artifact else {
            panic!("Strict validation was bypassed")
        };
        assert_eq!(failure.code, revision::error_code(&old));
        assert_eq!(failure.message, format!("{old:#}"));
    }
}

#[test]
fn malformed_json_has_explicit_unknown_schema_and_original_error() {
    let repo = Repository::new(b"not artifact JSON".to_vec());
    let old = revision::load(&repo.project, "HEAD").err().unwrap();
    assert_eq!(
        format!("{old:#}"),
        format!(
            "{:#}",
            artifact::storage::Manifest::from_bytes(&repo.bytes).unwrap_err()
        )
    );
    let inspected = revision::inspect(&repo.project, "HEAD").unwrap();
    assert_eq!(inspected.endpoint.schema, ObservedSchema::Unknown);
    assert!(serde_json::to_value(&inspected.endpoint).unwrap()["schema"].is_null());
    let ArtifactResult::Unavailable(failure) = inspected.artifact else {
        panic!("Malformed JSON became available")
    };
    assert_eq!(failure.code, "invalid_graph");
    assert_eq!(failure.message, format!("{old:#}"));
}

#[test]
fn valid_generation_without_flows_is_available_without_fabricated_inventory() {
    let mut document = artifact();
    document
        .graph
        .as_object_mut()
        .unwrap()
        .remove("flow_inventory");
    let repo = Repository::new(serde_json::to_vec(&document).unwrap());
    let inspected = revision::inspect(&repo.project, "HEAD").unwrap();
    let ArtifactResult::Available(artifact) = inspected.artifact else {
        panic!("Compatible historical artifact rejected")
    };
    assert!(artifact.graph["flow_inventory"].is_null());
    artifact.validate().unwrap();
}

#[test]
fn read_failures_keep_legacy_codes_and_messages() {
    let repo = Repository::new(valid_bytes());
    fs::remove_file(repo.project.join(artifact::GRAPH_PATH)).unwrap();
    for reference in ["WORKTREE", "missing-ref", "--help", "HEAD:file"] {
        let old = revision::load(&repo.project, reference).err().unwrap();
        let new = revision::inspect(&repo.project, reference).err().unwrap();
        assert_eq!(revision::error_code(&new), revision::error_code(&old));
        assert_eq!(format!("{new:#}"), format!("{old:#}"));
    }
}

#[test]
fn moved_ref_with_identical_blob_changes_binding_without_checkout_or_writes() {
    let repo = Repository::new(valid_bytes());
    let root = repo.directory.path();
    git(root, &["branch", "moving"]);
    let first = revision::inspect(&repo.project, "moving").unwrap();
    git(
        root,
        &["commit", "--allow-empty", "-qm", "same bytes new commit"],
    );
    git(root, &["update-ref", "refs/heads/moving", "HEAD"]);
    let before_head = git(root, &["rev-parse", "HEAD"]);
    let before_status = git(root, &["status", "--porcelain"]);
    let before_index = fs::read(root.join(".git/index")).unwrap();
    let second = revision::inspect(&repo.project, "moving").unwrap();
    assert_eq!(first.endpoint.digest, second.endpoint.digest);
    assert_ne!(first.endpoint.commit, second.endpoint.commit);
    let identity = |endpoint| {
        blake3::hash(
            &serde_json::to_vec(&(artifact::difference::VERSION, &first.endpoint, endpoint))
                .unwrap(),
        )
        .to_hex()
        .to_string()
    };
    assert_ne!(identity(&first.endpoint), identity(&second.endpoint));
    assert_eq!(
        fs::read(repo.project.join(artifact::GRAPH_PATH)).unwrap(),
        repo.bytes
    );
    assert_eq!(git(root, &["rev-parse", "HEAD"]), before_head);
    assert_eq!(git(root, &["status", "--porcelain"]), before_status);
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), before_index);
    assert!(!root.join(".git/FETCH_HEAD").exists());
    assert!(
        !repo
            .project
            .join(".codecallnavigator/analysis.lock")
            .exists()
    );
}

#[test]
fn worktree_observation_binds_undecodable_bytes_without_poisoning_committed_snapshot() {
    let repo = Repository::new(valid_bytes());
    let root = repo.directory.path();
    let head = git(root, &["rev-parse", "HEAD"]);
    for (bytes, schema) in [
        (b"{\"schema\":42}".to_vec(), ObservedSchema::Unknown),
        (b"{\"graph\":{}}".to_vec(), ObservedSchema::Unknown),
        (b"\xffnot UTF-8 JSON".to_vec(), ObservedSchema::Unknown),
        (
            b"{\"schema\":\"future.graph.v99\",\"graph\":{}}".to_vec(),
            ObservedSchema::Known("future.graph.v99".into()),
        ),
    ] {
        fs::write(repo.project.join(artifact::GRAPH_PATH), &bytes).unwrap();
        let status = git(root, &["status", "--porcelain"]);
        let index = fs::read(root.join(".git/index")).unwrap();
        let expected_failure = format!(
            "{:#}",
            artifact::storage::Manifest::from_bytes(&bytes).unwrap_err()
        );
        let observed = revision::inspect(&repo.project, "WORKTREE").unwrap();
        assert_eq!(
            serde_json::to_value(&observed.endpoint).unwrap(),
            json!({
                "kind":"worktree", "requested_ref":"WORKTREE", "commit":null,
                "path":"nested project/.codecallnavigator/graph.json", "schema":schema,
                "digest":blake3::hash(&bytes).to_hex().to_string(),
            })
        );
        let ArtifactResult::Unavailable(failure) = observed.artifact else {
            panic!("Undecodable worktree artifact became available")
        };
        assert_eq!(
            (failure.code, failure.message),
            ("invalid_graph", expected_failure)
        );
        assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
        assert_eq!(git(root, &["status", "--porcelain"]), status);
        assert_eq!(
            fs::read(repo.project.join(artifact::GRAPH_PATH)).unwrap(),
            bytes
        );
    }
    assert_eq!(git(root, &["rev-parse", "HEAD"]), head);
    assert!(matches!(
        revision::inspect(&repo.project, "HEAD").unwrap().artifact,
        ArtifactResult::Available(_)
    ));
}
