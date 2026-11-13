//! Narrow wire/process doubles; graph behavior is covered with the real CLI.
mod support;
use serde_json::json;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
use support::{Client, command};

fn cli(root: &Path, mode: &str) -> PathBuf {
    let source = r#"#!/usr/bin/env python3
import json, sys
mode = MODE_LITERAL
generation = 'a' * 64
operations = ['compare','status','search','symbol','source','explorer','selection','neighbors','flow']
capabilities = dict(query_version=1, generation_binding=True, operations=operations, comparison=dict(version=1,snapshot_binding=True))
if mode != 'legacy':
    capabilities['operations'] = operations + ['freshness']
    capabilities['freshness'] = dict(version=2 if mode=='future' else 1,mode='verified_polling',limited=True)
if sys.argv[1] == 'capabilities':
    print(json.dumps(capabilities))
    sys.exit(0)
request = json.load(sys.stdin)
kind = request['query']['kind']
observation = dict(version=1,generation=generation,input_revision='b'*64,contract_valid=True,input_status='match',reasons=[],reasons_total=0,omitted_reasons=0,coverage=dict(kind='observed_inputs',complete=True,certifies_build=False),mode=dict(kind='verified_polling',limited=True,reconcile_milliseconds=250),verification_milliseconds=1.0)
if kind == 'freshness':
    if mode=='generation': observation['generation']='c'*64
    elif mode=='revision': observation['input_revision']='bad'
    elif mode=='missing_revision': observation.pop('input_revision')
    elif mode=='contract': observation['contract_valid']=False
    elif mode=='oversized':
        observation['reasons']=[dict(code='input_changed',path='x'*8192)]
        observation['reasons_total']=1
legacy = {key:observation[key] for key in ['contract_valid','input_status','reasons','reasons_total','omitted_reasons']}
legacy.update(version=1,digest=generation)
if kind=='freshness' and mode=='mixed_summary': legacy['input_status']='stale'
result = dict(nodes=0,edges=0) if kind=='status' else observation
print(json.dumps(dict(version=1,generation=generation,freshness=legacy,result=result,error=None)))
"#.replace("MODE_LITERAL", &format!("{mode:?}"));
    let path = root.join("wire_cli.py");
    fs::write(&path, source).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[test]
fn optional_missing_or_future_capability_keeps_legacy_tools_usable() {
    for mode in ["legacy", "future"] {
        let root = tempfile::tempdir().unwrap();
        let mut client = Client::launch(command(&cli(root.path(), mode), root.path()));
        assert_eq!(
            client.graph("graph_status", json!({}))["result"]["nodes"],
            0
        );
        let reply = client.call("graph_freshness", json!({}));
        assert_eq!(reply["isError"], true, "{mode}: {reply}");
        assert_eq!(
            reply["structuredContent"]["error"]["code"],
            "unsupported_capability"
        );
        assert!(client.request("ping", json!({})).get("result").is_some());
    }
}

#[test]
fn malformed_observation_successes_are_errors_and_server_stays_usable() {
    for mode in [
        "generation",
        "revision",
        "missing_revision",
        "contract",
        "oversized",
        "mixed_summary",
    ] {
        let root = tempfile::tempdir().unwrap();
        let mut client = Client::launch(command(&cli(root.path(), mode), root.path()));
        let reply = client.call("graph_freshness", json!({}));
        assert_eq!(reply["isError"], true, "{mode}: {reply}");
        assert_eq!(
            reply["structuredContent"]["error"]["code"],
            "cli_transport_error"
        );
        assert_eq!(
            client.graph("graph_status", json!({}))["result"]["nodes"],
            0
        );
        assert!(client.request("ping", json!({})).get("result").is_some());
    }
}
