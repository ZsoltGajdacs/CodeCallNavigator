use super::*;
use crate::{
    artifact,
    model::{GraphData, Node},
};
use serde_json::json;

fn fixture() -> artifact::Artifact {
    let data = GraphData {
        nodes: vec![Node {
            id: "f".into(),
            language: "csharp".into(),
            build: "/tmp/diff/App.csproj".into(),
            project: "App".into(),
            file: "/tmp/diff/F.cs".into(),
            label: "F".into(),
            signature: "F()".into(),
            kind: "method".into(),
            ..Node::default()
        }],
        ..GraphData::default()
    };
    artifact::canonical(&data, &[("repo".into(), "/tmp/diff".into())]).unwrap()
}

#[test]
fn reports_field_changes_and_distinguishes_missing_from_null() {
    let before = fixture();
    let mut after = before.clone();
    after.graph["nodes"][0]["label"] = json!("Changed");
    after.graph["nodes"][0]["visibility"] = json!("public");
    after.extensions.insert("nullable".into(), json!(null));
    let diff = compare(&before, &after).unwrap();
    assert!(!diff.graph_equal);
    assert!(!diff.metadata_equal);
    assert_eq!(diff.summary["/graph/nodes"].changed_entities, 1);
    assert_eq!(diff.summary["/graph/nodes"].records, 2);
    let encoded = serde_json::to_value(&diff.changes).unwrap();
    let nullable = encoded
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["section"] == "extensions")
        .unwrap();
    assert!(nullable.get("before").is_none());
    assert_eq!(nullable.get("after"), Some(&json!(null)));
}

#[test]
fn reorders_only_documented_unordered_collections() {
    let mut before = fixture();
    before.graph["diagnostics"] = json!(["a", "b"]);
    before
        .extensions
        .insert("ordered".into(), json!(["a", "b"]));
    let mut after = before.clone();
    after.graph["diagnostics"] = json!(["b", "a"]);
    assert!(compare(&before, &after).unwrap().changes.is_empty());
    after.extensions.insert("ordered".into(), json!(["b", "a"]));
    assert_eq!(compare(&before, &after).unwrap().changes.len(), 1);
}

#[test]
fn identity_changes_are_add_remove_and_relationship_variants_are_preserved() {
    let mut before = fixture();
    let id = before.graph["nodes"][0]["id"].clone();
    before.graph["edges"] = json!([
        {"from":id,"to":id,"kind":"call","certainty":"direct","provenance":"compiler","count":1},
        {"from":id,"to":id,"kind":"call","certainty":"possible","provenance":"inference","count":2}
    ]);
    let mut after = before.clone();
    after.graph["edges"][0]["count"] = json!(3);
    let diff = compare(&before, &after).unwrap();
    assert_eq!(diff.summary["/graph/edges"].changed_entities, 1);
    assert_eq!(
        diff.changes[0]
            .before
            .as_ref()
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        2
    );
    after.graph["nodes"][0]["id"] = json!("renamed");
    after.graph["edges"] = json!([]);
    let diff = compare(&before, &after).unwrap();
    assert_eq!(diff.summary["/graph/nodes"].added, 1);
    assert_eq!(diff.summary["/graph/nodes"].removed, 1);
}

#[test]
fn all_inventory_sections_and_extensions_remain_observable() {
    let mut before = fixture();
    let node = before.graph["nodes"][0].clone();
    before.graph["flow_inventory"] = json!({"version":1,"functions":[{
        "symbol":node["id"],"file":node["file"],"build":node["build"],"project":"App","language":"csharp",
        "span":{"start":0,"end":0},"body":[{"kind":"action","text":"first","calls":[]},{"kind":"action","text":"second","calls":[]}],"diagnostics":[]
    }]});
    before.graph["architecture"] = json!({"version":1,"files":[{"build":node["build"],"project":"App","file":node["file"],"content":"before"}]});
    before.graph["entry_points"] =
        json!({"candidates":[node["id"]],"application_starts":[],"framework_entries":[]});
    before.graph["boundaries"] = json!({"version":1,"routes":[],"uses":[]});
    let mut after = before.clone();
    after.graph["flow_inventory"]["functions"][0]["body"]
        .as_array_mut()
        .unwrap()
        .reverse();
    after.graph["architecture"]["files"][0]["content"] = json!("after");
    after.graph["entry_points"]["candidates"] = json!([]);
    after.graph["entry_points"]["application_starts"] = json!([node["id"]]);
    after.graph["boundaries"]["routes"] = json!([{"symbol":node["id"],"build":node["build"],"application":"app","framework":"spring","kind":"http","path":"/x","methods":["GET"],"site":{"file":node["file"],"line":1},"order":0}]);
    after.graph["diagnostics"] = json!(["new diagnostic"]);
    after.graph["unknown_graph_field"] = json!({"ordered":[2,1]});
    let diff = compare(&before, &after).unwrap();
    for section in [
        "flow_inventory",
        "architecture",
        "entry_points",
        "boundaries",
        "diagnostics",
        "unknown_graph_field",
    ] {
        assert!(
            diff.summary.contains_key(&format!("/graph/{section}")),
            "Missing {section}"
        );
    }
    assert_eq!(diff.summary["/graph/entry_points"].changed_entities, 2);
    assert!(
        diff.changes
            .iter()
            .any(|r| r.section == "flow_inventory" && r.path == "/body")
    );
}

#[test]
fn metadata_only_changes_leave_graph_equal_and_escape_pointer_fields() {
    let mut before = fixture();
    before.graph["analysis"] = json!({"version":1,"targets":[],"roots":["repo"],"engine_versions":"","inputs":[],"limitations":[]});
    before
        .extensions
        .insert("vendor".into(), json!({"a/b~c":1}));
    let mut after = before.clone();
    after.graph["analysis"]["engine_versions"] = json!("new");
    after.extensions.insert("vendor".into(), json!({"a/b~c":2}));
    let diff = compare(&before, &after).unwrap();
    assert!(diff.graph_equal);
    assert!(!diff.metadata_equal);
    assert!(diff.changes.iter().any(|r| r.path == "/a~1b~0c"));
}

#[test]
fn graph_extension_fields_never_alias_artifact_metadata() {
    let mut before = fixture();
    before.graph["extensions"] = json!({"same":1});
    before.extensions.insert("same".into(), json!(1));
    let mut after = before.clone();
    after.graph["extensions"]["same"] = json!(2);
    after.extensions.insert("same".into(), json!(2));
    let diff = compare(&before, &after).unwrap();
    assert!(!diff.graph_equal);
    assert!(!diff.metadata_equal);
    assert_eq!(diff.summary["/graph/extensions"].records, 1);
    assert_eq!(diff.summary["/same"].records, 1);
    assert_eq!(
        diff.changes
            .iter()
            .filter(|c| c.scope == Scope::Graph)
            .count(),
        1
    );
    assert_eq!(
        diff.changes
            .iter()
            .filter(|c| c.scope == Scope::Artifact)
            .count(),
        1
    );
}
