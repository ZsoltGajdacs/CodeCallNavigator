//! Actual Spring Web mapping annotations, source context and conservative blockers.
#[path = "support/java.rs"]
mod support;
use std::collections::BTreeSet;

#[test]
#[ignore = "requires pinned Java analyzer, Maven, JDK25 and cached Spring Web6.2.5"]
fn spring_http_routes_keep_compiler_qualified_handlers_and_constraint_blockers() {
    let directory = support::fixture("spring-events");
    if let Some(command) = std::env::var_os("CODEGRAPH_TEST_MAVEN") {
        support::install_wrapper(directory.path(), &std::path::PathBuf::from(command));
    }
    let graph = support::run(vec![support::target(directory.path(), "maven")]).unwrap();
    let facts = graph
        .boundaries
        .as_ref()
        .expect("Spring HTTP inventory is required");
    assert_eq!(facts.routes.len(), 8);
    let supported = facts
        .routes
        .iter()
        .filter(|route| route.restrictions.is_empty())
        .map(|route| (route.path.as_deref().unwrap(), route.methods.join(",")))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        supported,
        BTreeSet::from([
            ("/api/items", "GET,HEAD".into()),
            ("/api/items/{id}", "GET,HEAD".into()),
            ("/api/items", "POST".into()),
            ("/api/health", "GET,HEAD".into()),
            ("/api/constrained", "GET,HEAD".into())
        ])
    );
    let blockers = facts
        .routes
        .iter()
        .filter(|route| !route.restrictions.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(blockers.len(), 3);
    assert_eq!(
        blockers
            .iter()
            .filter(|route| route.path.as_deref() == Some("/api/ambiguous"))
            .count(),
        2
    );
    assert_eq!(
        blockers
            .iter()
            .filter(|route| route.path.as_deref() == Some("/api/constrained"))
            .count(),
        1
    );
    for route in &facts.routes {
        assert_eq!(route.framework, "spring");
        assert_eq!(route.application, "codegraph.fixture:spring-events");
        let symbol = route.symbol.as_ref().unwrap();
        assert!(graph.nodes.iter().any(|node| &node.id == symbol));
        assert!(
            graph
                .entry_points
                .as_ref()
                .unwrap()
                .framework_entries
                .contains(symbol)
        );
        assert!(route.site.span.is_some());
        assert!(!symbol.contains(":pretend("));
    }
    graph.validate().unwrap();
}

#[test]
#[ignore = "requires pinned Java analyzer, Maven, JDK25 and cached Spring Web6.2.5"]
fn spring_http_unknown_composed_inherited_and_pattern_routes_remain_blockers() {
    let directory = support::fixture("spring-events");
    if let Some(command) = std::env::var_os("CODEGRAPH_TEST_MAVEN") {
        support::install_wrapper(directory.path(), &std::path::PathBuf::from(command));
    }
    std::fs::write(
        directory
            .path()
            .join("src/main/java/fixture/spring/http/UnsupportedRoutes.java"),
        r#"package fixture.spring.http;
import java.lang.annotation.*;
import org.springframework.web.bind.annotation.*;
class Parent { @GetMapping("/inherited") public void inherited() {} }
@RestController class Child extends Parent {}
@Target(ElementType.METHOD) @Retention(RetentionPolicy.RUNTIME)
@RequestMapping(path="/composed", method=RequestMethod.GET) @interface CustomRoute {}
@RestController class Other {
    @CustomRoute public void composed() {}
    @GetMapping("/wild/**") public void wildcard() {}
    @GetMapping("${api.route}") public void property() {}
}
@RestController @RequestMapping(path="/class", headers="X-Mode=special")
class ClassConstraint { @GetMapping("/endpoint") public void endpoint() {} }
"#,
    )
    .unwrap();
    let graph = support::run(vec![support::target(directory.path(), "maven")]).unwrap();
    let routes = &graph.boundaries.as_ref().unwrap().routes;
    assert_eq!(
        routes.len(),
        13,
        "Every unsupported attributed route remains represented"
    );
    let reasons = routes
        .iter()
        .flat_map(|route| route.restrictions.iter())
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    for reason in [
        "spring.inherited_method_mapping",
        "spring.composed_mapping",
        "spring.path_pattern",
        "spring.unresolved_path",
        "spring.headers:X-Mode=special",
    ] {
        assert!(reasons.contains(reason), "Missing {reason}");
    }
    assert_eq!(
        routes.iter().filter(|route| route.path.is_none()).count(),
        3
    );
    assert!(
        routes.iter().any(
            |route| route.path.as_deref() == Some("/wild/**") && !route.restrictions.is_empty()
        )
    );
}
