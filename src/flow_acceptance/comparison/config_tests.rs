use super::*;
pub fn value(project: &std::path::Path) -> serde_json::Value {
    serde_json::json!({"version":2,"project":project,"base_ref":"refs/tags/fixture-before","head_ref":"refs/tags/fixture-after","expected_viewport":[1050,680],"timeout_seconds":120,"initial":{"comparison_id":"a".repeat(64),"base_commit":"b".repeat(40),"head_commit":"c".repeat(40),"base_digest":"d".repeat(64),"head_digest":"e".repeat(64)},"reloaded":{"comparison_id":"f".repeat(64),"base_commit":"b".repeat(40),"head_commit":"d".repeat(40),"base_digest":"d".repeat(64),"head_digest":"e".repeat(64)},"cases":cases()})
}
#[test]
fn misleading_case_roles_unbounded_sources_and_unknown_configuration_fields_are_rejected() {
    let project = tempfile::tempdir().unwrap();
    let valid = value(project.path());
    Configuration::decode(&serde_json::to_vec(&valid).unwrap()).unwrap();
    for index in [2, 3, 4] {
        let mut invalid = valid.clone();
        invalid["cases"][index]["before"] = valid["cases"][0]["before"].clone();
        invalid["cases"][index]["after"] = valid["cases"][0]["after"].clone();
        assert!(Configuration::decode(&serde_json::to_vec(&invalid).unwrap()).is_err());
    }
    let mut invalid = valid.clone();
    invalid["cases"][0]["after"] = invalid["cases"][0]["before"].clone();
    assert!(Configuration::decode(&serde_json::to_vec(&invalid).unwrap()).is_err());
    let mut invalid = valid.clone();
    invalid["cases"][0]["before"]["source"] = "x".repeat(512 * 1024 + 1).into();
    assert!(Configuration::decode(&serde_json::to_vec(&invalid).unwrap()).is_err());
    let mut invalid = valid;
    invalid["unknown"] = true.into();
    assert!(Configuration::decode(&serde_json::to_vec(&invalid).unwrap()).is_err());
}
pub fn cases() -> serde_json::Value {
    let graph = crate::flow_scenes::data();
    let mut chart =
        codecallnavigator::flow::build(&graph.flow_inventory.as_ref().unwrap().functions[0])
            .unwrap();
    for step in &mut chart.steps {
        for call in &mut step.calls {
            for target in &mut call.targets {
                target.symbol = "case/6".into();
            }
        }
    }
    let source = graph.architecture.unwrap().files.remove(0).content;
    let template =
        serde_json::json!({"status":"ready","symbol":"template","source":source,"chart":chart});
    serde_json::Value::Array(
        CaseKind::ORDER
            .into_iter()
            .enumerate()
            .map(|(index, kind)| example_case(kind, index, &template))
            .collect(),
    )
}
fn example_case(kind: CaseKind, index: usize, template: &serde_json::Value) -> serde_json::Value {
    let symbol = if kind == CaseKind::Manual {
        "case/0".to_string()
    } else {
        format!("case/{index}")
    };
    let mut before = template.clone();
    before["symbol"] = symbol.clone().into();
    before["chart"]["symbol"] = symbol.clone().into();
    let mut after = before.clone();
    let head = (kind == CaseKind::Manual).then(|| "case/other".to_string());
    if let Some(head) = &head {
        after["symbol"] = head.clone().into();
        after["chart"]["symbol"] = head.clone().into();
    }
    if kind == CaseKind::Changed {
        after["source"] = format!("{}// changed", template["source"].as_str().unwrap()).into();
        after["chart"]["steps"][0]["label"] = "changed marker".into();
    }
    let before = match kind {
        CaseKind::Added => serde_json::json!({"status":"missing"}),
        CaseKind::Legacy => serde_json::json!({"status":"unavailable","reason":"missing_source"}),
        _ => before,
    };
    if kind == CaseKind::Removed {
        after = serde_json::json!({"status":"missing"});
    }
    serde_json::json!({"kind":kind_name(kind),"symbol":symbol,"head_symbol":head,"before":before,"after":after})
}
fn kind_name(kind: CaseKind) -> &'static str {
    match kind {
        CaseKind::Changed => "changed",
        CaseKind::Unchanged => "unchanged",
        CaseKind::Added => "added",
        CaseKind::Removed => "removed",
        CaseKind::Legacy => "legacy",
        CaseKind::Manual => "manual",
        CaseKind::Target => "target",
    }
}
