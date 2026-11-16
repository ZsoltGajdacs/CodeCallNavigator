//! Public expansion wire policy is pure; CLI/process ownership is exercised by engine tests.
use codecallnavigator_core::query::{self, Request};
use serde_json::json;

#[test]
fn expansion_wire_is_explicit_versioned_and_preserves_default_flow() {
    let wire = json!({"version":query::VERSION,"project":"/snapshot","expected_generation":"saved-digest","query":{"kind":"flow_expansion","expansion_version":1,"symbol":"root","selections":[],"limits":{"depth":0}}});
    let request: Request = serde_json::from_value(wire).unwrap();
    let encoded = serde_json::to_value(request).unwrap();
    assert_eq!(encoded["query"]["kind"], "flow_expansion");
    assert_eq!(encoded["query"]["expansion_version"], 1);
    assert_eq!(encoded["query"]["limits"]["depth"], 0);
    assert_eq!(encoded["query"]["limits"]["steps"], 1500);
    let flow = json!({"kind":"flow","symbol":"root"});
    let ordinary: query::Query = serde_json::from_value(flow.clone()).unwrap();
    assert_eq!(serde_json::to_value(ordinary).unwrap(), flow);
}

#[test]
fn expansion_capability_advertises_explicit_binding_and_hard_bounds() {
    let capability = query::capabilities();
    assert!(
        capability["operations"]
            .as_array()
            .unwrap()
            .contains(&json!("flow_expansion"))
    );
    assert!(
        capability["generation_required"]
            .as_array()
            .unwrap()
            .contains(&json!("flow_expansion"))
    );
    assert_eq!(capability["flow_expansion"]["version"], 1);
    assert_eq!(
        capability["flow_expansion"]["maximum_response_bytes"],
        24 * 1024 * 1024
    );
    assert_eq!(capability["flow_expansion"]["maximum_limits"]["depth"], 32);
    assert_eq!(
        capability["flow_expansion"]["default_limits"]["requests"],
        64
    );
}

#[path = "flow_expansion_query/inputs.rs"]
mod inputs;
