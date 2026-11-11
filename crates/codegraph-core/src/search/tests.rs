use super::*;

fn lookup(
    index: &NameSearchIndex,
    text: &str,
    category: SearchCategory,
    offset: usize,
) -> SearchPage {
    index.search(&NameSearchQuery {
        text,
        category,
        offset,
        limit: 30,
    })
}

fn method(id: &str, name: &str, class: &str) -> Node {
    Node {
        id: id.into(),
        build: "build-a".into(),
        project: "project".into(),
        language: "java".into(),
        kind: "method".into(),
        label: name.into(),
        signature: format!("{class}:{name}(HiddenParameter)"),
        container: class.into(),
        file: "hidden/NotAName.java".into(),
        ..Default::default()
    }
}

fn index(nodes: Vec<Node>) -> NameSearchIndex {
    NameSearchIndex::new(&GraphData {
        nodes,
        ..Default::default()
    })
}

#[test]
fn literal_substrings_ignore_case_and_do_not_search_parameters_or_paths() {
    let search = index(vec![method("1", "load$Value", "app.ÜberService")]);
    assert_eq!(
        lookup(&search, "  VALUE  ", SearchCategory::Methods, 0)
            .hits
            .len(),
        1
    );
    assert_eq!(
        lookup(&search, "$", SearchCategory::Methods, 0).hits.len(),
        1
    );
    assert_eq!(
        lookup(&search, "ÜBER", SearchCategory::Classes, 0).hits[0].name,
        "ÜberService"
    );
    for query in ["HiddenParameter", "NotAName", "%", "_", "", "  "] {
        assert!(
            lookup(&search, query, SearchCategory::All, 0)
                .hits
                .is_empty(),
            "{query}"
        );
    }
}

#[test]
fn category_tabs_return_deduplicated_classes_and_distinct_overloads() {
    let mut overload = method("2", "load", "app.Service");
    overload.signature = "app.Service:load(int)".into();
    let search = index(vec![method("1", "load", "app.Service"), overload]);
    assert_eq!(
        lookup(&search, "Service", SearchCategory::All, 0)
            .hits
            .len(),
        3
    );
    let classes = lookup(&search, "Service", SearchCategory::Classes, 0);
    assert_eq!(classes.hits.len(), 1);
    let SearchTarget::Class {
        members, identity, ..
    } = &classes.hits[0].target
    else {
        panic!("class result")
    };
    assert_eq!(&**members, &[0, 1]);
    assert_eq!(identity.container, "app.Service");
    let methods = lookup(&search, "load", SearchCategory::Methods, 0);
    assert_eq!(methods.hits.len(), 2);
    assert_ne!(methods.hits[0].target, methods.hits[1].target);
}

#[test]
fn class_identity_keeps_same_names_in_different_builds_and_projects() {
    let original = method("1", "load", "app.Service");
    let mut build = method("2", "load", "app.Service");
    build.build = "build-b".into();
    let mut project = method("3", "load", "app.Service");
    project.project = "another".into();
    let search = index(vec![original, build, project]);
    assert_eq!(
        lookup(&search, "Service", SearchCategory::Classes, 0)
            .hits
            .len(),
        3
    );
}

#[test]
fn ranking_prioritizes_exact_then_prefix_then_interior_and_orders_ties_by_identity() {
    let search = index(vec![
        method("4", "reload", "app.Worker"),
        method("3", "loader", "app.Worker"),
        method("2", "load", "app.Worker"),
        method("1", "load", "app.Worker"),
    ]);
    let page = lookup(&search, "load", SearchCategory::Methods, 0);
    let ids: Vec<_> = page
        .hits
        .iter()
        .map(|hit| match &hit.target {
            SearchTarget::Method { identity, .. } => identity.as_str(),
            _ => panic!("method result"),
        })
        .collect();
    assert_eq!(ids, ["1", "2", "3", "4"]);
}

#[test]
fn pagination_reaches_every_result_without_duplicates() {
    let search = index(
        (0..65)
            .map(|i| method(&format!("{i:03}"), "load", "app.Worker"))
            .collect(),
    );
    let first = lookup(&search, "load", SearchCategory::Methods, 0);
    let second = lookup(&search, "load", SearchCategory::Methods, 30);
    let third = lookup(&search, "load", SearchCategory::Methods, 60);
    assert!(first.more && second.more);
    assert!(!third.more);
    assert_eq!(third.hits.len(), 5);
    assert_ne!(first.hits[0].target, second.hits[0].target);
    assert!(
        lookup(&search, "load", SearchCategory::Methods, usize::MAX)
            .hits
            .is_empty()
    );
    assert!(
        search
            .search(&NameSearchQuery {
                text: "load",
                category: SearchCategory::Methods,
                offset: 0,
                limit: 0
            })
            .hits
            .is_empty()
    );
}

#[test]
fn friendly_lambdas_keep_their_compiler_alias() {
    let mut lambda = method("1", "validate · lambda at L12", "app.Worker");
    lambda.signature = "app.Worker:lambda$static$11(java.lang.String) -> java.lang.String".into();
    let search = index(vec![lambda]);
    for query in ["validate", "lambda$static$11", "Worker:lambda$static$11"] {
        assert_eq!(
            lookup(&search, query, SearchCategory::Methods, 0)
                .hits
                .len(),
            1
        );
    }
    assert!(
        lookup(&search, "java.lang.String", SearchCategory::Methods, 0)
            .hits
            .is_empty()
    );
}

#[test]
fn typescript_file_groups_are_identified_as_files() {
    let mut callable = method("1", "run", "src/main.test.ts");
    callable.language = "typescript".into();
    let search = index(vec![callable]);
    let page = lookup(&search, "main.test", SearchCategory::Classes, 0);
    assert_eq!(page.hits[0].name, "src/main.test.ts");
    assert_eq!(page.hits[0].kind_label(), "File");
}

#[test]
fn short_class_names_preserve_nested_and_generic_names() {
    for (full, short) in [
        ("app.Outer$Inner", "Inner"),
        ("app.Outer+Inner", "Inner"),
        ("app.Worker<System.String>", "Worker<System.String>"),
    ] {
        assert_eq!(simple_class_name(full), short);
    }
}
