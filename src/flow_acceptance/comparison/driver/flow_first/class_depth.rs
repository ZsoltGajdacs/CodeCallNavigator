//! Prove that the native depth selector updates both saved class diagrams.
use super::{
    Driver, Phase, Runtime, State, Step,
    class_flow::{class_graph_drawn, excerpt_drawn, named_node, node_key, visible_key},
};
use crate::flow_comparison::{SideId, Workspace};
use anyhow::{Context, Result, ensure};
use rune::{Key, KeyEvent, Mods};

fn depth_keys(depth: u32) -> (&'static str, &'static str) {
    if depth > 1 {
        (
            "comparison-after-depth-trigger",
            "comparison-after-depth-content",
        )
    } else {
        ("comparison-depth-trigger", "comparison-depth-content")
    }
}
const SECOND_HOP: &str = "DepthRates";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::flow_acceptance::comparison::driver) enum DepthStep {
    Open(u32),
    Choose(u32),
    Drawn(u32),
    HoverSecondHop,
    SecondHopSource,
}

impl DepthStep {
    fn phase(self) -> Phase {
        Phase::FlowFirst(Step::Depth(self))
    }
}

impl Driver {
    pub(super) fn depth_action(
        &mut self,
        runtime: &mut Runtime<State>,
        step: DepthStep,
    ) -> Result<()> {
        match step {
            DepthStep::Open(depth) => self.open_depth_menu(runtime, depth),
            DepthStep::Choose(depth) => self.choose_depth(runtime, depth),
            DepthStep::HoverSecondHop => {
                let pair = runtime
                    .state
                    .flow_comparison
                    .class_pair()
                    .context("No class diagram")?;
                let node = named_node(pair, SECOND_HOP)?;
                if self.input.hover(runtime, &node_key(SideId::After, node)) {
                    self.phase = DepthStep::SecondHopSource.phase();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn open_depth_menu(&mut self, runtime: &mut Runtime<State>, depth: u32) {
        self.input.hover(runtime, "comparison-base");
        if runtime.state.flow_comparison.previews.active.is_some() {
            runtime.key(KeyEvent {
                key: Key::Escape,
                mods: Mods::default(),
            });
        } else {
            self.click(
                runtime,
                depth_keys(depth).0,
                DepthStep::Choose(depth).phase(),
            );
        }
    }

    fn choose_depth(&mut self, runtime: &mut Runtime<State>, depth: u32) {
        let options = runtime.elements();
        let menu = depth_keys(depth).1;
        if !visible_key(&options, menu) {
            return;
        }
        let label = depth.to_string();
        let key = format!("option{}", depth - 1);
        if options.iter().any(|option| {
            option.path.contains(menu)
                && option.key.as_deref() == Some(&key)
                && option.label.as_deref() == Some(&label)
        }) {
            self.click(runtime, &key, DepthStep::Drawn(depth).phase());
        }
    }

    pub(super) fn depth_observed(
        &mut self,
        runtime: &Runtime<State>,
        step: DepthStep,
    ) -> Result<()> {
        match step {
            DepthStep::Drawn(2) if rendered_depth(runtime, 2) && second_hop_drawn(runtime)? => {
                self.report.check("class_depth_expanded_in_both_panes");
                self.phase = DepthStep::HoverSecondHop.phase();
            }
            DepthStep::SecondHopSource if excerpt_drawn(runtime, "=> 61;") => {
                self.report.check("class_second_hop_saved_source_drawn");
                self.phase = DepthStep::Open(1).phase();
            }
            DepthStep::Drawn(1) if rendered_depth(runtime, 1) && class_graph_drawn(runtime)? => {
                require_one_hop(&runtime.state.flow_comparison)?;
                self.report.check("class_depth_reduced_in_both_panes");
                self.phase = Phase::FlowFirst(Step::ReturnToMethod);
            }
            _ => {}
        }
        Ok(())
    }
}

fn rendered_depth(runtime: &Runtime<State>, depth: u32) -> bool {
    let workspace = &runtime.state.flow_comparison;
    workspace.class_depth() == depth
        && workspace
            .class_pair()
            .is_some_and(|pair| pair.depth == depth)
}

fn second_hop_drawn(runtime: &Runtime<State>) -> Result<bool> {
    let pair = runtime
        .state
        .flow_comparison
        .class_pair()
        .context("No class diagram")?;
    let node = named_node(pair, SECOND_HOP)?;
    let elements = runtime.elements();
    Ok([SideId::Before, SideId::After].into_iter().all(|side| {
        visible_key(&elements, &node_key(side, &pair.graph.root))
            && visible_key(&elements, &node_key(side, node))
    }))
}

fn require_one_hop(workspace: &Workspace) -> Result<()> {
    let pair = workspace.class_pair().context("No class diagram")?;
    ensure!(
        named_node(pair, SECOND_HOP).is_err(),
        "Depth one still includes the second-hop class"
    );
    Ok(())
}
