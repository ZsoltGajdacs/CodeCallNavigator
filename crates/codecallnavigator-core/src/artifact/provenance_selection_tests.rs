use super::*;
use serde_json::json;

#[test]
fn opaque_future_selection_survives_basic_provenance_roundtrip() {
    let mut analysis = serde_json::to_value(
        build(
            &[],
            &["/checkout".into()],
            "{}",
            &[("repo".into(), "/checkout".into())],
        )
        .unwrap(),
    )
    .unwrap();
    let future = json!({"version":99,"targets":["keep","order"],"opaque":{"payload":[3,1,2]}});
    analysis["selection"] = future.clone();
    let parsed = read(&json!({"analysis":analysis})).unwrap().unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap()["selection"], future);
}

fn target(path: &str, kind: &str, language: &str) -> crate::discovery::BuildTarget {
    crate::discovery::BuildTarget {
        path: path.into(),
        kind: kind.into(),
        language: language.into(),
        label: "display-only".into(),
    }
}

#[test]
fn same_directory_maven_and_gradle_selection_survives_empty_graph_and_relocation() {
    let selected = vec![
        target("/checkout", "maven", "java"),
        target("/checkout", "gradle", "java"),
    ];
    let analysis =
        build_selected(&[], &selected, "{}", &[("repo".into(), "/checkout".into())]).unwrap();
    assert_eq!(analysis.targets, vec!["@repo/"]);
    let resolved = super::super::selection::resolve(
        analysis.selection.as_ref(),
        &[("repo".into(), "/relocated".into())],
    )
    .unwrap();
    assert_eq!(resolved.len(), 2);
    assert!(
        resolved
            .iter()
            .all(|target| target.path == std::path::Path::new("/relocated"))
    );
    assert_eq!(
        resolved
            .iter()
            .map(|target| target.kind.as_str())
            .collect::<Vec<_>>(),
        vec!["gradle", "maven"]
    );
}

#[test]
fn incomplete_and_unknown_selections_never_fall_back_to_discovery_defaults() {
    let roots = [("repo".into(), "/checkout".into())];
    let selected = vec![
        target("/checkout", "maven", "java"),
        target("/external", "gradle", "java"),
    ];
    let analysis = build_selected(&[], &selected, "{}", &roots).unwrap();
    assert_eq!(analysis.selection.as_ref().unwrap()["complete"], false);
    assert!(super::super::selection::resolve(analysis.selection.as_ref(), &roots).is_err());
    for value in [
        json!({"version":99}),
        json!({"version":1,"complete":true,"targets":[{"path":"@repo/","kind":"future","language":"future"}]}),
        json!({"version":1,"complete":true,"targets":[]}),
    ] {
        assert!(super::super::selection::resolve(Some(&value), &roots).is_err());
    }
    assert!(super::super::selection::resolve(None, &roots).is_err());
}

#[test]
fn canonical_selection_normalizes_only_owned_v1_and_preserves_extension_order() {
    let roots = [("repo".into(), "/checkout".into())];
    let selected = vec![
        target("/checkout", "maven", "java"),
        target("/checkout", "gradle", "java"),
    ];
    let mut artifact =
        super::super::canonical(&crate::model::GraphData::default(), &roots).unwrap();
    artifact.graph["analysis"] =
        serde_json::to_value(build_selected(&[], &selected, "{}", &roots).unwrap()).unwrap();
    artifact.graph["analysis"]["selection"]["opaque"] = json!(["preserve", "this", "order"]);
    let canonical = artifact.bytes().unwrap();
    artifact.graph["analysis"]["selection"]["targets"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(canonical, artifact.bytes().unwrap());
    artifact.graph["analysis"]["selection"] =
        json!({"version":99,"targets":["z","a"],"opaque":[3,1]});
    let saved: Value = serde_json::from_slice(&artifact.bytes().unwrap()).unwrap();
    assert_eq!(
        saved["graph"]["analysis"]["selection"]["targets"],
        json!(["z", "a"])
    );
}

#[test]
fn explicit_null_selection_stays_opaque_in_basic_roundtrip() {
    let mut analysis = serde_json::to_value(
        build(&[], &[], "{}", &[("repo".into(), "/checkout".into())]).unwrap(),
    )
    .unwrap();
    analysis["selection"] = Value::Null;
    let parsed = read(&json!({"analysis":analysis})).unwrap().unwrap();
    assert!(
        serde_json::to_value(parsed)
            .unwrap()
            .get("selection")
            .unwrap()
            .is_null()
    );
}
