//! Real Git refs and library bundles share endpoint validation and generations.
use codecallnavigator_core::{
    artifact::{
        self,
        local_snapshots::{Retention, SnapshotIdentity, Store},
        revision::{self, ArtifactResult, ObservedSchema},
    },
    model::GraphData,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
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
    root: tempfile::TempDir,
    project: PathBuf,
    graph: artifact::EncodedArtifact,
}
impl Repository {
    fn new(document: &artifact::Artifact) -> Self {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("nested project");
        fs::create_dir(&project).unwrap();
        fs::write(project.join("source.txt"), "analyzed input").unwrap();
        git(root.path(), &["init", "-q"]);
        git(root.path(), &["config", "user.name", "Inspection tests"]);
        git(
            root.path(),
            &["config", "user.email", "test@example.invalid"],
        );
        git(root.path(), &["add", "."]);
        git(root.path(), &["commit", "-qm", "tracked source"]);
        let fixture = Self {
            root,
            project,
            graph: document.encode().unwrap(),
        };
        fixture.retain("HEAD");
        fixture
    }

    fn retain(&self, reference: &str) {
        let repository = revision::Repository::open(&self.project).unwrap();
        let identity = SnapshotIdentity {
            commit: repository.resolve(reference).unwrap(),
            project: repository.project,
            analyzer: "fixture".into(),
            selection: "{}".into(),
        };
        Store::system()
            .unwrap()
            .retain(
                &repository.common_dir,
                Retention {
                    identity,
                    encoded: &self.graph,
                },
                &AtomicBool::new(false),
            )
            .unwrap();
    }

    fn publish_working(&self) -> PathBuf {
        artifact::AnalysisLock::acquire(&self.project)
            .unwrap()
            .publish_encoded(&self.graph, &AtomicBool::new(false))
            .unwrap();
        artifact::working_manifest_path(&self.project).unwrap()
    }
}

fn artifact() -> artifact::Artifact {
    artifact::canonical(
        &GraphData::default(),
        &[("repo".into(), PathBuf::from("/snapshot"))],
    )
    .unwrap()
}

#[test]
fn valid_inspection_and_legacy_endpoint_have_identical_serialization() {
    let repo = Repository::new(&artifact());
    let old = revision::load(&repo.project, "HEAD").unwrap();
    let inspected = revision::inspect(&repo.project, "HEAD").unwrap();
    assert_eq!(
        serde_json::to_vec(&old.endpoint).unwrap(),
        serde_json::to_vec(&inspected.endpoint).unwrap()
    );
    let endpoint = inspected.endpoint;
    assert_eq!(endpoint.requested_ref, "HEAD");
    assert!(Path::new(&endpoint.path).is_absolute());
    assert!(Path::new(&endpoint.path).ends_with("graph.json"));
    assert_eq!(fs::read(&endpoint.path).unwrap(), repo.graph.as_ref());
    assert_eq!(
        endpoint.digest,
        blake3::hash(repo.graph.as_ref()).to_hex().to_string()
    );
    assert_eq!(
        endpoint.schema,
        ObservedSchema::Known(artifact::SCHEMA.into())
    );
    assert!(endpoint.snapshot.is_some());
    assert!(matches!(inspected.artifact, ArtifactResult::Available(_)));
}

#[test]
fn tampered_commit_manifest_fails_before_returning_provenance() {
    let repo = Repository::new(&artifact());
    let endpoint = revision::load(&repo.project, "HEAD").unwrap().endpoint;
    fs::write(endpoint.path, b"not the published manifest").unwrap();
    let error = revision::inspect(&repo.project, "HEAD").err().unwrap();
    assert_eq!(revision::error_code(&error), "snapshot_unavailable");
    assert!(error.to_string().contains("manifest differs"));
}

