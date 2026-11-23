//! Catalog actions use exact saved IDs and endpoint-owned availability.
use super::{
    click,
    flow_comparison_fixture::{choose, runtime},
    flow_comparisons::{choose_revision, edit, load, settle},
};
use crate::{
    engine_client::flow_comparison::CorrespondenceKind,
    flow_comparison::{Side, SnapshotSide},
};
fn text(runtime: &rune::Runtime<crate::app::State>, needle: &str) -> bool {
    runtime
        .elements()
        .iter()
        .any(|el| el.text.as_deref().is_some_and(|text| text.contains(needle)))
}
#[test]
fn explicit_counterpart_reset_and_search_keep_exact_identity_actions() {
    let data = crate::flow_scenes::data();
    let (mut ui, _project) = runtime(data.clone(), data, true);
    load(&mut ui);
    let symbol = choose(&mut ui, "load(");
    click(&mut ui, "comparison-choose-after");
    edit(&mut ui, "comparison-search", "save");
    click(&mut ui, "comparison-search-submit");
    settle(&mut ui);
    let target = choose(&mut ui, "save(");
    let pair = ui.state.flow_comparison.current.as_ref().unwrap();
    assert_eq!(pair.selection.symbol, symbol);
    assert_eq!(pair.selection.head_symbol.as_deref(), Some(target.as_str()));
    assert_eq!(pair.correspondence.kind, CorrespondenceKind::Manual);
    assert!(text(&ui, "Manual counterpart"));
    click(&mut ui, "comparison-source-after");
    assert!(text(&ui, "Manual counterpart"));
    click(&mut ui, "comparison-reset-counterpart");
    settle(&mut ui);
    let pair = ui.state.flow_comparison.current.as_ref().unwrap();
    assert!(pair.selection.head_symbol.is_none());
    assert_eq!(pair.correspondence.kind, CorrespondenceKind::Exact);
}
#[test]
fn catalog_paging_and_search_reset_use_returned_cursors() {
    let mut data = crate::flow_scenes::data();
    let template = data.nodes[0].clone();
    for i in 0..70 {
        let mut node = template.clone();
        node.id = format!("extra/{i:03}");
        node.label = format!("Extra {i:03}");
        node.signature = node.label.clone();
        data.nodes.push(node);
    }
    let (mut ui, _project) = runtime(data.clone(), data, false);
    load(&mut ui);
    assert_eq!(
        ui.state.flow_comparison.catalog.as_ref().unwrap().returned,
        50
    );
    click(&mut ui, "comparison-page-next");
    settle(&mut ui);
    assert_eq!(
        ui.state.flow_comparison.catalog.as_ref().unwrap().offset,
        50
    );
    click(&mut ui, "comparison-page-previous");
    settle(&mut ui);
    assert_eq!(ui.state.flow_comparison.catalog.as_ref().unwrap().offset, 0);
    edit(&mut ui, "comparison-search", "Extra 069");
    click(&mut ui, "comparison-search-submit");
    settle(&mut ui);
    let page = ui.state.flow_comparison.catalog.as_ref().unwrap();
    assert_eq!(page.total, 1);
    let symbol = page.records[0].symbol.clone();
    assert!(!ui.state.flow_comparison.has_previous_page());
    let row = ui
        .elements()
        .into_iter()
        .find(|el| el.key.as_deref() == Some(format!("comparison-record-{symbol}").as_str()))
        .unwrap();
    let help = row.help.unwrap();
    assert!(help.contains("Snapshot"));
    assert!(help.contains("flow.ts"));
    assert!(help.contains("Extra 069"));
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
    click(&mut ui, "comparison-catalog-open");
    choose(&mut ui, "removed(");
    assert!(text(&ui, "Missing in this revision"));
    click(&mut ui, "comparison-catalog-open");
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
    choose(&mut ui, "load(");
    assert_eq!(ui.state.flow_comparison.header(), Some(&pinned));
    assert!(ui.state.flow_comparison.current.is_some());

    let later = codecallnavigator::artifact::revision::Repository::open(project.path())
        .unwrap()
        .resolve("HEAD")
        .unwrap();
    choose_revision(&mut ui, SnapshotSide::Head, &later);
    click(&mut ui, "comparison-load");
    settle(&mut ui);
    let reloaded = ui.state.flow_comparison.header().unwrap();
    assert_eq!(reloaded.head.commit.as_deref(), Some(later.as_str()));
    assert_ne!(reloaded.comparison_id, pinned.comparison_id);
}
