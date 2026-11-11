//! Nested inventory versions are part of the public artifact compatibility boundary.
use codecallnavigator::{
    artifact::{self, Artifact},
    model::GraphData,
};

#[test]
fn encoded_java_context_paths_relocate_even_with_quoted_directory_names() {
    let export = |root: &str| {
        let graph = GraphData {
            nodes: vec![codecallnavigator::model::Node {
                id: format!("{}|fixture.App:run()|void", serde_json::json!([root, format!("{root}/src"), "main"])),
                project: "fixture".into(),
                build: root.into(),
                file: format!("{root}/src/App.java"),
                language: "java".into(),
                label: "run".into(),
                signature: "fixture.App:run() -> void".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        artifact::canonical(&graph, &[("repo".into(), root.into())]).unwrap().bytes().unwrap()
    };
    assert_eq!(export("/first/quoted\"project"), export("/second/renamed"));
}

#[test]
fn incompatible_inventory_versions_are_not_valid_or_comparable_artifacts() {
    let base = artifact::canonical(
        &GraphData::default(),
        &[("repo".into(), "/checkout".into())],
    )
    .unwrap();
    for (name, inventory) in [
        (
            "flow_inventory",
            serde_json::json!({"version":999,"functions":[]}),
        ),
        (
            "architecture",
            serde_json::json!({"version":999,"files":[]}),
        ),
    ] {
        let mut invalid = base.clone();
        invalid.graph[name] = inventory;
        let bytes = serde_json::to_vec(&invalid).unwrap();
        assert!(
            Artifact::from_bytes(&bytes).is_err(),
            "Accepted incompatible {name}"
        );
        assert!(
            artifact::compare(&invalid, &invalid).is_err(),
            "Compared incompatible {name}"
        );
    }
}
