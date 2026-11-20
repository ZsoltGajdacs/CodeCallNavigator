//! Pure fresh-resource compatibility policy; real CLI/compiler coverage lives in typescript_facts.
use super::validate_capabilities;
use serde_json::json;

#[test]
fn stale_missing_and_future_typescript_adapters_require_matching_resources() {
    for version in [
        None,
        Some(json!("6")),
        Some(json!("8")),
        Some(json!(7)),
        Some(json!(null)),
    ] {
        let mut versions =
            json!({"build_evidence":"1","boundary_rules":"1","input_policy":"scoped_v1"});
        if let Some(version) = version {
            versions["adapter"] = version;
        }
        let error =
            validate_capabilities(&versions).expect_err("Stale/unknown adapter must be rejected");
        assert!(
            error
                .to_string()
                .contains("rebuild matching analyzer resources")
        );
    }
}

#[test]
fn current_typescript_adapter_is_accepted_with_unchanged_prepare_contract() {
    let current =
        json!({"adapter":"7","build_evidence":"1","boundary_rules":"1","input_policy":"scoped_v1"});
    validate_capabilities(&current).unwrap();
    for boundary in [json!(null), json!(1), json!("2")] {
        assert!(
            validate_capabilities(
                &json!({"adapter":"7","build_evidence":"1","boundary_rules":boundary,"input_policy":"scoped_v1"})
            )
            .is_err()
        );
    }
    assert!(validate_capabilities(&json!({"adapter":"7"})).is_err());
    for policy in [json!(null), json!("legacy"), json!("future"), json!(1)] {
        assert!(
            validate_capabilities(
                &json!({"adapter":"7","build_evidence":"1","boundary_rules":"1","input_policy":policy})
            )
            .is_err()
        );
    }
    assert!(validate_capabilities(&json!({"adapter":"7","boundary_rules":"1"})).is_err());
    for evidence in [json!(null), json!("2"), json!(1)] {
        let mut stale = current.clone();
        stale["build_evidence"] = evidence;
        assert!(validate_capabilities(&stale).is_err());
    }
}
