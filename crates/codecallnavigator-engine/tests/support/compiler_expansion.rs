//! Compiler-to-artifact-to-expansion assertions shared by existing real SDK cases.
//! This helper reads only the supplied saved artifact; it never analyzes, hydrates,
//! consults live source, or reproduces core eligibility decisions.
use codecallnavigator_core::{
    artifact::{
        Artifact,
        flow::{Ready, Selection, View},
    },
    flow::{
        self, CallChoice, ExecutionTiming, InvocationDispatch, OccurrenceId, OccurrenceOutcome,
    },
};
use std::sync::atomic::AtomicBool;
pub(super) struct Pair<'a> {
    pub caller: &'a str,
    pub call_text: &'a str,
    pub label: &'a str,
    pub callee_label: &'a str,
    pub callee_signature: &'a str,
    pub content: [&'a str; 2],
}
pub(super) fn assert_cached_expansion(saved: &Artifact, pair: Pair<'_>) {
    let before = saved.bytes().unwrap();
    let view = View::new(saved).unwrap();
    let caller = ready(&view, pair.caller);
    let sites: Vec<_> = flow::call_sites(&caller.function)
        .into_iter()
        .filter(|site| {
            caller.content.get(site.site.span.start..site.site.span.end) == Some(pair.call_text)
        })
        .collect();
    assert_eq!(
        sites.len(),
        1,
        "The known compiler call site must be unique"
    );
    let site = &sites[0];
    assert_eq!(site.site.label, pair.label);
    assert_eq!(site.site.targets.len(), 1);
    let target = site.site.targets[0].clone();
    let callee = ready(&view, &target.symbol);
    assert_eq!(callee.node.label, pair.callee_label);
    assert!(
        callee.node.signature.contains(pair.callee_signature),
        "{}",
        callee.node.signature
    );
    assert_compiler_facts(&caller, &callee, site.site);
    let choice = CallChoice {
        site: site.address.clone(),
        target,
    };
    let request = flow::ExpansionRequest::new(&caller.function.symbol).select_root(choice.clone());
    let inputs = [
        flow::FunctionSource {
            function: &caller.function,
            content: caller.content,
        },
        flow::FunctionSource {
            function: &callee.function,
            content: callee.content,
        },
    ];
    let result = flow::expand(&inputs, &request, &AtomicBool::new(false)).unwrap();
    result.validate().unwrap();
    assert_eq!(result.occurrences[1].outcome, OccurrenceOutcome::Expanded);
    assert_eq!(result.occurrences[1].path.0, vec![choice]);
    assert_connected(&result);
    for (index, (ready, expected)) in [(&caller, pair.content[0]), (&callee, pair.content[1])]
        .into_iter()
        .enumerate()
    {
        assert_source(&result, index, ready, expected);
    }
    assert_eq!(
        result.occurrences[0].source == result.occurrences[1].source,
        caller.node.owner == callee.node.owner
    );
    assert_eq!(
        saved.bytes().unwrap(),
        before,
        "Expansion must not mutate cached artifact bytes"
    );
}
fn assert_compiler_facts(caller: &Ready<'_>, callee: &Ready<'_>, site: &flow::CallSite) {
    assert_eq!(
        (
            caller.function.execution,
            callee.function.execution,
            site.invocation.timing,
            site.invocation.dispatch,
            site.invocation.isolated_ordered
        ),
        (
            ExecutionTiming::Synchronous,
            ExecutionTiming::Synchronous,
            ExecutionTiming::Synchronous,
            InvocationDispatch::Direct,
            true
        )
    );
}
fn ready<'a>(view: &'a View<'_>, symbol: &str) -> Box<Ready<'a>> {
    match view.select(symbol) {
        Selection::Ready(ready) => ready,
        other => panic!("Saved compiler flow must be ready: {other:?}"),
    }
}
fn assert_source(result: &flow::ExpandedFlow, index: usize, ready: &Ready<'_>, expected: &str) {
    let source = result
        .sources
        .get(result.occurrences[index].source.unwrap())
        .unwrap();
    assert_eq!(source.content.as_bytes(), expected.as_bytes());
    assert_eq!(
        source.digest,
        blake3::hash(expected.as_bytes()).to_hex().to_string()
    );
    assert_eq!(
        (
            source.owner.build.as_str(),
            source.owner.project.as_str(),
            source.owner.file.as_str(),
            source.owner.language.as_str()
        ),
        (
            ready.node.owner.build,
            ready.node.owner.project,
            ready.node.owner.file,
            ready.node.owner.language
        )
    );
}
fn assert_connected(result: &flow::ExpandedFlow) {
    let child = &result.occurrences[1];
    let call = child.boundaries.call.unwrap();
    let entry = child.boundaries.entry.unwrap();
    assert!(reachable(result, result.chart.entry, entry));
    assert_eq!(
        (
            result.step_origins[call].occurrence,
            result.step_origins[entry].occurrence
        ),
        (OccurrenceId(0), OccurrenceId(1))
    );
    assert!(
        child
            .boundaries
            .returns
            .iter()
            .any(|boundary| reachable(result, entry, *boundary)
                && result.chart.links.iter().any(|link| link.from == *boundary
                    && result.step_origins[link.to].occurrence == OccurrenceId(0))),
        "A reachable callee return must reconnect to the caller"
    );
}
fn reachable(result: &flow::ExpandedFlow, from: usize, to: usize) -> bool {
    let mut pending = vec![from];
    let mut visited = std::collections::HashSet::new();
    while let Some(at) = pending.pop() {
        if !visited.insert(at) {
            continue;
        }
        if at == to {
            return true;
        }
        pending.extend(
            result
                .chart
                .links
                .iter()
                .filter(|link| link.from == at)
                .map(|link| link.to),
        );
    }
    false
}
