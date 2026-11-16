//! Partial endpoint and cancellation outcomes remain explicit through real controls.
use super::{
    click,
    flow_comparison_fixture::{choose, commit, runtime},
    flow_comparisons::load,
};
use crate::{
    app::State,
    flow_comparison::{Phase, Side},
};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    time::{Duration, Instant},
};
fn wait(ui: &mut rune::Runtime<State>, mut condition: impl FnMut(&State) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition(&ui.state) {
        ui.frame(1.0 / 60.0);
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    ui.frame(1.0 / 60.0);
}
#[test]
fn incompatible_endpoint_membership_and_missing_cached_source_are_not_guessed() {
    use codecallnavigator::artifact;
    let data = crate::flow_scenes::data();
    let (mut ui, project) = runtime(data.clone(), data.clone(), true);
    let saved = artifact::canonical(&data, &[("repo".into(), "/snapshot".into())]).unwrap();
    let mut manifest: serde_json::Value =
        serde_json::from_slice(saved.encode().unwrap().as_ref()).unwrap();
    manifest["schema"] = "future-schema".into();
    fs::write(
        project.path().join(artifact::GRAPH_PATH),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    for args in [
        &["add", artifact::GRAPH_PATH, ".codecallnavigator/objects"][..],
        &["commit", "-qm", "future"],
        &["tag", "-f", "base"],
    ] {
        crate::flow_comparison::tests::git(project.path(), args);
    }
    commit(project.path(), &data, "after-valid");
    fs::remove_file(project.path().join(artifact::GRAPH_PATH)).unwrap();
    load(&mut ui);
    assert!(!ui.state.flow_comparison.catalog.as_ref().unwrap().complete);
    let header = ui.state.flow_comparison.header().unwrap();
    assert_ne!(header.base.commit, header.head.commit);
    assert_eq!(header.base.schema.as_deref(), Some("future-schema"));
    assert_ne!(header.head.schema.as_deref(), Some("future-schema"));
    choose(&mut ui, "load(");
    let pair = ui.state.flow_comparison.current.as_ref().unwrap();
    assert!(matches!(pair.before, Side::Unavailable(_)));
    assert!(matches!(pair.after, Side::Ready(_)));
    assert!(ui.elements().iter().any(|el| {
        el.text
            .as_deref()
            .is_some_and(|text| text.contains("Membership unknown"))
    }));
    assert!(ui.elements().iter().any(|el| {
        el.text
            .as_deref()
            .is_some_and(|text| text.contains("Partial comparison"))
    }));
    let mut missing = data.clone();
    missing.architecture = None;
    let (mut ui, _project) = runtime(missing, data, false);
    load(&mut ui);
    choose(&mut ui, "load(");
    assert!(matches!(
        ui.state.flow_comparison.current.as_ref().unwrap().before,
        Side::Unavailable(_)
    ));
    assert!(ui.elements().iter().any(|el| {
        el.text
            .as_deref()
            .is_some_and(|text| text.contains("source"))
    }));
}
fn blocked_detail(root: &std::path::Path, real: &std::path::Path) -> std::path::PathBuf {
    let path = root.join("blocked-comparison-cli");
    fs::write(
        &path,
        format!(
            r#"#!/usr/bin/env python3
import os,pathlib,subprocess,sys,time
real={real:?}
if sys.argv[1]=='capabilities':os.execv(real,[real,'capabilities'])
for line in sys.stdin:
 if '"detail"' in line:
  pathlib.Path(__file__).with_name('detail.started').write_text('started')
  time.sleep(30)
 answer=subprocess.run([real,'query'],input=line,text=True,stdout=subprocess.PIPE,check=False)
 print(answer.stdout.strip(),flush=True)
"#,
            real = real.to_string_lossy()
        ),
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}
#[test]
fn cancel_control_reaps_the_owned_pending_query_and_does_not_publish_its_pair() {
    let data = crate::flow_scenes::data();
    let (mut ui, project) = runtime(data.clone(), data, true);
    let mut engine = ui.state.runtime.as_ref().unwrap().as_ref().clone();
    engine.engine = blocked_detail(project.path(), &engine.engine);
    ui.state.runtime = Some(std::sync::Arc::new(engine));
    load(&mut ui);
    let symbol = ui.state.flow_comparison.catalog.as_ref().unwrap().records[0]
        .symbol
        .clone();
    click(&mut ui, &format!("comparison-record-{symbol}"));
    wait(&mut ui, |_| project.path().join("detail.started").exists());
    assert!(ui.state.flow_comparison.detail_pending());
    click(&mut ui, "comparison-cancel");
    assert!(ui.state.flow_comparison.current.is_none());
    assert!(matches!(ui.state.flow_comparison.phase, Phase::Closed));
    wait(&mut ui, |state| state.flow_comparison.stopped());
    assert!(ui.state.flow_comparison.current.is_none());
    assert!(ui.find("comparison-load").is_some());
}
