//! Public snapshot preparation and comparison across real linked worktrees.
#![cfg(unix)]
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    String::from_utf8(checked(output)).unwrap().trim().into()
}

fn checked(output: Output) -> Vec<u8> {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

struct Fixture {
    directory: tempfile::TempDir,
    root: PathBuf,
    cache: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap().join("repo");
        let cache = root.with_file_name("cache");
        fs::create_dir(&root).unwrap();
        git(&root, &["init", "-q"]);
        git(&root, &["config", "user.email", "snapshot@example.invalid"]);
        git(&root, &["config", "user.name", "Snapshot acceptance"]);
        fs::write(root.join("tsconfig.json"), r#"{"files":["main.ts"]}"#).unwrap();
        fs::write(root.join("notes\t[preview].txt"), "baseline\n").unwrap();
        fs::write(
            root.join("main.ts"),
            "export function value() { return 1; }\n",
        )
        .unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "base"]);
        Self {
            directory,
            root,
            cache,
        }
    }

    fn engine(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"));
        command.env("CODECALLNAVIGATOR_SNAPSHOT_CACHE", &self.cache);
        command
    }

    fn status(&self, project: &Path, revision: &str) -> Value {
        let mut command = self.engine();
        let output = command
            .args(["snapshot-status", "--project"])
            .arg(project)
            .args(["--revision", revision])
            .output()
            .unwrap();
        serde_json::from_slice(&checked(output)).unwrap()
    }

    fn analyze(&self, project: &Path) -> Output {
        self.engine()
            .args(["analyze", "--project"])
            .arg(project)
            .arg("tsconfig.json")
            .output()
            .unwrap()
    }

    fn prepare(&self, revision: &str, target: &str) -> Output {
        self.engine()
            .args(["prepare-snapshot", "--project"])
            .arg(&self.root)
            .args(["--revision", revision, "--target", target])
            .output()
            .unwrap()
    }

    fn compare(&self, project: &Path, base: &str, head: &str) -> Value {
        let mut command = self.engine();
        let mut child = command
            .arg("query")
            .env("PATH", "/usr/bin:/bin")
            .env(
                "CODECALLNAVIGATOR_RESOURCES",
                self.directory.path().join("unavailable-resources"),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let request = json!({"version":2,"project":project,"query":{"kind":"compare","base":base,"head":head}});
        write!(child.stdin.take().unwrap(), "{request}").unwrap();
        serde_json::from_slice(&checked(child.wait_with_output().unwrap())).unwrap()
    }
}

fn link_modules(root: &Path) {
    std::os::unix::fs::symlink(
        std::env::var_os("CODECALLNAVIGATOR_TS_NODE_MODULES")
            .expect("TypeScript fixture dependencies"),
        root.join("node_modules"),
    )
    .unwrap();
}

#[test]
fn failed_preparation_removes_only_the_checkout_it_created() {
    let fixture = Fixture::new();
    let before = git(&fixture.root, &["worktree", "list", "--porcelain"]);
    let output = fixture.prepare("HEAD", "missing/tsconfig.json");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Preparing local snapshot"));
    assert_eq!(
        git(&fixture.root, &["worktree", "list", "--porcelain"]),
        before
    );
    assert_eq!(
        fixture.status(&fixture.root, "HEAD")["snapshots"],
        json!([])
    );
    assert!(!fixture.root.join(".codecallnavigator").exists());
}

#[test]
#[ignore = "requires installed TypeScript and Node"]
fn prepared_and_retained_graphs_compare_across_worktrees() {
    let fixture = Fixture::new();
    link_modules(&fixture.root);
    let base = git(&fixture.root, &["rev-parse", "HEAD"]);
    let before = git(&fixture.root, &["worktree", "list", "--porcelain"]);
    let output = fixture.prepare(&base, "tsconfig.json");
    let prepared: Value = serde_json::from_slice(&checked(output)).unwrap();
    assert_eq!(
        before,
        git(&fixture.root, &["worktree", "list", "--porcelain"])
    );
    let initial = fixture.status(&fixture.root, &base);
    assert_eq!(initial["snapshots"][0], prepared);
    assert!(!fixture.root.join(".codecallnavigator/graph.json").exists());

    let linked = fixture
        .directory
        .path()
        .canonicalize()
        .unwrap()
        .join("linked");
    git(
        &fixture.root,
        &[
            "worktree",
            "add",
            "--detach",
            linked.to_str().unwrap(),
            &base,
        ],
    );
    link_modules(&linked);
    assert_eq!(fixture.status(&linked, &base), initial);
    fs::write(
        linked.join("main.ts"),
        "export function added() { return 2; }\n",
    )
    .unwrap();
    fs::write(linked.join("notes\t[preview].txt"), "baseline\ninserted\n").unwrap();
    git(
        &linked,
        &[
            "--literal-pathspecs",
            "add",
            "--",
            "main.ts",
            "notes\t[preview].txt",
        ],
    );
    git(&linked, &["commit", "-qm", "head"]);
    let head = git(&linked, &["rev-parse", "HEAD"]);
    assert_impact_without_head_graph(&fixture, &linked, &base, &head);
    checked(fixture.analyze(&linked));
    let retained = fixture.status(&linked, &head);
    assert_eq!(
        retained["snapshots"].as_array().unwrap().len(),
        1,
        "{retained}"
    );

    let working = fixture.compare(&linked, &base, "WORKTREE");
    assert_eq!(working["result"]["base"]["snapshot"], prepared);
    assert_eq!(working["result"]["graph_equal"], false);
    assert_dirty_result_is_worktree_only(&fixture, &linked, &head, &retained);
    assert_retargeted_dependency_is_not_a_revision_snapshot(&fixture, &linked);
    git(
        &fixture.root,
        &["worktree", "remove", "--force", linked.to_str().unwrap()],
    );
    assert_saved_comparison_is_read_only(&fixture, &base, &head, &retained);
}

