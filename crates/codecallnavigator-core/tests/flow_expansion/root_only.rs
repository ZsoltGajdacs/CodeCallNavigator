//! No selection must preserve ordinary behavior without resolving saved callees.
use super::*;

#[test]
fn unselected_expansion_preserves_ordinary_cleanup_chart_and_owned_call_origins() {
    let root = function(
        "root",
        json!([
            {"kind":"try","resources":[action("acquire")],
             "body":[{"kind":"evaluate","value":call("missing child")},
                     {"kind":"if","condition":{"kind":"value","text":"flag"},
                      "then_branch":[{"kind":"return"}],"else_branch":[action("fallthrough")]}],
             "finally":[{"kind":"evaluate","value":call("missing cleanup")}],
             "cleanup":[action("resource cleanup")]},
            action("caller after")
        ]),
    );
    let ordinary = flow::build(&root).unwrap();
    let result = flow::expand(
        &[FunctionSource {
            function: &root,
            content: SOURCE,
        }],
        &ExpansionRequest::new("root"),
        &AtomicBool::new(false),
    )
    .unwrap();
    result.validate().unwrap();
    assert_eq!(
        serde_json::to_vec(&result.chart).unwrap(),
        serde_json::to_vec(&ordinary).unwrap()
    );
    assert_eq!(result.occurrences.len(), 1);
    assert_eq!(result.sources.len(), 1);
    let occurrence = &result.occurrences[0];
    assert_eq!(occurrence.outcome, flow::OccurrenceOutcome::Root);
    assert!(occurrence.path.0.is_empty());
    let source = occurrence.source.unwrap();
    let snapshot = result.sources.get(source).unwrap();
    assert_eq!(snapshot.content.as_bytes(), SOURCE.as_bytes());
    assert_eq!(snapshot.owner.file, root.file);
    assert!(result.step_origins.iter().all(|origin| {
        origin.occurrence == flow::OccurrenceId(0) && origin.source == Some(source)
    }));
    assert_eq!(
        result
            .step_origins
            .iter()
            .map(|origin| origin.calls.len())
            .sum::<usize>(),
        ordinary
            .steps
            .iter()
            .map(|step| step.calls.len())
            .sum::<usize>()
    );
}
