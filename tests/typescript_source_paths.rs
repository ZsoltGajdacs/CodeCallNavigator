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
        let modules = std::env::var_os("CODEGRAPH_TS_NODE_MODULES")
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
    fn analyze(&self, alias: bool) -> Vec<u8> {
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
        let config = json!({"compilerOptions":{"strict":true,"target":"ES2023","module":"esnext","moduleResolution":"bundler","jsx":"react-jsx","types":[],"skipLibCheck":true},"files":[project.join("src/app/page.tsx"),project.join("src/app/api/route.ts"),external.join("shared.ts"),support]});
        fs::write(
            self.project.join("tsconfig.json"),
            serde_json::to_vec(&config).unwrap(),
        )
        .unwrap();
        let engine = std::env::var_os("CODEGRAPH_TEST_ENGINE")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/codegraph-engine")
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
        fs::read(self.project.join(".codegraph/graph.json")).unwrap()
    }
}

#[test]
#[ignore = "requires prebuilt standalone engine and installed TypeScript6/React/Next dependencies"]
fn compiler_source_aliases_match_canonical_spelling_with_named_roots() {
    let fixture = Fixture::new();
    let aliased = fixture.analyze(true);
    let canonical = fixture.analyze(false);
    assert_eq!(
        aliased, canonical,
        "Source spelling aliases must not alter portable graph semantics"
    );
    let artifact: Value = serde_json::from_slice(&aliased).unwrap();
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
            .filter(|edge| edge["provenance"] == "codegraph.boundary.v1")
            .count(),
        2
    );
    assert!(
        graph["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["provenance"] == "codegraph.observer.v1")
    );
}