fn assert_saved_comparison_is_read_only(
    fixture: &Fixture,
    base: &str,
    head: &str,
    retained: &Value,
) {
    let status = git(&fixture.root, &["status", "--porcelain"]);
    let index = fs::read(fixture.root.join(".git/index")).unwrap();
    let refs = git(&fixture.root, &["show-ref"]);
    let committed = fixture.compare(&fixture.root, base, head);
    assert_eq!(
        committed["result"]["head"]["snapshot"],
        retained["snapshots"][0]
    );
    assert_eq!(committed["result"]["graph_equal"], false);
    assert_eq!(git(&fixture.root, &["status", "--porcelain"]), status);
    assert_eq!(fs::read(fixture.root.join(".git/index")).unwrap(), index);
    assert_eq!(git(&fixture.root, &["show-ref"]), refs);
}

fn assert_impact_without_head_graph(fixture: &Fixture, linked: &Path, base: &str, head: &str) {
    assert!(
        fixture.status(linked, head)["snapshots"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let run = |base: &str, head: &str| {
        fixture
            .engine()
            .args(["impact-preview", "--project"])
            .arg(linked)
            .args(["--base", base, "--head", head])
            .env("PATH", "/usr/bin:/bin")
            .env(
                "CODECALLNAVIGATOR_RESOURCES",
                fixture.directory.path().join("unavailable-resources"),
            )
            .output()
            .unwrap()
    };
    let missing = run(head, head);
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("snapshot_missing"));
    let preview: Value = serde_json::from_slice(&checked(run(base, head))).unwrap();
    assert_eq!(
        (&preview["base"]["commit"], &preview["head"]["commit"]),
        (&json!(base), &json!(head))
    );
    assert_eq!(
        (
            &preview["records"][0]["path"],
            &preview["records"][0]["old_start"]
        ),
        (&json!("main.ts"), &json!(1))
    );
    assert!(
        preview["records"][0]["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .any(|symbol| symbol["label"] == "value")
    );
    let inserted = &preview["records"][1];
    assert_eq!(
        (
            &inserted["path"],
            &inserted["old_count"],
            &inserted["unresolved"]
        ),
        (
            &json!("notes\t[preview].txt"),
            &json!(0),
            &json!("insert_only")
        )
    );
    assert!(!linked.join(".codecallnavigator/graph.json").exists());
}

#[test]
#[ignore = "requires JDK 25 and Maven"]
fn maven_preparation_retains_a_revision_with_generated_source_roots() {
    let fixture = Fixture::new();
    let templates = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/maven");
    for directory in ["parent", "standalone"] {
        let project = fixture.root.join(directory);
        fs::create_dir(&project).unwrap();
        fs::copy(
            templates.join(directory).join("pom.xml"),
            project.join("pom.xml"),
        )
        .unwrap();
    }
    let sources = fixture.root.join("standalone/src/main/java");
    fs::create_dir_all(&sources).unwrap();
    fs::write(
        sources.join("Example.java"),
        "public class Example { public static int value() { return 1; } }\n",
    )
    .unwrap();
    git(&fixture.root, &["add", "parent", "standalone"]);
    git(&fixture.root, &["commit", "-qm", "Maven revision"]);
    let checkouts = git(&fixture.root, &["worktree", "list", "--porcelain"]);
    let snapshot: Value =
        serde_json::from_slice(&checked(fixture.prepare("HEAD", "standalone/pom.xml"))).unwrap();
    assert_eq!(
        fixture.status(&fixture.root, "HEAD")["snapshots"],
        json!([snapshot])
    );
    assert_eq!(
        fixture.compare(&fixture.root, "HEAD", "HEAD")["result"]["graph_equal"],
        true
    );
    assert_eq!(
        git(&fixture.root, &["worktree", "list", "--porcelain"]),
        checkouts
    );
}

fn assert_dirty_result_is_worktree_only(
    fixture: &Fixture,
    linked: &Path,
    head: &str,
    retained: &Value,
) {
    fs::write(
        linked.join("main.ts"),
        "export function dirty() { return 3; }\n",
    )
    .unwrap();
    let output = fixture.analyze(linked);
    let messages = String::from_utf8_lossy(&output.stderr).into_owned();
    checked(output);
    assert!(
        messages.contains("Local snapshot was not retained"),
        "{messages}"
    );
    assert_eq!(fixture.status(linked, head), *retained);
    assert_eq!(
        fixture.compare(linked, head, "WORKTREE")["result"]["graph_equal"],
        false
    );
}

fn assert_retargeted_dependency_is_not_a_revision_snapshot(fixture: &Fixture, linked: &Path) {
    git(linked, &["restore", "--", "main.ts"]);
    git(linked, &["add", "-f", "node_modules"]);
    git(linked, &["commit", "-qm", "Track original dependency link"]);
    let alternate = fixture.directory.path().join("alternate");
    fs::create_dir(&alternate).unwrap();
    link_modules(&alternate);
    fs::remove_file(linked.join("node_modules")).unwrap();
    std::os::unix::fs::symlink(alternate.join("node_modules"), linked.join("node_modules"))
        .unwrap();

    let output = fixture.analyze(linked);
    let messages = String::from_utf8_lossy(&output.stderr).into_owned();
    checked(output);
    assert!(
        messages.contains("Tracked link prevents exact input attribution"),
        "{messages}"
    );
    assert_eq!(fixture.status(linked, "HEAD")["snapshots"], json!([]));
}