#[test]
fn valid_generation_without_flows_is_available_without_fabricated_inventory() {
    let mut document = artifact();
    document
        .graph
        .as_object_mut()
        .unwrap()
        .remove("flow_inventory");
    let repo = Repository::new(&document);
    let inspected = revision::inspect(&repo.project, "HEAD").unwrap();
    let ArtifactResult::Available(artifact) = inspected.artifact else {
        panic!("Compatible verified artifact rejected");
    };
    assert!(artifact.graph["flow_inventory"].is_null());
    artifact.validate().unwrap();
}

#[test]
fn read_failures_keep_legacy_codes_and_messages() {
    let repo = Repository::new(&artifact());
    for reference in ["WORKTREE", "missing-ref", "--help", "HEAD:file"] {
        let old = revision::load(&repo.project, reference).err().unwrap();
        let new = revision::inspect(&repo.project, reference).err().unwrap();
        assert_eq!(revision::error_code(&new), revision::error_code(&old));
        assert_eq!(format!("{new:#}"), format!("{old:#}"));
    }
}

#[test]
fn moved_ref_with_identical_bundle_changes_binding_without_checkout_or_writes() {
    let repo = Repository::new(&artifact());
    let root = repo.root.path();
    git(root, &["branch", "moving"]);
    let first = revision::inspect(&repo.project, "moving").unwrap();
    git(
        root,
        &["commit", "--allow-empty", "-qm", "same graph new commit"],
    );
    repo.retain("HEAD");
    git(root, &["update-ref", "refs/heads/moving", "HEAD"]);
    let before_head = git(root, &["rev-parse", "HEAD"]);
    let before_status = git(root, &["status", "--porcelain"]);
    let before_index = fs::read(root.join(".git/index")).unwrap();
    let second = revision::inspect(&repo.project, "moving").unwrap();
    assert_eq!(first.endpoint.digest, second.endpoint.digest);
    assert_ne!(first.endpoint.commit, second.endpoint.commit);
    assert_ne!(first.endpoint.path, second.endpoint.path);
    let identity = |endpoint| {
        blake3::hash(
            &serde_json::to_vec(&(artifact::difference::VERSION, &first.endpoint, endpoint))
                .unwrap(),
        )
        .to_hex()
        .to_string()
    };
    assert_ne!(identity(&first.endpoint), identity(&second.endpoint));
    assert_eq!(git(root, &["rev-parse", "HEAD"]), before_head);
    assert_eq!(git(root, &["status", "--porcelain"]), before_status);
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), before_index);
    assert!(!root.join(".git/FETCH_HEAD").exists());
    assert!(!repo.project.join(".codecallnavigator").exists());
}

#[test]
fn corrupt_working_manifest_keeps_catalog_binding_and_verified_snapshot() {
    let repo = Repository::new(&artifact());
    let root = repo.root.path();
    let working_path = repo.publish_working();
    let valid = revision::inspect(&repo.project, "WORKTREE").unwrap();
    let status = git(root, &["status", "--porcelain"]);
    let index = fs::read(root.join(".git/index")).unwrap();
    let bytes = b"{\"schema\":\"future.graph.v99\",\"graph\":{}}";
    fs::write(&working_path, bytes).unwrap();
    let observed = revision::inspect(&repo.project, "WORKTREE").unwrap();
    assert_eq!(observed.endpoint.commit, None);
    assert_eq!(observed.endpoint.path, working_path.display().to_string());
    assert_eq!(observed.endpoint.digest, valid.endpoint.digest);
    let ArtifactResult::Unavailable(failure) = observed.artifact else {
        panic!("Unverified manifest bytes became available");
    };
    assert_eq!(failure.code, "invalid_graph");
    assert!(failure.message.contains("manifest differs"));
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
    assert_eq!(git(root, &["status", "--porcelain"]), status);
    assert_eq!(fs::read(&working_path).unwrap(), bytes);
    assert!(matches!(
        revision::inspect(&repo.project, "HEAD").unwrap().artifact,
        ArtifactResult::Available(_)
    ));
}
