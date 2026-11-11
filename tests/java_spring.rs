//! Actual Spring libraries, compiler-owned types and shared graph/flow inference.
#[path = "support/java.rs"]
mod support;
use codecallnavigator::{flow, model::GraphData};
use std::collections::BTreeSet;

fn inferred(graph: &GraphData) -> Vec<&codecallnavigator::model::Edge> {
    graph
        .edges
        .iter()
        .filter(|edge| edge.provenance == "codecallnavigator.spring.event.v1")
        .collect()
}

#[test]
#[ignore = "requires pinned Java analyzer, Maven, JDK25 and cached Spring6.2.5"]
fn spring_dispatch_is_compiler_bound_possible_and_matches_flow_sites() {
    let directory = support::fixture("spring-events");
    if let Some(command) = std::env::var_os("CODECALLNAVIGATOR_TEST_MAVEN") {
        support::install_wrapper(directory.path(), &std::path::PathBuf::from(command));
    }
    let graph = support::run(vec![support::target(directory.path(), "maven")]).unwrap();
    assert_listener_catalog(&graph);
    let edges = inferred(&graph);
    assert_eq!(
        edges.len(),
        16,
        "Two payload publishers each reach seven listeners; Started reaches two"
    );
    let mut actual = BTreeSet::new();
    for edge in edges {
        let from = &graph.nodes[edge.from as usize];
        let to = &graph.nodes[edge.to as usize];
        assert_eq!(edge.certainty, "possible");
        assert_eq!(
            edge.classification(),
            codecallnavigator::model::RelationshipClassification {
                kind: codecallnavigator::model::RelationshipKind::FrameworkLifecycle,
                resolution: codecallnavigator::model::TargetResolution::Possible,
            }
        );
        assert_eq!(edge.count as usize, edge.evidence.len());
        assert_supported(&from.label, &to.label);
        let expected_count = if from.label == "publishPayloadTwice" {
            2
        } else {
            1
        };
        assert_eq!(edge.count, expected_count);
        for evidence in &edge.evidence {
            assert_site(&graph, edge, evidence);
            assert_qualifiers(&to.label, evidence);
            actual.insert((
                from.label.clone(),
                to.label.clone(),
                evidence.semantics.clone(),
            ));
        }
    }
    assert!(
        actual
            .iter()
            .any(|(_, to, semantics)| to == "onAsyncPayload" && semantics == "async-boundary")
    );
    assert!(
        actual
            .iter()
            .any(|(_, to, semantics)| to == "onCommittedPayload" && semantics == "deferred")
    );
    assert!(actual.iter().any(|(_, to, _)| to == "onApplicationEvent"));
    assert_pairs(&actual);
}

fn assert_site(
    graph: &GraphData,
    edge: &codecallnavigator::model::Edge,
    evidence: &codecallnavigator::model::RelationshipEvidence,
) {
    assert_eq!(evidence.rule, "codecallnavigator.spring:event");
    let span = evidence.invocation.span.unwrap();
    let text = std::fs::read(&evidence.invocation.file).unwrap();
    assert!(
        std::str::from_utf8(&text[span.start..span.end])
            .unwrap()
            .contains("publishEvent(")
    );
    let from = &graph.nodes[edge.from as usize];
    let to = &graph.nodes[edge.to as usize];
    let mut function = graph
        .flow_inventory
        .as_ref()
        .unwrap()
        .functions
        .iter()
        .find(|function| function.symbol == from.id)
        .unwrap()
        .clone();
    let mut matched = false;
    flow::visit_calls_mut(&mut function, &mut |call| {
        if call.span != span {
            return;
        }
        matched = call.targets.iter().any(|target| {
            (
                target.symbol.as_str(),
                target.certainty.as_str(),
                target.provenance.as_str(),
            ) == (to.id.as_str(), "possible", edge.provenance.as_str())
        });
        assert!(
            call.diagnostic
                .as_ref()
                .is_some_and(|text| text.contains("possible"))
        );
    });
    assert!(matched, "Missing matching source flow target");
}
fn assert_qualifiers(name: &str, evidence: &codecallnavigator::model::RelationshipEvidence) {
    let qualifiers = &evidence.qualifiers;
    assert_eq!(
        qualifiers["spring.async"],
        if name == "onAsyncPayload" {
            "true"
        } else {
            "false"
        }
    );
    if name == "onConditionalPayload" {
        assert_eq!(
            qualifiers["spring.condition"],
            "#root.args[0].id == 'enabled'"
        );
    }
    if name == "onCommittedPayload" {
        assert_eq!(qualifiers["spring.transaction_phase"], "AFTER_COMMIT");
        assert_eq!(qualifiers["spring.fallback_execution"], "false");
    }
}

fn assert_supported(from: &str, to: &str) {
    assert!(!to.contains("Other") && !to.contains("Pretend"));
    assert!(!from.contains("Erased") && !from.contains("Generic"));
}

