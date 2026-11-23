//! Exercise class selection, saved previews, and call changes on actual drawn frames.
use super::{Driver, Phase, Step, change_key, focused, preview_drawn};
use crate::{
    app::State,
    flow_comparison::{ClassPair, SideId},
};
use anyhow::{Context, Result, ensure};
use codecallnavigator::artifact::comparison_changes::GraphChange;
use rune::{Runtime, runtime::ElementInfo};
use std::time::{Duration, Instant};

impl Driver {
    pub(super) fn class_flow_action(
        &mut self,
        runtime: &mut Runtime<State>,
        step: Step,
    ) -> Result<()> {
        match step {
            Step::HoverOutlineClass => {
                if let Some(key) = self.outline_class_key(runtime)?
                    && self.input.hover(runtime, &key)
                {
                    self.requested = Some(Instant::now());
                    self.phase = Phase::FlowFirst(Step::OutlineClassHovered);
                }
            }
            Step::FocusOutlineClass => {
                self.input.hover(runtime, "comparison-base");
                if self
                    .outline_class_key(runtime)?
                    .is_some_and(|key| !focused(runtime, &key))
                {
                    tab(runtime);
                }
            }
            Step::Class => {
                if let Some(key) = self.outline_class_key(runtime)? {
                    self.click(runtime, &key, Phase::FlowFirst(Step::ClassDrawn));
                }
            }
            Step::HoverClass => {
                let pair = runtime
                    .state
                    .flow_comparison
                    .class_pair()
                    .context("No class graph")?;
                if self
                    .input
                    .hover(runtime, &node_key(SideId::After, &pair.graph.root))
                {
                    self.phase = Phase::FlowFirst(Step::ClassHovered);
                }
            }
            Step::FocusClassNeighbor => {
                self.input.hover(runtime, "comparison-base");
                if !focused(runtime, &neighbor_key(runtime)?) {
                    tab(runtime);
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn class_flow_observed(
        &mut self,
        runtime: &Runtime<State>,
        step: Step,
    ) -> Result<()> {
        match step {
            Step::OutlineClassHovered
                if self
                    .requested
                    .is_some_and(|start| start.elapsed() >= Duration::from_millis(240)) =>
            {
                require_closed_preview(runtime)?;
                self.report.check("outline_hover_has_no_source_preview");
                self.phase = Phase::FlowFirst(Step::FocusOutlineClass);
            }
            Step::FocusOutlineClass
                if self
                    .outline_class_key(runtime)?
                    .is_some_and(|key| focused(runtime, &key)) =>
            {
                require_closed_preview(runtime)?;
                self.report.check("outline_focus_has_no_source_preview");
                self.phase = Phase::FlowFirst(Step::SelectMethod);
            }
            Step::ClassDrawn if class_graph_drawn(runtime)? => {
                require_closed_preview(runtime)?;
                self.report.check("class_call_changes_drawn");
                self.phase = Phase::FlowFirst(Step::HoverClass);
            }
            Step::ClassHovered if preview_drawn(runtime, "Leaf(2)") => {
                self.report.check("class_source_diff_drawn_on_hover");
                self.phase = Phase::FlowFirst(Step::FocusClassNeighbor);
            }
            Step::FocusClassNeighbor
                if focused(runtime, &neighbor_key(runtime)?) && excerpt_drawn(runtime, "=> 23") =>
            {
                self.report.check("class_saved_excerpt_drawn_on_focus");
                self.phase = Phase::FlowFirst(Step::ReturnToMethod);
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn outline_class_key(&self, runtime: &Runtime<State>) -> Result<Option<String>> {
        let id = runtime
            .state
            .flow_comparison
            .changes
            .rows
            .iter()
            .find_map(|row| {
                (row.symbol.as_deref() == Some(&self.config.cases[0].symbol))
                    .then_some(row.class_id.as_deref())
                    .flatten()
            })
            .context("The changed fixture method has no owning class row")?;
        Ok(change_key(runtime, id))
    }
}

fn require_closed_preview(runtime: &Runtime<State>) -> Result<()> {
    ensure!(
        !runtime.elements().iter().any(|element| {
            element.key.as_deref() == Some("comparison-unified-source") && visible(element)
        }),
        "The outline opened a source preview"
    );
    Ok(())
}

fn class_graph_drawn(runtime: &Runtime<State>) -> Result<bool> {
    let Some(pair) = runtime.state.flow_comparison.class_pair() else {
        return Ok(false);
    };
    let removed = named_node(pair, "OldNeighbor")?;
    let added = named_node(pair, "NewNeighbor")?;
    let elements = runtime.elements();
    let expected = [
        Redirect {
            side: SideId::Before,
            shown: removed,
            hidden: added,
            status: GraphChange::Removed,
            label: "Removed",
        },
        Redirect {
            side: SideId::After,
            shown: added,
            hidden: removed,
            status: GraphChange::Added,
            label: "Added",
        },
    ];
    Ok(expected.iter().all(|side| side.drawn(pair, &elements)))
}

struct Redirect<'a> {
    side: SideId,
    shown: &'a str,
    hidden: &'a str,
    status: GraphChange,
    label: &'a str,
}

impl Redirect<'_> {
    fn drawn(&self, pair: &ClassPair, elements: &[ElementInfo]) -> bool {
        let root = &pair.graph.root;
        let visible_nodes = [root.as_str(), self.shown]
            .iter()
            .all(|id| visible_key(elements, &node_key(self.side, id)));
        let hidden_node = !visible_key(elements, &node_key(self.side, self.hidden));
        visible_nodes && hidden_node && self.link_drawn(pair, elements)
    }

    fn link_drawn(&self, pair: &ClassPair, elements: &[ElementInfo]) -> bool {
        let root = &pair.graph.root;
        let key = super::observation::address(self.side)
            .key(&format!("class-link-{root}-{}", self.shown));
        let matches_fixture =
            pair.graph.links.iter().any(|link| {
                &link.from == root && link.to == self.shown && link.status == self.status
            });
        matches_fixture
            && elements.iter().any(|element| {
                element.key.as_deref() == Some(&key)
                    && element
                        .text
                        .as_deref()
                        .is_some_and(|text| text.contains(self.label))
                    && visible(element)
            })
    }
}

fn neighbor_key(runtime: &Runtime<State>) -> Result<String> {
    let pair = runtime
        .state
        .flow_comparison
        .class_pair()
        .context("No class graph")?;
    Ok(node_key(SideId::After, named_node(pair, "NewNeighbor")?))
}

fn named_node<'a>(pair: &'a ClassPair, name: &str) -> Result<&'a str> {
    pair.graph
        .nodes
        .iter()
        .find(|node| node.name.rsplit('.').next() == Some(name))
        .map(|node| node.id.as_str())
        .with_context(|| format!("Fixture class graph omitted {name}"))
}

fn node_key(side: SideId, id: &str) -> String {
    super::observation::address(side).key(&format!("class-{id}"))
}

fn visible_key(nodes: &[ElementInfo], key: &str) -> bool {
    nodes
        .iter()
        .any(|element| element.key.as_deref() == Some(key) && visible(element))
}

fn visible(element: &ElementInfo) -> bool {
    !element.exiting && element.visible[2].min(element.visible[3]) > 0.0
}

fn excerpt_drawn(runtime: &Runtime<State>, excerpt: &str) -> bool {
    runtime.elements().iter().any(|element| {
        visible(element)
            && element.path.contains("/comparison-unified-source")
            && element
                .text
                .as_deref()
                .is_some_and(|text| text.contains(excerpt))
    })
}

fn tab(runtime: &mut Runtime<State>) {
    runtime.key(rune::KeyEvent {
        key: rune::Key::Tab,
        mods: rune::Mods::default(),
    });
}
