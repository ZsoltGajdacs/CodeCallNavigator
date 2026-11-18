//! Pure wire validation; these records do not stand in for compiler execution.
use super::*;
use serde_json::{Value, json};

fn document() -> Value {
    let empty = json!({"scopes":[],"inputs":[],"links":[]});
    json!({"version":1,"attempt":"current","operations":[{
        "project":"/project","sourceSet":"main","operation":"native:compile",
        "outcome":"no_source","before":empty,"consumedAfter":empty,"after":empty,
        "outputs":{"classes":["/project/classes"],"generatedSources":[]}
    }]})
}

#[test]
fn wire_rejects_stale_unknown_and_incomplete_compilation_evidence() {
    assert!(decode(&serde_json::to_vec(&document()).unwrap(), "current").is_ok());
    let rejected = [
        ("/version", json!(0)),
        ("/attempt", json!("previous")),
        ("/operations/0/outcome", json!("invented")),
        ("/operations/0/project", json!("relative/project")),
        (
            "/operations/0/after",
            json!({"scopes":[],"inputs":[{"path":"/project/Input.java","kind":"file"}],"links":[]}),
        ),
        ("/operations/0/outputs/classes/0", json!("relative/output")),
    ];
    for (path, replacement) in rejected {
        let mut value = document();
        *value.pointer_mut(path).unwrap() = replacement;
        assert!(
            decode(&serde_json::to_vec(&value).unwrap(), "current").is_err(),
            "Accepted {path}"
        );
    }
    let mut value = document();
    value["failure"] = json!({"path":"/project/Changed.java","change":"content_changed","operation":"foreign:compile"});
    assert!(decode(&serde_json::to_vec(&value).unwrap(), "current").is_err());
    let mut value = document();
    value["operations"][0]["outcome"] = json!("unsupported");
    value["operations"][0]["detail"] = json!("Native compiler is configured to skip compilation");
    let evidence = decode(&serde_json::to_vec(&value).unwrap(), "current").unwrap();
    let message = evidence.require_supported().unwrap_err().to_string();
    assert!(message.contains("native:compile") && message.contains("skip compilation"));
}

#[test]
fn typed_diagnostic_preserves_the_changed_input_and_native_operation() {
    let mut value = document();
    value["failure"] = json!({"path":"/project/src/Changed.java","change":"content_changed","operation":"native:compile"});
    let evidence = decode(&serde_json::to_vec(&value).unwrap(), "current").unwrap();
    let error = evidence.require_no_failure().unwrap_err();
    assert!(error.is::<fingerprint::InputsChanged>());
    let message = format!("{error:#}");
    assert!(message.contains("/project/src/Changed.java"));
    assert!(message.contains("native:compile"));
    assert!(message.contains("content changed"));
}

#[test]
fn consumed_aliases_stay_protected_when_unused_directory_bindings_change() {
    let empty = json!({
        "scopes":[{"path":"/project/src","selection":"java","members":[]}],
        "inputs":[{"path":"/project/src","kind":"directory"}],"links":[]
    });
    let before: Snapshot = serde_json::from_value(empty.clone()).unwrap();
    let mut unused = empty.clone();
    unused["links"] = json!([{"path":"/project/src/unused","target":"/old-output"}]);
    let unused: Snapshot = serde_json::from_value(unused).unwrap();
    before.require_same_consumed(&unused).unwrap();

    let mut selected = empty;
    selected["scopes"][0]["members"] = json!(["/project/src/alias/Input.java"]);
    selected["inputs"].as_array_mut().unwrap().push(
        json!({"path":"/project/src/alias/Input.java","kind":"file","sha256":"00".repeat(32)}),
    );
    selected["links"] = json!([{"path":"/project/src/alias","target":"/original"}]);
    let mut retargeted = selected.clone();
    retargeted["links"][0]["target"] = json!("/replacement");
    let before: Snapshot = serde_json::from_value(selected).unwrap();
    let after: Snapshot = serde_json::from_value(retargeted).unwrap();
    let error = before.require_same_consumed(&after).unwrap_err();
    assert!(error.is::<fingerprint::InputsChanged>());
    assert!(error.to_string().contains("/project/src/alias"));
}
