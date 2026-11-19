//! Real compiler SourceFile aliases share portable paths without changing native bindings.
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

struct Fixture {
    root: tempfile::TempDir,
    project: PathBuf,
    external: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let base = root.path().canonicalize().unwrap();
        let project = base.join("project");
        let external = base.join("external");
        fs::create_dir_all(project.join("src/app/api")).unwrap();
        fs::create_dir_all(project.join("src/test-support")).unwrap();
        fs::create_dir_all(&external).unwrap();
        fs::write(project.join("src/app/page.tsx"), "import {redirect} from 'next/navigation'; export default function Page(){return null} export function go(){redirect('/')} export function request(){return fetch('/api')}").unwrap();
        fs::write(
            project.join("src/app/api/route.ts"),
            "export function GET(){return new Response('ok')}",
        )
        .unwrap();
        fs::write(
            project.join("src/test-support/helper.ts"),
            "export function support(){return 1}",
        )
        .unwrap();
        fs::write(external.join("shared.ts"), "type Listener=()=>number;const callbacks=new Set<Listener>();export function subscribe(cb:Listener){callbacks.add(cb)}export function notify(){for(const cb of callbacks)cb()}export function redraw(){return 1}subscribe(redraw);").unwrap();
        let modules = std::env::var_os("CODECALLNAVIGATOR_TS_NODE_MODULES")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../PacificFateMapEditor/node_modules")
            });
        symlink(
            modules.canonicalize().unwrap(),
            project.join("node_modules"),
        )
        .unwrap();
        symlink(&project, base.join("project-alias")).unwrap();
        symlink(&external, base.join("external-alias")).unwrap();
        symlink(
            project.join("src/test-support"),
            base.join("ordinary-alias"),
        )
        .unwrap();
        Self {
            root,
            project,
            external,
        }
    }
    fn configuration(&self, alias: bool) -> Value {
        let base = self.root.path().canonicalize().unwrap();
        let (project, external, support) = if alias {
            (
                base.join("project-alias"),
                base.join("external-alias"),
                base.join("ordinary-alias/helper.ts"),
            )
        } else {
            (
                self.project.clone(),
                self.external.clone(),
                self.project.join("src/test-support/helper.ts"),
            )
        };
        json!({"compilerOptions":{"strict":true,"target":"ES2023","module":"esnext","moduleResolution":"bundler","jsx":"react-jsx","types":[],"skipLibCheck":true},"files":[project.join("src/app/page.tsx"),project.join("src/app/api/route.ts"),external.join("shared.ts"),support]})
    }

    fn analyze(&self, alias: bool) -> Snapshot {
        let config = serde_json::to_vec(&self.configuration(alias)).unwrap();
        let config_hash = blake3::hash(&config).to_hex().to_string();
        fs::write(self.project.join("tsconfig.json"), config).unwrap();
        let engine = std::env::var_os("CODECALLNAVIGATOR_TEST_ENGINE")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/codecallnavigator-engine")
            });
        let mut child = Command::new(engine)
            .arg("serve-once")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        // EOF cancels analysis: retain this handle through wait_with_output.
        let mut input = child.stdin.take().unwrap();
        writeln!(
            input,
            "{}",
            json!({
                "command":"analyze", "version":codecallnavigator_core::protocol::VERSION,
                "project":self.project, "roots":[["ref",self.external]],
                "infer_linked_roots":false,
                "targets":[{"path":self.project.join("tsconfig.json"),
                    "language":"typescript", "kind":"typescript", "label":"tsconfig.json"}]
            })
        )
        .unwrap();
        input.flush().unwrap();
        let result = child.wait_with_output().unwrap();
        drop(input);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let events = String::from_utf8(result.stdout).unwrap();
        let complete = events
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .find(|event| event["event"] == "complete")
            .expect("analysis must complete");
        Snapshot {
            bytes: fs::read(
                codecallnavigator::artifact::working_manifest_path(&self.project).unwrap(),
            )
            .unwrap(),
            artifact: serde_json::to_value(
                codecallnavigator::artifact::load(&self.project).unwrap(),
            )
            .unwrap(),
            config_hash,
            roots: serde_json::from_value(complete["roots"].clone()).unwrap(),
            fingerprints: serde_json::from_value(complete["fingerprints"].clone()).unwrap(),
            digest: complete["digest"].as_str().unwrap().to_owned(),
        }
        .check_runtime_coverage(self, alias)
    }
}

struct Snapshot {
    bytes: Vec<u8>,
    artifact: Value,
    config_hash: String,
    roots: Vec<(String, PathBuf)>,
    fingerprints: Vec<codecallnavigator_core::model::FileFingerprint>,
    digest: String,
}

