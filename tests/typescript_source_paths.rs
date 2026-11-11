//! Real compiler SourceFile aliases share portable paths without changing native bindings.
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::Command,
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
        let result = Command::new(engine)
            .args(["analyze", "--project"])
            .arg(&self.project)
            .arg("--root")
            .arg(format!("ref={}", self.external.display()))
            .arg("tsconfig.json")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        Snapshot {
            bytes: fs::read(self.project.join(".codecallnavigator/graph.json")).unwrap(),
            config_hash,
        }
    }
}

struct Snapshot {
    bytes: Vec<u8>,
    config_hash: String,
}

impl Snapshot {
    fn graph_and_checked_provenance(&self) -> (Value, Value) {
        let mut artifact: Value = serde_json::from_slice(&self.bytes).unwrap();
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
#[ignore = "requires prebuilt standalone engine and installed TypeScript6/React/Next dependencies"]
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
