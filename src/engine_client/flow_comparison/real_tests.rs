//! Actual installed CLI wire decode over saved library artifacts; no compiler or UI consumer.
use super::*;
use crate::{
    engine_client::Descriptor,
    flow_comparison::tests::{fixture_runtime, git, retain_fixture},
};
use codecallnavigator::artifact::{self, Artifact};
use std::{path::Path, process::Command};

fn commit(root: &Path, saved: &Artifact, name: &str) {
    git(root, &["commit", "--allow-empty", "-qm", name]);
    git(root, &["tag", name]);
    retain_fixture(root, saved, "HEAD");
}

fn saved_pair(root: &Path) -> (Artifact, Artifact) {
    git(root, &["init", "-q"]);
    git(
        root,
        &["config", "user.name", "Typed comparison integration"],
    );
    git(root, &["config", "user.email", "test@example.invalid"]);
    let mut graph = crate::flow_scenes::data();
    let roots = [("repo".into(), "/snapshot".into())];
    let before = artifact::canonical(&graph, &roots).unwrap();
    commit(root, &before, "base");
    graph.architecture.as_mut().unwrap().files[0]
        .content
        .push_str("// head snapshot café 🧭\r\n");
    let after = artifact::canonical(&graph, &roots).unwrap();
    commit(root, &after, "head");
    git(
        root,
        &[
            "commit",
            "--allow-empty",
            "-qm",
            "unrelated subsequent commit",
        ],
    );

    (before, after)
}

fn load_symbol(page: &Catalog) -> String {
    page.records
        .iter()
        .find(|record| {
            matches!(&record.before, CatalogSide::Present(present)
            if present.node.as_ref().is_some_and(|node| node.label.starts_with("load(")))
        })
        .unwrap()
        .symbol
        .clone()
}

struct ExpectedSource<'a> {
    symbol: &'a str,
    content: &'a str,
}

fn cached_side(side: &DetailSide, expected: ExpectedSource<'_>) {
    let DetailSide::Ready(ready) = side else {
        panic!("Saved flow did not decode as ready: {side:?}")
    };
    assert_eq!(ready.source.content, expected.content);
    assert_eq!(
        (ready.source.symbol.as_str(), ready.function.symbol.as_str()),
        (expected.symbol, expected.symbol)
    );
    assert!(ready.targets.iter().any(|target| target.label == "save()"));
}

fn assert_pinned_revision(root: &Path, bound: &BoundComparison) {
    let tip = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "refs/tags/head"])
        .output()
        .unwrap();
    assert_eq!(
        bound.header().head.commit.as_deref(),
        Some(String::from_utf8(tip.stdout).unwrap().trim())
    );
    let current = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    assert_ne!(
        bound.header().head.commit.as_deref(),
        Some(String::from_utf8(current.stdout).unwrap().trim())
    );
}

#[test]
fn packaged_cli_decodes_bound_saved_catalog_and_detail_without_current_graph() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let (before, after) = saved_pair(&root);
    let runtime = fixture_runtime(&root);
    assert!(
        !root.join("flow.ts").exists(),
        "There is no live source to reconstruct"
    );
    let unrelated = "e".repeat(64);
    let context = QueryContext::new(
        &runtime,
        Descriptor {
            project: root.clone(),
            generation: Some(unrelated.clone()),
            roots: vec![],
        },
    );
    let cancel = AtomicBool::new(false);
    let (bound, page) = context
        .flow_comparison()
        .open("refs/tags/base", "refs/tags/head", &cancel)
        .unwrap();
    assert_pinned_revision(&root, &bound);
    assert!(page.complete);
    assert_eq!(
        context.descriptor.generation.as_deref(),
        Some(unrelated.as_str())
    );
    assert_ne!(bound.header().head.digest, unrelated);
    let symbol = load_symbol(&page);
    let filtered = bound
        .catalog(
            CatalogPage {
                text: String::new(),
                offset: 1,
                limit: 1,
            },
            &cancel,
        )
        .unwrap();
    assert_eq!((filtered.returned, filtered.offset), (1, 1));
    let detail = bound.detail(&symbol, None, &cancel).unwrap();
    for (side, saved) in [(&detail.before, &before), (&detail.after, &after)] {
        cached_side(
            side,
            ExpectedSource {
                symbol: &symbol,
                content: saved.graph["architecture"]["files"][0]["content"]
                    .as_str()
                    .unwrap(),
            },
        );
    }
    assert_eq!(
        (
            bound.header().base.requested_ref.as_str(),
            bound.header().head.requested_ref.as_str()
        ),
        ("refs/tags/base", "refs/tags/head")
    );
    assert!(!root.join(".codecallnavigator").exists());
}
