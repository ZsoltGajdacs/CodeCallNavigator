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
        fs::write(root.join("tsconfig.alt.json"), r#"{"files":["main.ts"]}"#).unwrap();
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
        command.env("CODECALLNAVIGATOR_GRAPH_LIBRARY", &self.cache);
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
        self.analyze_target(project, "tsconfig.json")
    }

    fn analyze_target(&self, project: &Path, target: &str) -> Output {
        self.engine()
            .args(["analyze", "--project"])
            .arg(project)
            .arg(target)
            .output()
            .unwrap()
    }

    fn graph_path(&self, project: &Path) -> Result<PathBuf, Output> {
        let output = self
            .engine()
            .args(["graph-path", "--project"])
            .arg(project)
            .output()
            .unwrap();
        if output.status.success() {
            Ok(PathBuf::from(
                String::from_utf8(output.stdout).unwrap().trim(),
            ))
        } else {
            Err(output)
        }
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
        self.query(project, json!({"kind":"compare","base":base,"head":head}))
    }

    fn query(&self, project: &Path, query: Value) -> Value {
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
        let request = json!({"version":2,"project":project,"query":query});
        write!(child.stdin.take().unwrap(), "{request}").unwrap();
        let output = child.wait_with_output().unwrap();
        let response: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "Invalid query response: {error}; {}",
                String::from_utf8_lossy(&output.stderr)
            );
        });
        assert_eq!(
            output.status.success(),
            response["error"].is_null(),
            "{response}"
        );
        response
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
    let messages = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success() && messages.contains("Preparing local snapshot"),
        "{messages}"
    );
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
    assert!(fixture.graph_path(&fixture.root).is_err());

    let (linked, head) = linked_head(&fixture, &base, &initial);
    assert_impact_without_head_graph(&fixture, &linked, &base, &head);
    checked(fixture.analyze(&linked));
    let retained = fixture.status(&linked, &head);
    let working = fixture.compare(&linked, &base, "WORKTREE");
    assert_eq!(working["result"]["base"]["snapshot"], prepared);
    assert_eq!(working["result"]["graph_equal"], false);
    assert_verified_publication(&fixture, &linked, &head, &retained);
    assert_dirty_result_is_worktree_only(&fixture, &linked, &head, &retained);
    assert_retargeted_dependency_is_not_a_revision_snapshot(&fixture, &linked);
    git(
        &fixture.root,
        &["worktree", "remove", "--force", linked.to_str().unwrap()],
    );
    assert_saved_comparison_is_read_only(&fixture, &base, &head, &retained);
    assert_distinct_variants(&fixture, &base);
    assert_head_switch_is_worktree_only(&fixture, &base);
}

fn linked_head(fixture: &Fixture, base: &str, initial: &Value) -> (PathBuf, String) {
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
            base,
        ],
    );
    link_modules(&linked);
    assert_eq!(&fixture.status(&linked, base), initial);
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
    (linked, head)
}

fn assert_verified_publication(fixture: &Fixture, linked: &Path, head: &str, retained: &Value) {
    assert_eq!(
        retained["snapshots"].as_array().unwrap().len(),
        1,
        "{retained}"
    );
    let manifest = fixture.graph_path(linked).unwrap();
    assert!(manifest.starts_with(&fixture.cache));
    assert!(manifest.components().any(|part| part.as_os_str() == head));
    assert!(!linked.join(".codecallnavigator").exists());
    let selection = typescript_selection(&retained["snapshots"][0]);
    let status = fixture.query(linked, json!({"kind":"status"}));
    assert_eq!(status["result"]["effective_analysis"], selection);
    let exact = fixture.compare(linked, head, "WORKTREE");
    let (base, current) = (&exact["result"]["base"], &exact["result"]["head"]);
    let manifest = manifest.to_str().unwrap();
    assert_eq!(
        (&base["path"], &current["path"], &current["digest"]),
        (&json!(manifest), &json!(manifest), &base["digest"])
    );
    assert_eq!(exact["result"]["graph_equal"], true);
}

fn typescript_selection(record: &Value) -> Value {
    let selected: Value =
        serde_json::from_str(record["identity"]["selection"].as_str().unwrap()).unwrap();
    let analyzer = &selected["analyzers"][0];
    assert_eq!(
        (
            &selected["builds"]["targets"][0]["kind"],
            &analyzer["path"],
            &analyzer["backend"]
        ),
        (
            &json!("typescript"),
            &json!("@repo/tsconfig.json"),
            &json!("TypeScript")
        )
    );
    assert!(analyzer["version"].is_string() && selected["dependency_identity"].is_string());
    selected
}

