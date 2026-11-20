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
    engine_temp: Option<PathBuf>,
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
            engine_temp: None,
        }
    }

    fn isolate_engine_temp(&mut self) -> PathBuf {
        let path = self.directory.path().join("engine-temp");
        fs::create_dir(&path).unwrap();
        self.engine_temp = Some(path.clone());
        path
    }

    fn engine(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"));
        command.env("CODECALLNAVIGATOR_GRAPH_LIBRARY", &self.cache);
        if let Some(path) = &self.engine_temp {
            command.env("TMPDIR", path);
        }
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

    fn prepare_exact(&self, targets: Value) -> Output {
        let mut child = self
            .engine()
            .arg("serve-once")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let request = json!({
            "command": "prepare_snapshot",
            "version": codecallnavigator_core::protocol::VERSION,
            "project": self.root,
            "revision": "HEAD",
            "exact_targets": targets
        });
        let mut input = child.stdin.take().unwrap();
        writeln!(input, "{request}").unwrap();
        // The machine protocol interprets EOF as cancellation while preparing a snapshot.
        let output = child.wait_with_output().unwrap();
        drop(input);
        output
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
    std::os::unix::fs::symlink("tsconfig.json", fixture.root.join("tsconfig.link.json")).unwrap();
    git(&fixture.root, &["add", "tsconfig.link.json"]);
    git(&fixture.root, &["commit", "-qm", "Redirected target"]);
    let before = git(&fixture.root, &["worktree", "list", "--porcelain"]);
    let output = fixture.prepare("HEAD", "missing/tsconfig.json");
    let messages = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success() && messages.contains("Preparing local snapshot"),
        "{messages}"
    );
    for (path, cause) in [
        ("missing/tsconfig.json", "Target path is unavailable"),
        ("tsconfig.link.json", "Target path is redirected"),
    ] {
        let output = fixture.prepare_exact(json!([{
            "path": path, "kind": "typescript", "language": "typescript"
        }]));
        let message = String::from_utf8_lossy(&output.stdout);
        assert!(
            !output.status.success()
                && message.contains("Preparing local snapshot")
                && message.contains(path)
                && message.contains("HEAD")
                && message.contains(cause),
            "{message}"
        );
    }
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
#[ignore = "requires JDK 25 and Gradle"]
fn gradle_snapshot_proves_custom_class_output_and_ignores_unrelated_directory() {
    let mut fixture = Fixture::new();
    let engine_temp = fixture.isolate_engine_temp();
    let template = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/gradle");
    fs::create_dir_all(fixture.root.join("gradle/wrapper")).unwrap();
    for input in [
        "gradlew",
        "gradle/wrapper/gradle-wrapper.jar",
        "gradle/wrapper/gradle-wrapper.properties",
    ] {
        fs::copy(template.join(input), fixture.root.join(input)).unwrap();
    }

    fs::write(
        fixture.root.join("settings.gradle"),
        "rootProject.name = 'evidence'",
    )
    .unwrap();
    fs::write(
        fixture.root.join("build.gradle"),
        r#"
plugins { id 'java' }
java { toolchain { languageVersion = JavaLanguageVersion.of(25) } }
layout.buildDirectory = layout.projectDirectory.dir('compiled-products')
def renderAssets = tasks.register('renderAssets') {
    doLast {
        file('unrelated-results').mkdirs()
        file('unrelated-results/index.html').text = '<p>report</p>'
    }
}
tasks.named('compileJava') { dependsOn(renderAssets) }
"#,
    )
    .unwrap();
    fs::write(
        fixture.root.join(".gitignore"),
        "/.gradle/\n/compiled-products/\n/unrelated-results/\n",
    )
    .unwrap();
    let sources = fixture.root.join("src/main/java");
    fs::create_dir_all(&sources).unwrap();
    fs::write(
        sources.join("Example.java"),
        "public class Example { public static int value() { return 17; } }",
    )
    .unwrap();
    git(&fixture.root, &["add", "."]);
    git(&fixture.root, &["commit", "-qm", "Gradle evidence fixture"]);
    let checkouts = registered_worktrees(&fixture);
    checked(fixture.prepare_exact(json!([{
        "path": ".", "kind": "gradle", "language": "java"
    }])));
    let status = fixture.status(&fixture.root, "HEAD");
    assert_eq!(status["snapshots"].as_array().unwrap().len(), 1, "{status}");
    assert_snapshot_checkout_removed(&fixture, &engine_temp, &checkouts);
}

#[test]
#[ignore = "requires JDK 25 and Maven"]
fn maven_preparation_retains_a_revision_with_generated_source_roots() {
    let mut fixture = Fixture::new();
    let engine_temp = fixture.isolate_engine_temp();
    let templates = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    for (template, directory) in [("maven/parent", "parent"), ("maven-snapshot", "standalone")] {
        let project = fixture.root.join(directory);
        fs::create_dir(&project).unwrap();
        fs::copy(
            templates.join(template).join("pom.xml"),
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
    fs::write(fixture.root.join("standalone/build.gradle"), "").unwrap();
    fs::write(
        fixture.root.join("standalone/.gitignore"),
        "/target/\n/asyncapi.yaml\n/arbitrary-frontend-products/\n/src/main/java/IgnoredGenerated.java\n",
    )
    .unwrap();
    git(&fixture.root, &["add", "parent", "standalone"]);
    git(&fixture.root, &["commit", "-qm", "Maven revision"]);
    let stale_class = seed_preexisting_java_output(&fixture);
    let stale_bytes = fs::read(&stale_class).unwrap();
    let commit = git(&fixture.root, &["rev-parse", "HEAD"]);
    let checkouts = registered_worktrees(&fixture);
    let output = checked(fixture.prepare_exact(json!([{
        "path": "standalone", "kind": "maven", "language": "java"
    }])));
    let terminal: Value = serde_json::from_slice(
        output
            .split(|byte| *byte == b'\n')
            .rfind(|line| !line.is_empty())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(terminal["event"], "snapshot_prepared", "{terminal}");
    let snapshot = terminal["snapshot"].clone();
    assert_eq!(snapshot["identity"]["commit"], commit);
    assert_eq!(fs::read(stale_class).unwrap(), stale_bytes);
    let selection: Value =
        serde_json::from_str(snapshot["identity"]["selection"].as_str().unwrap()).unwrap();
    assert_eq!(
        selection["builds"]["targets"],
        json!([{"path": "@repo/standalone", "kind": "maven", "language": "java"}])
    );
    assert_eq!(
        fixture.status(&fixture.root, "HEAD")["snapshots"],
        json!([snapshot])
    );
    assert_eq!(
        fixture.compare(&fixture.root, "HEAD", "HEAD")["result"]["graph_equal"],
        true
    );
    assert_generated_method_is_retained(&fixture);
    assert_snapshot_checkout_removed(&fixture, &engine_temp, &checkouts);
    assert_preexisting_java_outputs_are_worktree_only(&fixture, &snapshot);
    let pom = fs::read_to_string(fixture.root.join("standalone/pom.xml")).unwrap();
    assert_build_rewriting_tracked_java_cannot_prepare_snapshot(&fixture, &pom);
    assert_snapshot_checkout_removed(&fixture, &engine_temp, &checkouts);
    assert_build_compiling_ignored_java_cannot_prepare_snapshot(&fixture, &pom);
    assert_snapshot_checkout_removed(&fixture, &engine_temp, &checkouts);
}

#[test]
#[ignore = "requires Maven and JDK 25"]
fn failed_maven_snapshot_reports_compiler_error_and_retains_logs() {
    const MISSING: &str = "missingSnapshotDiagnostic";
    let mut fixture = Fixture::new();
    let engine_temp = fixture.isolate_engine_temp();
    commit_broken_maven_project(&fixture, MISSING);
    let worktrees = registered_worktrees(&fixture);

    let terminal = fixture.prepare("HEAD", "broken");
    let stderr = String::from_utf8_lossy(&terminal.stderr);
    assert!(
        !terminal.status.success() && stderr.contains(MISSING),
        "{stderr}"
    );
    assert!(
        stderr.contains("Build logs") && stderr.contains("codecallnavigator-"),
        "{stderr}"
    );

    assert_maven_failure_event(&fixture, &engine_temp, MISSING);
    assert_snapshot_checkout_removed(&fixture, &engine_temp, &worktrees);
    assert_eq!(
        fixture.status(&fixture.root, "HEAD")["snapshots"],
        json!([])
    );
}

fn commit_broken_maven_project(fixture: &Fixture, missing: &str) {
    let project = fixture.root.join("broken");
    let sources = project.join("src/test/java");
    fs::create_dir_all(&sources).unwrap();
    let parent = fixture.root.join("parent");
    fs::create_dir(&parent).unwrap();
    fs::write(
        parent.join("pom.xml"),
        include_str!("../../../fixtures/maven/parent/pom.xml"),
    )
    .unwrap();
    fs::write(
        project.join("pom.xml"),
        r#"<project>
  <modelVersion>4.0.0</modelVersion>
  <parent><groupId>codecallnavigator.fixture</groupId><artifactId>external-parent</artifactId><version>1.0</version><relativePath>../parent/pom.xml</relativePath></parent>
  <groupId>snapshot</groupId><artifactId>broken</artifactId><version>1</version>
</project>"#,
    )
    .unwrap();
    fs::write(
        sources.join("Broken.java"),
        format!("class Broken {{ Object result = {missing}(); }}"),
    )
    .unwrap();
    git(&fixture.root, &["add", "broken", "parent"]);
    git(&fixture.root, &["commit", "-qm", "Broken Maven revision"]);
}

fn assert_maven_failure_event(fixture: &Fixture, engine_temp: &Path, missing: &str) {
    let machine = fixture.prepare_exact(json!([{
        "path": "broken", "kind": "maven", "language": "java"
    }]));
    assert!(!machine.status.success());
    let event: Value = serde_json::from_slice(
        machine
            .stdout
            .split(|byte| *byte == b'\n')
            .rfind(|line| !line.is_empty())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        (&event["event"], &event["code"], &event["target"]["phase"]),
        (&json!("failed"), &json!("failed"), &json!("compilation")),
        "{event}"
    );
    assert!(
        event["message"].as_str().unwrap().contains(missing),
        "{event}"
    );
    let logs = Path::new(event["target"]["logs"].as_str().unwrap());
    assert!(logs.starts_with(engine_temp), "{}", logs.display());
    assert!(
        fs::read_to_string(logs.join("maven-compile.stdout"))
            .unwrap()
            .contains(missing)
    );
    assert!(logs.join("maven-compile.stderr").is_file());
}

fn assert_generated_method_is_retained(fixture: &Fixture) {
    let generated = fixture.query(
        &fixture.root,
        json!({
            "kind": "flow_comparison", "base": "HEAD", "head": "HEAD",
            "selection": {"kind": "catalog", "text": "GeneratedThing"}
        }),
    );
    assert!(
        generated["result"]["records"]
            .as_array()
            .unwrap()
            .iter()
            .any(|record| {
                record["membership"] == "both"
                    && record["symbol"]
                        .as_str()
                        .unwrap()
                        .contains("GeneratedThing")
            }),
        "{generated}"
    );
}

fn registered_worktrees(fixture: &Fixture) -> Vec<String> {
    git(&fixture.root, &["worktree", "list", "--porcelain"])
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .map(str::to_owned)
        .collect()
}

fn assert_snapshot_checkout_removed(fixture: &Fixture, engine_temp: &Path, checkouts: &[String]) {
    assert_eq!(registered_worktrees(fixture), checkouts);
    let snapshot_directories: Vec<_> = fs::read_dir(engine_temp)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .filter(|name| name.to_string_lossy().starts_with("ccnavigator-snapshot-"))
        .collect();
    assert!(snapshot_directories.is_empty(), "{snapshot_directories:?}");
}

fn seed_preexisting_java_output(fixture: &Fixture) -> PathBuf {
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
    stale_classes.join("Stale.class")
}

fn assert_preexisting_java_outputs_are_worktree_only(fixture: &Fixture, snapshot: &Value) {
    let output = fixture.analyze_target(&fixture.root, "standalone/pom.xml");
    let messages = String::from_utf8_lossy(&output.stderr).to_string();
    checked(output);
    assert_eq!(
        fs::read_to_string(fixture.root.join("standalone/asyncapi.yaml")).unwrap(),
        "asyncapi: 3.0.0"
    );
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

fn assert_build_rewriting_tracked_java_cannot_prepare_snapshot(fixture: &Fixture, pom: &str) {
    add_validation_action(
        fixture,
        pom,
        r#"<echo file="${project.basedir}/src/main/java/Example.java">public class Example { public static int value() { return 42; } }</echo>"#,
    );
    git(&fixture.root, &["add", "standalone/pom.xml"]);
    git(
        &fixture.root,
        &["commit", "-qm", "Build rewrites tracked Java"],
    );
    let blob = git(
        &fixture.root,
        &["rev-parse", "HEAD:standalone/src/main/java/Example.java"],
    );
    assert_snapshot_input_rejected(
        fixture,
        &format!("An analyzed input differs from its Git blob {blob}"),
    );
    assert!(
        fs::read_to_string(fixture.root.join("standalone/src/main/java/Example.java"))
            .unwrap()
            .contains("return 1")
    );
}

fn assert_build_compiling_ignored_java_cannot_prepare_snapshot(fixture: &Fixture, pom: &str) {
    add_validation_action(
        fixture,
        pom,
        r#"<echo file="${project.basedir}/src/main/java/IgnoredGenerated.java">class IgnoredGenerated { static int value() { return 42; } }</echo>"#,
    );
    fs::write(
        fixture.root.join("standalone/src/main/java/Example.java"),
        "public class Example { public static int value() { return IgnoredGenerated.value(); } }\n",
    )
    .unwrap();
    git(
        &fixture.root,
        &[
            "add",
            "standalone/pom.xml",
            "standalone/src/main/java/Example.java",
        ],
    );
    git(
        &fixture.root,
        &["commit", "-qm", "Build consumes ignored Java"],
    );
    assert_snapshot_input_rejected(
        fixture,
        "Input differs from the Git revision: standalone/src/main/java/IgnoredGenerated.java",
    );
}

fn add_validation_action(fixture: &Fixture, pom: &str, action: &str) {
    let updated = pom.replace(
        "Validation completed</echo>",
        &format!("Validation completed</echo>\n                {action}"),
    );
    assert_ne!(pom, updated);
    fs::write(fixture.root.join("standalone/pom.xml"), updated).unwrap();
}

fn assert_snapshot_input_rejected(fixture: &Fixture, expected: &str) {
    let output = fixture.prepare("HEAD", "standalone/pom.xml");
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success() && diagnostic.contains(expected),
        "{diagnostic}"
    );
    assert_eq!(
        fixture.status(&fixture.root, "HEAD")["snapshots"],
        json!([])
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
