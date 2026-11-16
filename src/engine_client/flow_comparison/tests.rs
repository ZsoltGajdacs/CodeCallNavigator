use super::*;
use crate::{
    engine_client::{Descriptor, QueryContext},
    runtime::Runtime,
};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, sync::atomic::AtomicBool};

fn endpoint(reference: &str) -> Value {
    json!({"kind":"commit","requested_ref":reference,"commit":"a".repeat(64),"path":".codecallnavigator/graph.json","schema":null,"digest":"b".repeat(64)})
}
fn header() -> Value {
    json!({"version":1,"base":endpoint("base"),"head":endpoint("head"),"comparison_id":"c".repeat(64)})
}
fn catalog() -> Value {
    let mut wire = header();
    wire.as_object_mut().unwrap().extend(json!({"kind":"catalog","complete":false,"endpoints":{"before":{"status":"available"},"after":{"status":"unavailable","reason":"future_reason","message":"unsupported snapshot"}},"text":"","records":[{"symbol":"one","membership":"unknown","before":{"status":"present","node":null,"flow":{"status":"unavailable","reason":"missing_inventory","message":"Refresh"}},"after":{"status":"unknown","reason":"future_reason","message":"unknown"}}],"offset":0,"limit":50,"returned":1,"total":1,"more":false,"next_offset":null}).as_object().unwrap().clone());
    wire
}
fn fake(root: &std::path::Path, reply: Value) -> QueryContext {
    let path = root.join("engine");
    let capabilities = codecallnavigator::query::capabilities().to_string();
    let script = format!(
        r#"#!/usr/bin/env python3
import json,pathlib,sys
if sys.argv[1]=='capabilities':
 print({capabilities:?})
 sys.exit(0)
for line in sys.stdin:
 req=json.loads(line)
 with pathlib.Path(__file__).with_name('requests').open('a') as log: log.write(json.dumps(req)+'\n')
 payload=json.loads({reply:?})
 result=payload.get(req['query']['selection']['kind'],payload)
 if req['query'].get('expected_comparison'): result=payload.get('_bound_result',result)
 if payload.get('_delay'):
  import time
  time.sleep(20)
 error=payload.get('_error') if req['query'].get('expected_comparison') else None
 print(json.dumps({{'version':2,'generation':payload.get('_generation','b'*64),'freshness':None,'result':None if error else result,'error':error}}),flush=True)
"#,
        reply = reply.to_string()
    );
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    QueryContext::new(
        &Runtime {
            engine: path,
            resources: root.into(),
            dot: Default::default(),
        },
        Descriptor {
            project: root.into(),
            generation: Some("current-generation".into()),
            roots: vec![],
        },
    )
}
#[test]
fn comparison_client_clears_current_generation_and_binds_every_followup() {
    let root = tempfile::tempdir().unwrap();
    let context = fake(root.path(), catalog());
    let cancel = AtomicBool::new(false);
    let (bound, page) = context
        .flow_comparison()
        .open("base", "head", &cancel)
        .unwrap();
    assert_eq!(page.header.base.commit.as_ref().unwrap().len(), 64);
    assert!(!page.complete);
    assert!(matches!(page.records[0].membership, Membership::Unknown));
    bound.catalog(CatalogPage::default(), &cancel).unwrap();
    let requests = fs::read_to_string(root.path().join("requests")).unwrap();
    let rows: Vec<Value> = requests
        .lines()
        .map(|row| serde_json::from_str(row).unwrap())
        .collect();
    assert!(rows.iter().all(|row| row["expected_generation"].is_null()));
    assert!(rows[0]["query"]["expected_comparison"].is_null());
    assert_eq!(rows[1]["query"]["expected_comparison"], "c".repeat(64));
}
#[test]
fn bound_client_rejects_changed_binding_before_publishing_a_page() {
    let root = tempfile::tempdir().unwrap();
    let context = fake(root.path(), catalog());
    let cancel = AtomicBool::new(false);
    let (bound, _) = context
        .flow_comparison()
        .open("base", "head", &cancel)
        .unwrap();
    let mut changed = bound.header().clone();
    changed.comparison_id = "d".repeat(64);
    assert!(bound.check(&changed).is_err());
}
#[test]
fn detail_states_keep_manual_correspondence_and_unknown_reasons() {
    let mut wire = header();
    wire.as_object_mut().unwrap().extend(json!({"kind":"detail","correspondence":{"kind":"manual","symbol":"old","head_symbol":"new"},"before":{"status":"missing"},"after":{"status":"unavailable","membership":"unknown","node":null,"reason":"future_reason","message":"unavailable"}}).as_object().unwrap().clone());
    let detail: Detail = serde_json::from_value(wire).unwrap();
    assert!(matches!(
        detail.correspondence.kind,
        CorrespondenceKind::Manual
    ));
    assert!(matches!(detail.before, DetailSide::Missing));
    let DetailSide::Unavailable(side) = detail.after else {
        panic!()
    };
    assert_eq!(side.failure.reason, "future_reason");
    assert!(matches!(side.membership, Presence::Unknown));
}