fn assert_distinct_variants(fixture: &Fixture, commit: &str) {
    checked(fixture.analyze_target(&fixture.root, "tsconfig.alt.json"));
    let status = fixture.status(&fixture.root, commit);
    let records = status["snapshots"].as_array().unwrap();
    assert_eq!(records.len(), 2);
    assert_ne!(records[0]["id"], records[1]["id"]);
    let mut paths = records
        .iter()
        .map(|record| {
            let selected: Value =
                serde_json::from_str(record["identity"]["selection"].as_str().unwrap()).unwrap();
            selected["analyzers"][0]["path"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect::<Vec<_>>();
    paths.sort();
    assert_eq!(paths, ["@repo/tsconfig.alt.json", "@repo/tsconfig.json"]);
    assert_eq!(
        fixture.compare(&fixture.root, commit, "WORKTREE")["error"]["code"],
        "snapshot_ambiguous"
    );
}

fn assert_head_switch_is_worktree_only(fixture: &Fixture, original: &str) {
    use std::os::unix::fs::PermissionsExt;
    git(
        &fixture.root,
        &["commit", "--allow-empty", "-qm", "Different commit"],
    );
    let replacement = git(&fixture.root, &["rev-parse", "HEAD"]);
    git(&fixture.root, &["reset", "--soft", original]);
    let driver = fixture.directory.path().join("git-driver");
    fs::create_dir(&driver).unwrap();
    let wrapper = driver.join("git");
    // Delegate every successful read to Git. Move HEAD after the engine captures
    // it at analysis start, before the next real HEAD verification returns.
    fs::write(
        &wrapper,
        r#"#!/bin/sh
case "$*" in
  *'rev-parse --verify --end-of-options HEAD^{commit}'*)
    if [ -f "$TEST_HEAD_OBSERVED" ]; then
      /usr/bin/git -C "$TEST_GIT_PROJECT" update-ref HEAD "$TEST_GIT_REPLACEMENT" || exit 1
    else
      touch "$TEST_HEAD_OBSERVED" || exit 1
    fi
    ;;
esac
exec /usr/bin/git "$@"
"#,
    )
    .unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).unwrap();
    let mut paths = vec![driver.clone()];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let before = fixture.status(&fixture.root, original);
    let output = fixture
        .engine()
        .args(["analyze", "--project"])
        .arg(&fixture.root)
        .arg("tsconfig.json")
        .env("PATH", std::env::join_paths(paths).unwrap())
        .env("TEST_HEAD_OBSERVED", driver.join("observed"))
        .env("TEST_GIT_PROJECT", &fixture.root)
        .env("TEST_GIT_REPLACEMENT", &replacement)
        .output()
        .unwrap();
    let messages = String::from_utf8_lossy(&output.stderr).to_string();
    checked(output);
    assert!(
        messages.contains("HEAD changed during analysis"),
        "{messages}"
    );
    assert_eq!(fixture.status(&fixture.root, original), before);
    assert_eq!(
        fixture.status(&fixture.root, &replacement)["snapshots"],
        json!([])
    );
    assert!(
        fixture
            .graph_path(&fixture.root)
            .unwrap()
            .starts_with(fixture.cache.join("worktrees"))
    );
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
    assert_preexisting_java_outputs_are_worktree_only(&fixture, &snapshot);
}

fn assert_preexisting_java_outputs_are_worktree_only(fixture: &Fixture, snapshot: &Value) {
    let stale_source = fixture.directory.path().join("Stale.java");
    fs::write(
        &stale_source,
        "public class Stale { public static int old() { return 1; } }\n",
    )
    .unwrap();
    let stale_classes = fixture.root.join("standalone/target/classes");
    fs::create_dir_all(&stale_classes).unwrap();
    let jdk = codecallnavigator_engine::analyzers::AnalyzerRuntime::discover()
        .unwrap()
        .java_home
        .expect("JDK 25 for Java acceptance");
    checked(
        Command::new(jdk.join("bin/javac"))
            .arg("-d")
            .arg(&stale_classes)
            .arg(&stale_source)
            .output()
            .unwrap(),
    );
    let output = fixture.analyze_target(&fixture.root, "standalone/pom.xml");
    let messages = String::from_utf8_lossy(&output.stderr).to_string();
    checked(output);
    assert!(
        messages.contains("Java outputs predate the observed compiler attempt"),
        "{messages}"
    );
    assert_eq!(
        fixture.status(&fixture.root, "HEAD")["snapshots"],
        json!([snapshot])
    );
    assert!(
        fixture
            .graph_path(&fixture.root)
            .unwrap()
            .starts_with(fixture.cache.join("worktrees"))
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
    assert!(messages.contains("Saved working graph only:"), "{messages}");
    let manifest = fixture.graph_path(linked).unwrap();
    assert!(
        manifest
            .components()
            .any(|part| part.as_os_str() == "worktrees")
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
