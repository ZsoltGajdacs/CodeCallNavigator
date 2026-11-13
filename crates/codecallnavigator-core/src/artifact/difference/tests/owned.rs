use super::fixture;
use crate::artifact::{
    Artifact,
    difference::{compare, compare_owned},
};
use serde_json::json;

#[test]
fn owned_api_matches_borrowed_and_preserves_borrowed_inputs() {
    let mut before = fixture();
    before.graph["diagnostics"] = json!(["z", "a"]);
    before.graph["unknown"] = json!({"ordered":[2,1],"nullable":null});
    before
        .extensions
        .insert("vendor".into(), json!({"ordered":[2,1]}));
    let mut after = before.clone();
    after.graph["nodes"][0]["label"] = json!("Renamed display");
    after.graph["diagnostics"] = json!(["b", "a", "a"]);
    after.graph["unknown"]
        .as_object_mut()
        .unwrap()
        .remove("nullable");
    after
        .extensions
        .insert("vendor".into(), json!({"ordered":[1,2]}));
    let original_before = serde_json::to_value(&before).unwrap();
    let original_after = serde_json::to_value(&after).unwrap();
    let borrowed = serde_json::to_value(compare(&before, &after).unwrap()).unwrap();
    assert_eq!(serde_json::to_value(&before).unwrap(), original_before);
    assert_eq!(serde_json::to_value(&after).unwrap(), original_after);
    let owned = serde_json::to_value(compare_owned(before, after).unwrap()).unwrap();
    assert_eq!(owned, borrowed);
    assert!(!owned["changes"].as_array().unwrap().is_empty());
}

fn inventories() -> Artifact {
    let mut artifact = fixture();
    let node = artifact.graph["nodes"][0].clone();
    artifact.graph["flow_inventory"] = json!({"version":1,"functions":[{
        "symbol":node["id"],"file":node["file"],"build":node["build"],"project":"App","language":"csharp",
        "span":{"start":0,"end":0},"body":[{"kind":"action","text":"second","calls":[]},{"kind":"action","text":"first","calls":[]}],
        "diagnostics":[],"extension":{"ordered":[2,1]}
    }]});
    artifact.graph["architecture"] = json!({"version":1,"files":[{
        "build":node["build"],"project":"App","file":node["file"],"content":"one","extension":{"ordered":[2,1]}
    }]});
    artifact
}

fn duplicate_inventories(artifact: &mut Artifact) {
    let functions = artifact.graph["flow_inventory"]["functions"]
        .as_array_mut()
        .unwrap();
    let mut duplicate = functions[0].clone();
    duplicate["diagnostics"] = json!(["duplicate"]);
    functions.push(duplicate);
    let files = artifact.graph["architecture"]["files"]
        .as_array_mut()
        .unwrap();
    let mut duplicate = files[0].clone();
    duplicate["content"] = json!("two");
    files.push(duplicate);
}

#[test]
fn duplicate_function_and_file_groups_preserve_singleton_array_transitions() {
    let before = inventories();
    let mut after = before.clone();
    duplicate_inventories(&mut after);
    for (old, new) in [(&before, &after), (&after, &before)] {
        let difference = compare_owned(old.clone(), new.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(&difference).unwrap(),
            serde_json::to_value(compare(old, new).unwrap()).unwrap()
        );
        assert_eq!(difference.changes.len(), 2);
        for change in &difference.changes {
            assert!(change.path.is_empty());
            let (field, inventory) = if change.section == "flow_inventory" {
                ("functions", "flow_inventory")
            } else {
                ("files", "architecture")
            };
            let mut expected_old = old.graph[inventory][field].as_array().unwrap().clone();
            let mut expected_new = new.graph[inventory][field].as_array().unwrap().clone();
            expected_old.sort_by_cached_key(serde_json::Value::to_string);
            expected_new.sort_by_cached_key(serde_json::Value::to_string);
            let payload = |mut values: Vec<serde_json::Value>| {
                if values.len() == 1 {
                    values.remove(0)
                } else {
                    json!(values)
                }
            };
            assert_eq!(change.before.as_ref().unwrap(), &payload(expected_old));
            assert_eq!(change.after.as_ref().unwrap(), &payload(expected_new));
        }
    }
}

