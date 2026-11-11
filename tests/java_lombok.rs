//! Real Lombok compiler outputs must retain authored signatures, not generated helpers.
#[path = "support/java.rs"]
mod support;
use std::{
    fs,
    path::{Path, PathBuf},
};
use support::{fixture, install_wrapper, run, target};

fn lombok_acceptance(kind: &str) {
    let root = fixture("java-lombok");
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    fs::copy(
        repository.join("vendor/lombok-1.18.48.jar"),
        root.path().join("lombok.jar"),
    )
    .unwrap();
    if kind == "gradle" {
        install_wrapper(root.path(), &repository.join("fixtures/gradle/gradlew"));
    } else if let Some(command) = std::env::var_os("CODECALLNAVIGATOR_TEST_MAVEN") {
        install_wrapper(root.path(), &PathBuf::from(command));
    }
    let graph = run(vec![target(root.path(), kind)]).unwrap();
    graph.validate().unwrap();
    for signature in [
        "fixture.Flow:recordBuilder() -> fixture.Value$ValueBuilder",
        "fixture.Flow:classBuilder() -> fixture.Item$ItemBuilder",
        "fixture.Flow:childBuilder() -> fixture.Child$ChildBuilder",
        "fixture.Flow:arrays(fixture.Value$ValueBuilder[]) -> fixture.Value$ValueBuilder[]",
        "fixture.Flow:bounded(fixture.Item$ItemBuilder) -> fixture.Item$ItemBuilder",
    ] {
        assert!(
            graph.nodes.iter().any(|node| node.signature == signature),
            "Missing {signature}"
        );
    }
    assert_eq!(
        graph.nodes.len(),
        7,
        "Only seven authored methods should survive"
    );
    let main = graph
        .nodes
        .iter()
        .position(|node| node.signature == "fixture.Flow:main(java.lang.String[]) -> void")
        .unwrap() as u32;
    let builder = graph
        .nodes
        .iter()
        .position(|node| {
            node.signature == "fixture.Flow:recordBuilder() -> fixture.Value$ValueBuilder"
        })
        .unwrap() as u32;
    let leaf = graph
        .nodes
        .iter()
        .position(|node| node.signature == "fixture.Flow:leaf() -> void")
        .unwrap() as u32;
    assert!(
        graph
            .edges
            .iter()
            .any(|edge| edge.from == main && edge.to == builder)
    );
    assert!(
        graph
            .edges
            .iter()
            .any(|edge| edge.from == builder && edge.to == leaf)
    );
}

#[test]
#[ignore = "requires pinned Lombok, Java analyzer, Maven and JDK 25"]
fn maven_lombok_generated_signature_types_preserve_authored_methods() {
    lombok_acceptance("maven");
}

#[test]
#[ignore = "requires pinned Lombok, Java analyzer, Gradle and JDK 25"]
fn gradle_lombok_generated_signature_types_preserve_authored_methods() {
    lombok_acceptance("gradle");
}
