//! Optional wire advertisement cannot disable the legacy single-function client.
use super::*;
use serde_json::json;
#[test]
fn optional_expansion_advertisement_preserves_ordinary_flow_compatibility() {
    let original = codecallnavigator::query::capabilities();
    let decode = |wire: &serde_json::Value| validate(&serde_json::to_vec(wire).unwrap()).unwrap();
    assert!(decode(&original).flow_expansion);
    let mut cases = vec![];
    let mut missing = original.clone();
    missing.as_object_mut().unwrap().remove("flow_expansion");
    cases.push(missing);
    let mut version = original.clone();
    version["flow_expansion"]["version"] = json!(2);
    cases.push(version);
    let mut operation = original.clone();
    operation["operations"]
        .as_array_mut()
        .unwrap()
        .retain(|name| name != "flow_expansion");
    cases.push(operation);
    let mut required = original.clone();
    required["generation_required"]
        .as_array_mut()
        .unwrap()
        .retain(|name| name != "flow_expansion");
    cases.push(required);
    let mut unbound = original.clone();
    unbound["flow_expansion"]["expected_generation"] = json!(false);
    cases.push(unbound);
    for wire in cases {
        let capabilities = decode(&wire);
        assert!(
            !capabilities.flow_expansion,
            "Malformed optional capability: {wire}"
        );
        assert_eq!(
            capabilities.flow_languages,
            decode(&original).flow_languages
        );
    }
}
