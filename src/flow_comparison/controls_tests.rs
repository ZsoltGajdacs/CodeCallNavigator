//! Paging records actual prior cursors and forgets them at a new search or binding.
use crate::flow_comparison::tests::{git, project, settle, state};
fn many(root: &std::path::Path) {
    project(root);
    let mut graph = crate::flow_scenes::data();
    let template = graph.nodes[0].clone();
    for i in 0..150 {
        let mut node = template.clone();
        node.id = format!("extra/{i}");
        node.label = format!("extra{i}()");
        node.signature = node.label.clone();
        node.body_unavailable = true;
        graph.nodes.push(node);
    }
    let saved =
        codecallnavigator::artifact::canonical(&graph, &[("repo".into(), "/snapshot".into())])
            .unwrap();
    std::fs::write(
        root.join(codecallnavigator::artifact::GRAPH_PATH),
        saved.bytes().unwrap(),
    )
    .unwrap();
    git(root, &["add", codecallnavigator::artifact::GRAPH_PATH]);
    git(root, &["commit", "-qm", "many identities"]);
    git(root, &["tag", "-f", "head"]);
}
#[test]
fn paging_keeps_prior_offsets_and_resets_for_search_reload_and_new_comparison() {
    let root = tempfile::tempdir().unwrap();
    many(root.path());
    let mut state = state(root.path());
    state
        .open_flow_comparison("refs/tags/base".into(), "refs/tags/head".into())
        .unwrap();
    settle(&mut state);
    for expected in [50, 100] {
        state.flow_comparison.next_control();
        settle(&mut state);
        assert_eq!(
            state.flow_comparison.catalog.as_ref().unwrap().offset,
            expected
        );
    }
    assert_eq!(state.flow_comparison.controls.previous, vec![0, 50]);
    for expected in [50, 0] {
        state.flow_comparison.previous_control();
        settle(&mut state);
        assert_eq!(
            state.flow_comparison.catalog.as_ref().unwrap().offset,
            expected
        );
    }
    assert!(!state.flow_comparison.has_previous_page());
    state.flow_comparison.next_control();
    settle(&mut state);
    state.flow_comparison.search_control();
    settle(&mut state);
    assert!(!state.flow_comparison.has_previous_page());
    state.flow_comparison.next_control();
    settle(&mut state);
    state.flow_comparison.reload_control();
    settle(&mut state);
    assert!(
        !state.flow_comparison.has_previous_page(),
        "Reload retained offsets from the previous comparison"
    );
    state.flow_comparison.next_control();
    settle(&mut state);
    state
        .open_flow_comparison("refs/tags/base".into(), "refs/tags/head".into())
        .unwrap();
    settle(&mut state);
    assert!(!state.flow_comparison.has_previous_page());
}