fn detail_wire() -> Value {
    let mut wire = header();
    wire.as_object_mut().unwrap().extend(json!({"kind":"detail","correspondence":{"kind":"manual","symbol":"old","head_symbol":"new"},"before":{"status":"missing"},"after":{"status":"unavailable","membership":"unknown","node":null,"reason":"missing_source","message":"Prepare the historical artifact"}}).as_object().unwrap().clone());
    wire
}
#[test]
fn bound_detail_sends_explicit_counterpart_and_keeps_unavailable_states() {
    let root = tempfile::tempdir().unwrap();
    let context = fake(
        root.path(),
        json!({"catalog":catalog(),"detail":detail_wire()}),
    );
    let cancel = AtomicBool::new(false);
    let (bound, _) = context
        .flow_comparison()
        .open("base", "head", &cancel)
        .unwrap();
    let detail = bound.detail("old", Some("new"), &cancel).unwrap();
    assert!(matches!(detail.before, DetailSide::Missing));
    let DetailSide::Unavailable(side) = detail.after else {
        panic!()
    };
    assert_eq!(side.failure.reason, "missing_source");
    let rows = fs::read_to_string(root.path().join("requests")).unwrap();
    let last: Value = serde_json::from_str(rows.lines().last().unwrap()).unwrap();
    assert_eq!(last["query"]["selection"]["head_symbol"], "new");
    assert_eq!(last["query"]["expected_comparison"], "c".repeat(64));
}
#[test]
fn comparison_mismatch_retains_machine_readable_error_for_reload() {
    let root = tempfile::tempdir().unwrap();
    let context = fake(
        root.path(),
        json!({"catalog":catalog(),"_error":{"code":"comparison_mismatch","message":"Reload comparison"}}),
    );
    let cancel = AtomicBool::new(false);
    let (bound, _) = context
        .flow_comparison()
        .open("base", "head", &cancel)
        .unwrap();
    let Error::Cli(error) = bound.catalog(CatalogPage::default(), &cancel).unwrap_err() else {
        panic!()
    };
    assert_eq!(error.code, "comparison_mismatch");
    assert_eq!(error.message, "Reload comparison");
}
#[test]
fn ready_source_and_target_metadata_are_owned_by_the_requested_side() {
    use codecallnavigator::flow::{self, FunctionFlowIR};
    let function = FunctionFlowIR {
        symbol: "new".into(),
        file: "F.cs".into(),
        build: "B".into(),
        project: "P".into(),
        language: "csharp".into(),
        ..Default::default()
    };
    let chart = flow::build(&function).unwrap();
    let node = json!({"symbol":"new","label":"New","signature":"New()","line":1,"body_unavailable":false,"owner":{"build":"B","project":"P","file":"F.cs","language":"csharp"}});
    let target = json!({"symbol":"target","label":"Target","signature":"Target()","line":2,"body_unavailable":true,"owner":{"build":"B","project":"P","file":"F.cs","language":"csharp"}});
    let mut detail = detail_wire();
    detail["after"] = json!({"status":"ready","membership":"present","node":node,"function":function,"chart":chart,"source":{"origin":"analyzed","symbol":"new","file":"F.cs","line":1,"content":"\u{feff}é😀\r\n"},"targets":[target]});
    let parsed: Detail = serde_json::from_value(detail.clone()).unwrap();
    validation::detail(&parsed, "old", "new").unwrap();
    let DetailSide::Ready(ready) = parsed.after else {
        panic!()
    };
    assert_eq!(ready.source.content, "\u{feff}é😀\r\n");
    assert!(ready.targets[0].body_unavailable);
    detail["after"]["source"]["symbol"] = json!("wrong-side");
    let wrong: Detail = serde_json::from_value(detail).unwrap();
    assert!(validation::detail(&wrong, "old", "new").is_err());
}

#[test]
fn bound_page_rejects_an_actual_cli_reply_from_another_comparison() {
    let root = tempfile::tempdir().unwrap();
    let mut changed = catalog();
    changed["comparison_id"] = json!("d".repeat(64));
    let context = fake(
        root.path(),
        json!({"catalog":catalog(),"_bound_result":changed}),
    );
    let cancel = AtomicBool::new(false);
    let (bound, _) = context
        .flow_comparison()
        .open("base", "head", &cancel)
        .unwrap();
    assert!(
        bound
            .catalog(CatalogPage::default(), &cancel)
            .unwrap_err()
            .to_string()
            .contains("another flow comparison binding")
    );
}
#[test]
fn cancelling_a_comparison_uses_the_existing_owned_query_process() {
    use std::{
        sync::{Arc, atomic::Ordering},
        time::{Duration, Instant},
    };
    let root = tempfile::tempdir().unwrap();
    let context = fake(root.path(), json!({"catalog":catalog(),"_delay":true}));
    let cancel = Arc::new(AtomicBool::new(false));
    let trigger = cancel.clone();
    let marker = root.path().join("requests");
    let worker = std::thread::spawn(move || {
        let started = Instant::now();
        while !marker.exists() && started.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(5));
        }
        trigger.store(true, Ordering::Release);
    });
    let result = context.flow_comparison().open("base", "head", &cancel);
    worker.join().unwrap();
    assert!(result.err().unwrap().to_string().contains("cancelled"));
}

