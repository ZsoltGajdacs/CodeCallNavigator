//! Optional revision-flow support requires the complete v1 interaction contract.
pub(super) fn supported(value: &serde_json::Value, operations: &[serde_json::Value]) -> bool {
    let comparison = &value["flow_comparison"];
    operations.iter().any(|name| name == "flow_comparison")
        && comparison["version"] == 1
        && comparison["snapshot_binding"] == true
        && comparison["correspondence"] == "exact_identity"
        && comparison["manual_counterpart"] == true
        && comparison["selections"]
            .as_array()
            .is_some_and(|selections| {
                ["catalog", "detail"]
                    .iter()
                    .all(|required| selections.iter().any(|name| name == required))
            })
}

#[cfg(test)]
mod tests {
    use crate::engine_client::capabilities::validate;
    #[test]
    fn flow_comparison_is_optional_but_requires_its_complete_v1_contract() {
        let mut wire = codecallnavigator::query::capabilities();
        let parse =
            |wire: &serde_json::Value| validate(&serde_json::to_vec(wire).unwrap()).unwrap();
        assert!(parse(&wire).flow_comparison);
        for key in [
            "version",
            "selections",
            "snapshot_binding",
            "correspondence",
            "manual_counterpart",
        ] {
            let mut incomplete = wire.clone();
            incomplete["flow_comparison"]
                .as_object_mut()
                .unwrap()
                .remove(key);
            assert!(!parse(&incomplete).flow_comparison, "{key}");
        }
        let mut no_operation = wire.clone();
        no_operation["operations"]
            .as_array_mut()
            .unwrap()
            .retain(|name| name != "flow_comparison");
        assert!(!parse(&no_operation).flow_comparison);
        wire["flow_comparison"]["version"] = 2.into();
        assert!(!parse(&wire).flow_comparison);
        wire.as_object_mut().unwrap().remove("flow_comparison");
        assert!(!parse(&wire).flow_comparison);
        assert!(!parse(&wire).flow_languages.is_empty());
    }
}