fn assert_pairs(actual: &BTreeSet<(String, String, String)>) {
    let mut expected = BTreeSet::new();
    for publisher in ["publishPayload", "publishPayloadTwice"] {
        for listener in [
            "onPayload",
            "onExplicitPayload",
            "onConditionalPayload",
            "onCommittedPayload",
            "onAsyncPayload",
            "onPayloadEnvelope",
            "onAnyApplicationEvent",
        ] {
            expected.insert((publisher.to_owned(), listener.to_owned()));
        }
    }
    for listener in ["onApplicationEvent", "onAnyApplicationEvent"] {
        expected.insert(("publishApplicationEvent".into(), listener.into()));
    }
    let actual = actual
        .iter()
        .map(|(from, to, _)| (from.clone(), to.clone()))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        actual, expected,
        "Qualified payload and API negatives must stay separate"
    );
}

#[test]
#[ignore = "requires pinned Java analyzer, Maven, JDK25 and cached Spring6.2.5"]
fn raw_generic_payloads_and_raw_enclosing_owners_are_deferred() {
    let directory = support::fixture("spring-events");
    if let Some(command) = std::env::var_os("CODECALLNAVIGATOR_TEST_MAVEN") {
        support::install_wrapper(directory.path(), &std::path::PathBuf::from(command));
    }
    let source = directory
        .path()
        .join("src/main/java/fixture/spring/RawEvents.java");
    std::fs::write(source, r#"package fixture.spring;
import org.springframework.context.ApplicationEventPublisher;
import org.springframework.context.event.EventListener;
public final class RawEvents {
    static class Box<T> {}
    static final class RawBox extends Box {}
    static final class StringBox extends Box<String> {}
    static class Owner<T> { class Member {} }
    static final class RawMember extends Owner.Member { RawMember(Owner owner) { owner.super(); } }
    static final class Outer<T> { final class Inner<U> {} }
    public void preservedSubclass(ApplicationEventPublisher publisher) { publisher.publishEvent(new StringBox()); }
    public void directGeneric(ApplicationEventPublisher publisher) { publisher.publishEvent(new Box<String>()); }
    public void inheritedRawOwner(ApplicationEventPublisher publisher, RawMember event) { publisher.publishEvent(event); }
    public void rawSubclass(ApplicationEventPublisher publisher, RawBox event) { publisher.publishEvent(event); }
    public void rawPayload(ApplicationEventPublisher publisher, Box event) { publisher.publishEvent(event); }
    public void rawEnclosingOwner(ApplicationEventPublisher publisher, Outer.Inner event) { publisher.publishEvent(event); }
    @EventListener public void typedBox(Box<String> event) { Sink.record("typed box"); }
    @EventListener public void typedInner(Outer<String>.Inner<String> event) { Sink.record("typed inner"); }
}
"#).unwrap();
    let graph = support::run(vec![support::target(directory.path(), "maven")]).unwrap();
    assert_eq!(
        inferred(&graph).len(),
        18,
        "Only concrete nongeneric subclasses retain generic payload metadata"
    );
    let targets = inferred(&graph)
        .into_iter()
        .filter(|edge| graph.nodes[edge.from as usize].label == "preservedSubclass")
        .map(|edge| graph.nodes[edge.to as usize].label.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        targets,
        BTreeSet::from(["typedBox", "onAnyApplicationEvent"])
    );
    assert!(
        graph
            .diagnostics
            .iter()
            .filter(|text| text.contains("Spring publishEvent payload is erased"))
            .count()
            >= 5
    );
}

#[test]
#[ignore = "requires pinned Java analyzer, Maven, JDK25 and cached Spring6.2.5"]
fn spring_listener_catalog_survives_without_any_publisher_invocations() {
    let directory = support::fixture("spring-events");
    if let Some(command) = std::env::var_os("CODECALLNAVIGATOR_TEST_MAVEN") {
        support::install_wrapper(directory.path(), &std::path::PathBuf::from(command));
    }
    let publisher = directory
        .path()
        .join("src/main/java/fixture/spring/Publisher.java");
    let original = std::fs::read_to_string(&publisher).unwrap();
    let changed = original
        .lines()
        .filter(|line| !line.contains("publisher.publishEvent("))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(publisher, changed).unwrap();
    let graph = support::run(vec![support::target(directory.path(), "maven")]).unwrap();
    assert!(
        inferred(&graph).is_empty(),
        "Registration alone cannot create invocation edges"
    );
    assert_listener_catalog(&graph);
}

fn assert_listener_catalog(graph: &GraphData) {
    let entries = &graph.entry_points.as_ref().unwrap().framework_entries;
    let actual = graph
        .nodes
        .iter()
        .filter(|node| entries.contains(&node.id) && !node.namespace.ends_with(".http"))
        .map(|node| node.label.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        actual,
        BTreeSet::from([
            "onPayload",
            "onExplicitPayload",
            "onConditionalPayload",
            "onCommittedPayload",
            "onAsyncPayload",
            "onPayloadEnvelope",
            "onAnyApplicationEvent",
            "onOtherCreated",
            "onApplicationEvent"
        ])
    );
    assert_eq!(
        entries.len(),
        17,
        "Nine event listeners and eight HTTP handlers stay catalogued"
    );
}