#[test]
fn duplicate_group_order_is_canonical_but_nested_flow_and_extensions_are_not_reordered() {
    let mut before = inventories();
    duplicate_inventories(&mut before);
    let mut after = before.clone();
    after.graph["flow_inventory"]["functions"]
        .as_array_mut()
        .unwrap()
        .reverse();
    after.graph["architecture"]["files"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert!(
        compare_owned(before.clone(), after.clone())
            .unwrap()
            .changes
            .is_empty()
    );
    after.graph["flow_inventory"]["functions"][0]["body"]
        .as_array_mut()
        .unwrap()
        .reverse();
    after.graph["architecture"]["files"][0]["extension"]["ordered"] = json!([1, 2]);
    let diff = compare_owned(before.clone(), after.clone()).unwrap();
    assert_eq!(diff.changes.len(), 2);
    assert_eq!(
        serde_json::to_value(&diff).unwrap(),
        serde_json::to_value(compare(&before, &after).unwrap()).unwrap()
    );
}

#[test]
fn owned_and_borrowed_comparison_reject_invalid_artifacts() {
    let valid = inventories();
    let mut invalid_schema = valid.clone();
    invalid_schema.schema = "future".into();
    let mut invalid_reference = valid.clone();
    invalid_reference.graph["flow_inventory"]["functions"][0]["symbol"] = json!("missing");
    let mut duplicate_node = valid.clone();
    let node = duplicate_node.graph["nodes"][0].clone();
    duplicate_node.graph["nodes"]
        .as_array_mut()
        .unwrap()
        .push(node);
    for invalid in [invalid_schema, invalid_reference, duplicate_node] {
        assert!(compare(&invalid, &valid).is_err());
        assert!(compare_owned(invalid.clone(), valid.clone()).is_err());
        assert!(compare_owned(valid.clone(), invalid).is_err());
    }
}

#[test]
fn borrowed_comparison_leaves_large_opaque_extensions_unchanged() {
    let mut before = fixture();
    before.extensions.insert(
        "large".into(),
        json!({"payload":"x".repeat(2*1024*1024),"ordered":[2,1],"nullable":null}),
    );
    before.graph["diagnostics"] = json!(["z", "a"]);
    let mut after = before.clone();
    after.graph["nodes"][0]["label"] = json!("Changed");
    let diff = compare(&before, &after).unwrap();
    assert_eq!(diff.changes.len(), 1);
    assert_eq!(diff.changes[0].section, "nodes");
    assert!(diff.metadata_equal);
    assert_eq!(before.extensions, after.extensions);
    assert_eq!(before.extensions["large"]["ordered"], json!([2, 1]));
    assert_eq!(before.graph["diagnostics"], json!(["z", "a"]));
    assert_eq!(after.graph["diagnostics"], json!(["z", "a"]));
}

#[test]
fn borrowed_comparison_rejects_invalid_originals_without_altering_opaque_values() {
    let mut invalid = fixture();
    invalid.schema = "future".into();
    invalid.extensions.insert(
        "large".into(),
        json!({"payload":"x".repeat(2*1024*1024),"ordered":[2,1]}),
    );
    let valid = fixture();
    for (before, after) in [(&invalid, &valid), (&valid, &invalid)] {
        let error = compare(before, after).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Unsupported graph artifact schema")
        );
    }
    assert_eq!(invalid.schema, "future");
    assert_eq!(invalid.extensions["large"]["ordered"], json!([2, 1]));
    assert_eq!(
        invalid.extensions["large"]["payload"]
            .as_str()
            .unwrap()
            .len(),
        2 * 1024 * 1024
    );
}