#[test]
fn mixed_envelope_generation_is_rejected_before_exposing_catalog() {
    let root = tempfile::tempdir().unwrap();
    let context = fake(
        root.path(),
        json!({"catalog":catalog(),"_generation":"wrong-generation"}),
    );
    let result = context
        .flow_comparison()
        .open("base", "head", &AtomicBool::new(false));
    assert!(
        result
            .err()
            .unwrap()
            .to_string()
            .contains("generation differs")
    );
}
#[test]
fn endpoint_kinds_must_match_the_requested_reference_roles() {
    let mut wire = header();
    wire["base"]["kind"] = json!("worktree");
    wire["base"]["commit"] = Value::Null;
    let wrong: Header = serde_json::from_value(wire).unwrap();
    assert!(wrong.validate("base", "head").is_err());
    let mut wire = header();
    wire["head"]["requested_ref"] = json!("WORKTREE");
    let wrong: Header = serde_json::from_value(wire.clone()).unwrap();
    assert!(wrong.validate("base", "WORKTREE").is_err());
    wire["head"]["kind"] = json!("worktree");
    wire["head"]["commit"] = Value::Null;
    let valid: Header = serde_json::from_value(wire).unwrap();
    valid.validate("base", "WORKTREE").unwrap();
}

fn malformed_catalogs() -> Vec<(&'static str, Value)> {
    let mut node_mismatch = catalog();
    node_mismatch["records"][0]["before"] = json!({"status":"present","node":{"symbol":"other","label":"Other","signature":"Other()","line":1,"body_unavailable":false,"owner":{"build":"B","project":"P","file":"F.cs","language":"csharp"}},"flow":{"status":"unavailable","reason":"missing_inventory","message":"Refresh"}});
    let mut duplicate = catalog();
    let record = duplicate["records"][0].clone();
    duplicate["records"] = json!([record, record]);
    duplicate["returned"] = json!(2);
    duplicate["total"] = json!(2);
    let mut unsorted = duplicate.clone();
    unsorted["records"][0]["symbol"] = json!("two");
    let mut unavailable_known = catalog();
    unavailable_known["records"][0]["after"] = json!({"status":"missing"});
    let mut available_unknown = catalog();
    available_unknown["records"][0]["before"] = available_unknown["records"][0]["after"].clone();
    let mut incomplete = catalog();
    incomplete["complete"] = json!(true);
    let mut membership = catalog();
    membership["records"][0]["membership"] = json!("added");
    let mut absent = catalog();
    absent["records"][0]["before"] = json!({"status":"missing"});
    let mut count = catalog();
    count["total"] = json!(0);
    vec![
        ("node symbol", node_mismatch),
        ("duplicate IDs", duplicate),
        ("unsorted IDs", unsorted),
        ("unavailable endpoint known side", unavailable_known),
        ("available endpoint unknown side", available_unknown),
        ("complete flag", incomplete),
        ("membership", membership),
        ("total count", count),
        ("no known present identity", absent),
    ]
}
#[test]
fn malformed_v1_catalogs_are_rejected_before_publishing_records() {
    let cancel = AtomicBool::new(false);
    let mut accepted = Vec::new();
    for (name, wire) in malformed_catalogs() {
        let root = tempfile::tempdir().unwrap();
        let context = fake(root.path(), wire);
        if context
            .flow_comparison()
            .open("base", "head", &cancel)
            .is_ok()
        {
            accepted.push(name);
        }
    }
    assert!(
        accepted.is_empty(),
        "Accepted malformed v1 catalogs: {accepted:?}"
    );
}

#[test]
fn valid_membership_and_empty_pages_beyond_total_remain_accepted() {
    let present = json!({"status":"present","node":null,"flow":{"status":"unavailable","reason":"missing_inventory","message":"Refresh"}});
    let missing = json!({"status":"missing"});
    for (membership, before, after) in [
        ("both", present.clone(), present.clone()),
        ("added", missing.clone(), present.clone()),
        ("removed", present, missing),
    ] {
        let mut wire = catalog();
        wire["complete"] = json!(true);
        wire["endpoints"]["after"] = json!({"status":"available"});
        wire["records"][0] =
            json!({"symbol":"one","membership":membership,"before":before,"after":after});
        let mut empty = wire.clone();
        empty["offset"] = json!(42);
        empty["returned"] = json!(0);
        empty["records"] = json!([]);
        let root = tempfile::tempdir().unwrap();
        let context = fake(root.path(), json!({"catalog":wire,"_bound_result":empty}));
        let cancel = AtomicBool::new(false);
        let (bound, page) = context
            .flow_comparison()
            .open("base", "head", &cancel)
            .unwrap();
        assert_eq!(page.records.len(), 1);
        let page = bound
            .catalog(
                CatalogPage {
                    offset: 42,
                    ..Default::default()
                },
                &cancel,
            )
            .unwrap();
        assert!(page.records.is_empty());
    }
}
