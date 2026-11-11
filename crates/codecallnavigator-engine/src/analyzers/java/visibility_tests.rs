//! Declared Java access through actual javac metadata and graph normalization.
use super::*;
use crate::{
    analyzers::AnalyzerRuntime,
    model::Visibility::{Package, Private, Protected, Public},
};

fn analyze_fixture() -> GraphData {
    let directory = tempfile::tempdir().unwrap();
    let runtime = AnalyzerRuntime::discover().unwrap();
    let input = tests::compile_source_fixture(
        &runtime,
        directory.path(),
        include_str!("../../../../../fixtures/java-visibility/Flow.java"),
        "-g",
    );
    let output = tests::run_callback_engine(&runtime, directory.path(), &input);
    let build = BuildTarget {
        path: directory.path().into(),
        language: "java".into(),
        kind: "gradle".into(),
        label: "visibility".into(),
    };
    normalize_bytecode(&build, &output, &[input], None).unwrap()
}

#[test]
#[ignore = "requires the pinned bytecode distribution and installed JDK 25"]
fn compiled_visibility_preserves_declared_class_and_method_access() {
    let graph = analyze_fixture();
    for (container, name, method_access, class_access) in [
        ("fixture.Flow", "publicMethod", Public, Public),
        ("fixture.Flow", "privateMethod", Private, Public),
        ("fixture.Flow", "protectedMethod", Protected, Public),
        ("fixture.Flow", "packageMethod", Package, Public),
        ("fixture.Flow$PublicNested", "member", Private, Public),
        ("fixture.Flow$PrivateNested", "member", Public, Private),
        ("fixture.Flow$ProtectedNested", "member", Public, Protected),
        ("fixture.Flow$PackageNested", "member", Public, Package),
        ("fixture.Flow$Contract", "implicitPublic", Public, Package),
        ("fixture.PackageType", "member", Public, Package),
    ] {
        let node = graph
            .nodes
            .iter()
            .find(|node| node.container == container && node.label == name)
            .unwrap_or_else(|| panic!("Missing {container}:{name}"));
        assert_eq!(node.visibility, Some(method_access), "{}", node.signature);
        assert_eq!(
            node.container_visibility,
            Some(class_access),
            "{}",
            node.signature
        );
    }
    let lambda = graph
        .nodes
        .iter()
        .find(|node| node.signature.contains(":lambda$"))
        .unwrap();
    assert_eq!(
        lambda.visibility, None,
        "Lambda bytecode access is not source visibility"
    );
    let anonymous = graph
        .nodes
        .iter()
        .find(|node| node.label == "run" && node.container.ends_with("$1"))
        .unwrap();
    assert_eq!(
        anonymous.container_visibility, None,
        "Anonymous classes have no access modifier"
    );
    let local = graph
        .nodes
        .iter()
        .find(|node| node.label == "localMember")
        .unwrap();
    assert_eq!(
        local.container_visibility, None,
        "Local classes have no access modifier"
    );
}