impl Snapshot {
    fn check_runtime_coverage(self, fixture: &Fixture, alias: bool) -> Self {
        assert_eq!(self.digest, blake3::hash(&self.bytes).to_hex().to_string());
        let config = fixture.project.join("tsconfig.json");
        assert!(
            self.roots
                .iter()
                .any(|(name, root)| name == "repo" && config.strip_prefix(root).is_ok())
        );
        let analysis = &self.artifact["graph"]["analysis"];
        assert_eq!(analysis["targets"], json!(["@repo/tsconfig.json"]));
        let unmapped = self
            .fingerprints
            .iter()
            .filter(|input| {
                !self
                    .roots
                    .iter()
                    .any(|(_, root)| Path::new(&input.path).strip_prefix(root).is_ok())
            })
            .count();
        assert!(
            unmapped > 0,
            "fixture must exercise unmapped input reporting"
        );
        assert_eq!(
            analysis["limitations"],
            json!([{"code":"unmapped_input","count":unmapped}])
        );
        for source in fixture.configuration(alias)["files"].as_array().unwrap() {
            let lexical = Path::new(source.as_str().unwrap());
            let physical = lexical.canonicalize().unwrap();
            for expected in [lexical, physical.as_path()] {
                assert!(
                    self.fingerprints
                        .iter()
                        .any(|input| Path::new(&input.path) == expected),
                    "Required source input missing: {}",
                    expected.display()
                );
            }
        }
        self
    }

    fn graph_and_checked_provenance(&self) -> (Value, Value) {
        let mut artifact = self.artifact.clone();
        let mut analysis = artifact["graph"]
            .as_object_mut()
            .unwrap()
            .remove("analysis")
            .unwrap();
        let configurations: Vec<_> = analysis["inputs"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .filter(|input| input["path"] == "@repo/tsconfig.json")
            .collect();
        assert_eq!(
            configurations.len(),
            1,
            "The actual configuration must be fingerprinted once"
        );
        for input in configurations {
            assert_eq!(input["kind"], "file");
            assert_eq!(
                input["hash"], self.config_hash,
                "Keep the hash of the actual configuration bytes"
            );
            // Only the deliberately changed config hash is excluded from comparison.
            input.as_object_mut().unwrap().remove("hash");
        }
        for path in ["@repo/src/app/package.json", "@ref/package.json"] {
            assert!(
                analysis["inputs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|input| input["path"] == path && input["kind"] == "missing"),
                "Missing input coverage: {path}"
            );
        }
        // Each request's complete runtime inventory independently proves its exact
        // limitation count above. Distinct lexical probes need not have equal counts.
        analysis.as_object_mut().unwrap().remove("limitations");
        // Artifact input order includes content hashes; the deliberately changed
        // config hash can move its row even when input coverage is identical.
        analysis["inputs"]
            .as_array_mut()
            .unwrap()
            .sort_by(|a, b| a["path"].as_str().unwrap().cmp(b["path"].as_str().unwrap()));
        (artifact, analysis)
    }
}

#[test]
#[ignore = "requires prebuilt standalone engine and installed TypeScript5.9-or-6.0/React/Next dependencies"]
fn compiler_source_aliases_match_canonical_spelling_with_named_roots() {
    let fixture = Fixture::new();
    let aliased = fixture.analyze(true);
    assert_eq!(
        aliased.bytes,
        fixture.analyze(true).bytes,
        "Alias inputs must repeat byte-for-byte"
    );
    let canonical = fixture.analyze(false);
    assert_eq!(
        canonical.bytes,
        fixture.analyze(false).bytes,
        "Canonical inputs must repeat byte-for-byte"
    );
    assert_ne!(aliased.config_hash, canonical.config_hash);
    assert_ne!(
        aliased.bytes, canonical.bytes,
        "Provenance must retain changed configuration bytes"
    );
    let (alias_graph, alias_inputs) = aliased.graph_and_checked_provenance();
    let (canonical_graph, canonical_inputs) = canonical.graph_and_checked_provenance();
    assert_eq!(
        alias_graph, canonical_graph,
        "Source spelling aliases must not alter graph semantics"
    );
    assert_eq!(
        alias_inputs, canonical_inputs,
        "Normalized input coverage must match except the independently checked configuration hash"
    );
    assert_semantic_facts(&alias_graph);
}

fn assert_semantic_facts(artifact: &Value) {
    let graph = &artifact["graph"];
    let nodes = graph["nodes"].as_array().unwrap();
    assert!(
        nodes
            .iter()
            .any(|node| node["label"] == "support" && node["is_test"] == true)
    );
    assert!(
        graph["architecture"]["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file["file"] == "@repo/src/test-support/helper.ts"
                && file["is_test"] == true)
    );
    assert_eq!(
        graph["boundaries"]["routes"].as_array().unwrap().len(),
        2,
        "Next convention checks must recognize canonical source locations"
    );
    assert_eq!(
        graph["edges"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|edge| edge["provenance"] == "codecallnavigator.boundary.v1")
            .count(),
        2
    );
    assert!(
        graph["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["provenance"] == "codecallnavigator.observer.v1")
    );
}
