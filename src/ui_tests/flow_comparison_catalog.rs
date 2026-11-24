//! Saved comparison pairs keep exact revision identity and endpoint availability.
use super::{
    click,
    flow_comparison_fixture::{choose, runtime},
    flow_comparisons::{choose_revision, load, settle},
};
use crate::flow_comparison::{Side, SnapshotSide};
fn text(runtime: &rune::Runtime<crate::app::State>, needle: &str) -> bool {
    runtime
        .elements()
        .iter()
        .any(|el| el.text.as_deref().is_some_and(|text| text.contains(needle)))
}
#[test]
fn missing_and_unavailable_sides_stay_explicit() {
    let mut before = crate::flow_scenes::data();
    let mut after = before.clone();
    let mut removed = before.nodes[1].clone();
    removed.id = "old/save".into();
    removed.signature = "removed()".into();
    removed.label = removed.signature.clone();
    before.nodes.push(removed);
    let mut added = after.nodes[1].clone();
    added.id = "new/save".into();
    added.signature = "added()".into();
    added.label = added.signature.clone();
    after.nodes.push(added);
    after.flow_inventory = None;
    let (mut ui, _project) = runtime(before, after, true);
    load(&mut ui);
    choose(&mut ui, "load(");
    assert!(matches!(
        ui.state.flow_comparison.current.as_ref().unwrap().before,
        Side::Ready(_)
    ));
    let fit = ui
        .elements()
        .into_iter()
        .find(|el| el.key.as_deref() == Some("comparison-fit-after"))
        .unwrap();
    assert!(!fit.focusable, "Unavailable endpoint must disable Fit");
    assert!(text(&ui, "flow unavailable"));
    assert!(text(&ui, "Refresh") || text(&ui, "refresh"));
    choose(&mut ui, "removed(");
    assert!(text(&ui, "Missing in this revision"));
    choose(&mut ui, "added(");
    assert!(matches!(
        ui.state.flow_comparison.current.as_ref().unwrap().before,
        Side::Missing
    ));
    assert!(text(&ui, "Missing in this revision"));
}
#[test]
fn moved_tag_preserves_the_selected_commit_until_another_is_loaded() {
    let data = crate::flow_scenes::data();
    let (mut ui, project) = runtime(data.clone(), data.clone(), true);
    super::flow_comparison_fixture::commit(project.path(), &data, "later");
    load(&mut ui);
    let pinned = ui.state.flow_comparison.header().unwrap().clone();
    crate::flow_comparison::tests::git(project.path(), &["tag", "-f", "head"]);
    click(&mut ui, "comparison-snapshot-after-find");
    settle(&mut ui);
    let revision = pinned.head.commit.as_deref().unwrap();
    let lookup = ui
        .state
        .flow_comparison
        .snapshots
        .lookup(SnapshotSide::Head, revision)
        .unwrap();
    assert_eq!(lookup.status.commit, revision);
    assert_eq!(lookup.status.snapshots.len(), 1);
    assert!(ui.find("comparison-snapshot-variants").is_some());
    click(&mut ui, "comparison-snapshot-variants-done");
    assert!(ui.find("comparison-snapshot-variants").is_none());
    choose(&mut ui, "load(");
    assert_eq!(ui.state.flow_comparison.header(), Some(&pinned));
    assert!(ui.state.flow_comparison.current.is_some());

    let later = codecallnavigator::artifact::revision::Repository::open(project.path())
        .unwrap()
        .resolve("HEAD")
        .unwrap();
    choose_revision(&mut ui, SnapshotSide::Head, &later);
    let reloaded = ui.state.flow_comparison.header().unwrap();
    assert_eq!(reloaded.head.commit.as_deref(), Some(later.as_str()));
    assert_ne!(reloaded.comparison_id, pinned.comparison_id);
}
