//! Minimum-size controls remain inside the window and clear of native chrome.
use super::{
    click,
    flow_comparison_fixture::{choose, runtime},
    flow_comparisons::load,
};
fn contained(ui: &rune::Runtime<crate::app::State>, key: &str) {
    let el = ui
        .elements()
        .into_iter()
        .find(|el| el.key.as_deref() == Some(key) && !el.exiting)
        .unwrap_or_else(|| panic!("Missing {key}"));
    assert!(el.rect[2] > 0.0 && el.rect[3] > 0.0, "Empty {key}");
    assert!(
        el.rect[0] >= -0.5
            && el.rect[1] >= -0.5
            && el.rect[0] + el.rect[2] <= 1050.5
            && el.rect[1] + el.rect[3] <= 680.5,
        "Outside {key}: {:?}",
        el.rect
    );
    assert!(
        (el.rect[2] - el.visible[2]).abs() < 0.5 && (el.rect[3] - el.visible[3]).abs() < 0.5,
        "Clipped {key}: {:?} {:?}",
        el.rect,
        el.visible
    );
}
#[test]
fn minimum_light_and_dark_layouts_keep_full_controls_and_local_panes() {
    for dark in [false, true] {
        let data = crate::flow_scenes::data();
        let (mut ui, _project) = runtime(data.clone(), data, dark);
        let inset = rune::tooling::Chrome::DEFAULT.titlebar_inset();
        ui.set_titlebar_inset(inset);
        load(&mut ui);
        for key in [
            "comparison-catalog",
            "comparison-search",
            "comparison-search-submit",
            "comparison-catalog-close",
            "comparison-page-next",
            "comparison-page-previous",
        ] {
            contained(&ui, key);
        }
        choose(&mut ui, "load(");
        for key in [
            "comparison-close",
            "comparison-base",
            "comparison-head",
            "comparison-load",
            "comparison-reload",
            "comparison-cancel",
            "comparison-back",
            "comparison-fit-before",
            "comparison-fit-after",
            "comparison-catalog-open",
            "comparison-choose-after",
            "comparison-reset-counterpart",
            "comparison-before-canvas",
            "comparison-after-canvas",
            "comparison-source-before",
            "comparison-source-after",
            "comparison-source-area",
        ] {
            contained(&ui, key);
        }
        let close = ui
            .elements()
            .into_iter()
            .find(|el| el.key.as_deref() == Some("comparison-close"))
            .unwrap();
        assert!(close.rect[0] >= inset);
        click(&mut ui, "comparison-source-after");
        contained(&ui, "comparison-after-details");
    }
}

pub(super) const QUALIFIED_TARGET: &str = "[Application.Core]::Very.Long.命名空间.🧭.TransactionProcessingCoordinator::PersistAsync(System.Collections.Generic.IReadOnlyDictionary<System.String,System.Collections.Generic.List<System.Int32>>)";

pub(super) fn qualified_fixture() -> codecallnavigator::model::GraphData {
    let mut graph = crate::flow_scenes::data();
    graph
        .nodes
        .iter_mut()
        .find(|node| node.id == "flow/save")
        .unwrap()
        .signature = QUALIFIED_TARGET.into();
    graph
}

pub(super) fn target_label_contained(
    ui: &rune::Runtime<crate::app::State>,
    key: &str,
    inspector: &str,
) {
    let elements = ui.elements();
    let button = elements
        .iter()
        .find(|el| el.key.as_deref() == Some(key) && !el.exiting)
        .unwrap();
    let label = elements
        .iter()
        .find(|el| el.path == format!("{}/label", button.path) && !el.exiting)
        .unwrap();
    let pane = elements
        .iter()
        .find(|el| el.key.as_deref() == Some(inspector) && !el.exiting)
        .unwrap();
    assert_eq!(
        button.label.as_deref(),
        Some(format!("{QUALIFIED_TARGET} · direct").as_str())
    );
    assert_eq!(label.text, button.label);
    assert!(button.focusable);
    inside(label.rect, button.rect, "target label in button");
    inside(button.rect, pane.rect, "target button in inspector");
    assert!(
        (label.rect[2] - label.visible[2]).abs() < 0.5,
        "Clipped label: rect={:?} visible={:?}",
        label.rect,
        label.visible
    );
    assert!(label.visible[2] > 0.0 && label.visible[3] > 0.0);
}
fn inside(child: [f32; 4], parent: [f32; 4], description: &str) {
    assert!(
        child[0] >= parent[0] - 0.5
            && child[1] >= parent[1] - 0.5
            && child[0] + child[2] <= parent[0] + parent[2] + 0.5
            && child[1] + child[3] <= parent[1] + parent[3] + 0.5,
        "{description}: child={child:?} parent={parent:?}"
    );
}

#[test]
fn qualified_target_label_stays_visible_inside_minimum_comparison_inspector() {
    for dark in [false, true] {
        let data = qualified_fixture();
        let (mut ui, _project) = runtime(data.clone(), data, dark);
        ui.set_titlebar_inset(rune::tooling::Chrome::DEFAULT.titlebar_inset());
        load(&mut ui);
        choose(&mut ui, "load(");
        click(&mut ui, "comparison-source-after");
        let crate::flow_comparison::Side::Ready(after) =
            &ui.state.flow_comparison.current.as_ref().unwrap().after
        else {
            panic!("After chart missing")
        };
        let index = after
            .session
            .ready
            .as_ref()
            .unwrap()
            .chart
            .steps
            .iter()
            .position(|step| !step.calls.is_empty())
            .unwrap();
        click(&mut ui, &format!("comparison-after-step-{index}"));
        for _ in 0..4 {
            ui.frame(1.0 / 60.0);
        }
        target_label_contained(
            &ui,
            "comparison-after-target-0-0",
            "comparison-after-details",
        );
        click(&mut ui, "comparison-after-target-0-0");
        super::flow_comparisons::settle(&mut ui);
        assert_eq!(ui.state.flow_comparison.history.len(), 1);
    }
}
