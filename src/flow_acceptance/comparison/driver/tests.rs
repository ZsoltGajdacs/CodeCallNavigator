//! Terminal worker outcomes cannot restart a completed writer or strand native shutdown.
use super::*;
fn driver() -> (Driver, tempfile::TempDir, tempfile::TempDir, Runtime<State>) {
    let project = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let value = super::super::config::tests::value(project.path());
    let config = Configuration::decode(&serde_json::to_vec(&value).unwrap()).unwrap();
    let worker = Worker::start(output.path().into(), Duration::from_secs(1)).unwrap();
    let driver = Driver::new(config, worker, Instant::now());
    let runtime = Runtime::new(
        State::fixture(),
        |_, _| rune::el(),
        (1050.0, 680.0),
        rune::Env::new(true, 1.0),
    );
    (driver, project, output, runtime)
}
fn next_reply(driver: &Driver, deadline: Instant) -> Reply {
    loop {
        if let Some(reply) = driver.io.reply() {
            return reply;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
}

#[test]
fn no_graph_entry_opens_project_picker_before_comparison_controls() {
    let (mut driver, _project, _output, _) = driver();
    let mut runtime = Runtime::new(
        State::new(),
        crate::view::view,
        (1050.0, 680.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    assert!(runtime.find("compare-flows").is_none());
    driver.phase = Phase::Editing(Editor::ProjectPicker);
    let deadline = Instant::now() + Duration::from_secs(2);
    while !runtime.state.flow_comparison.controls.visible {
        assert!(
            Instant::now() < deadline,
            "Actual project-picker entry did not open comparison"
        );
        driver.advance(&mut runtime);
        runtime.frame(1.0 / 60.0);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(driver.phase, Phase::Editing(Editor::Base));
    assert!(runtime.find("comparison-base").is_some());
    assert!(runtime.find("comparison-head").is_some());
    assert!(runtime.state.graph.is_none());
}
#[test]
fn terminal_write_failure_is_terminal_instead_of_retrying_a_finished_worker() {
    let (mut driver, _project, output, mut runtime) = driver();
    std::fs::create_dir(output.path().join("report.json")).unwrap();
    driver.finish(true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    let reply = next_reply(&driver, deadline);
    assert!(
        driver.reply(&mut runtime, reply).is_ok(),
        "Report-write failure must close normally, not restart finish"
    );
    assert_eq!(driver.phase, Phase::Closing);
    assert!(runtime.state.closing);
}
#[test]
fn cancelled_external_reply_cannot_queue_another_terminal_report() {
    let (mut driver, _project, output, mut runtime) = driver();
    driver
        .external(Operation::ArmGitGate, Checkpoint::GateArmed, Phase::Boot)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while !output.path().join("external-01-request.json").exists() {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    driver.fail(&mut runtime, "controlled cancellation".into());
    let reply = next_reply(&driver, deadline);
    assert!(
        driver.reply(&mut runtime, reply).is_ok(),
        "Obsolete cancelled exchange must not finish again"
    );
    assert_eq!(driver.phase, Phase::Writing);
    let terminal = next_reply(&driver, deadline);
    driver.reply(&mut runtime, terminal).unwrap();
    assert_eq!(driver.phase, Phase::Closing);
}

fn catalog_page(
    text: &str,
    symbols: &[&str],
    offset: usize,
) -> std::sync::Arc<crate::engine_client::flow_comparison::Catalog> {
    let endpoint = serde_json::json!({"kind":"commit","requested_ref":"fixture","commit":"a".repeat(40),"path":"/library/commits/graph.json","schema":null,"digest":"b".repeat(64)});
    let records: Vec<_> = symbols.iter().map(|symbol| serde_json::json!({"symbol":symbol,"membership":"both","before":{"status":"present","node":null,"flow":{"status":"recorded"}},"after":{"status":"present","node":null,"flow":{"status":"recorded"}}})).collect();
    std::sync::Arc::new(serde_json::from_value(serde_json::json!({"version":1,"base":endpoint,"head":endpoint,"comparison_id":"c".repeat(64),"kind":"catalog","complete":true,"endpoints":{"before":{"status":"available"},"after":{"status":"available"}},"text":text,"records":records,"offset":offset,"limit":50,"returned":symbols.len(),"total":symbols.len(),"more":false,"next_offset":null})).unwrap())
}

#[test]
fn catalog_selection_waits_for_the_published_query_before_asserting_exact_identity() {
    let (mut driver, _project, _output, mut runtime) = driver();
    driver.selected_case = 1;
    driver.phase = Phase::Selection(Selecting::Wait);
    // Settled-but-unpublished Latest answers are not pending. The visible page
    // still belongs to the previous search until the workspace publishes it.
    assert!(!runtime.state.flow_comparison.catalog_pending());
    runtime.state.flow_comparison.catalog = Some(catalog_page("case/0", &["case/0"], 0));
    driver
        .selection_observed(&runtime, Selecting::Wait)
        .unwrap();
    assert_eq!(driver.phase, Phase::Selection(Selecting::Wait));
    runtime.state.flow_comparison.catalog = Some(catalog_page("case/1", &["case/1"], 1));
    driver
        .selection_observed(&runtime, Selecting::Wait)
        .unwrap();
    assert_eq!(driver.phase, Phase::Selection(Selecting::Wait));
    runtime.state.flow_comparison.catalog = Some(catalog_page("case/1", &["case/1"], 0));
    driver
        .selection_observed(&runtime, Selecting::Wait)
        .unwrap();
    assert_eq!(driver.phase, Phase::Selection(Selecting::Record));
}

#[test]
fn a_published_matching_query_without_its_expected_id_still_fails() {
    let (mut driver, _project, _output, mut runtime) = driver();
    driver.selected_case = 1;
    driver.phase = Phase::Selection(Selecting::Wait);
    runtime.state.flow_comparison.catalog = Some(catalog_page("case/1", &[], 0));
    let error = driver
        .selection_observed(&runtime, Selecting::Wait)
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Exact catalog search did not return its expected ID")
    );
    assert_eq!(driver.phase, Phase::Selection(Selecting::Wait));
}
