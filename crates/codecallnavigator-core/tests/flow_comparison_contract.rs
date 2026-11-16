use codecallnavigator_core::query::{self, Request};
use serde_json::json;

#[test]
fn flow_comparison_defaults_and_capability_are_additive() {
    let request: Request = serde_json::from_value(json!({"version":query::VERSION,"project":"/repo","query":{"kind":"flow_comparison","base":"HEAD","selection":{"kind":"catalog"}}})).unwrap();
    let value = serde_json::to_value(request).unwrap();
    assert_eq!(value["query"]["head"], "WORKTREE");
    assert_eq!(
        value["query"]["selection"],
        json!({"kind":"catalog","text":"","offset":0,"limit":50})
    );
    let capabilities = query::capabilities();
    assert_eq!(capabilities["flow_comparison"]["version"], 1);
    assert!(
        capabilities["operations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|operation| operation == "flow_comparison")
    );
    assert_eq!(capabilities["comparison"]["version"], 1);
}

#[test]
fn flow_detail_selection_retains_explicit_counterpart_and_rejects_unknown_fields() {
    let query = json!({"kind":"flow_comparison","base":"base-ref","head":"head-ref","expected_comparison":"a".repeat(64),"selection":{"kind":"detail","symbol":"before-id","head_symbol":"after-id"}});
    let request: Request =
        serde_json::from_value(json!({"version":query::VERSION,"project":"/repo","query":query}))
            .unwrap();
    assert_eq!(
        serde_json::to_value(request).unwrap()["query"]["selection"],
        query["selection"]
    );
    let mut invalid = query;
    invalid["selection"]["guess_rename"] = json!(true);
    assert!(
        serde_json::from_value::<Request>(
            json!({"version":query::VERSION,"project":"/repo","query":invalid})
        )
        .is_err()
    );
}
