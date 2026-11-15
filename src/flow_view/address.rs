//! Concrete pane addresses keep interaction, metadata and source identity endpoint-local.
use crate::{
    app::State,
    flow_comparison::{Side, SideId},
    flow_workspace::{Session, SourceKeys},
};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Address {
    Single,
    Before,
    After,
}
pub(crate) struct Target {
    pub name: String,
    pub unavailable: bool,
    pub known: bool,
}
impl Address {
    pub fn side(self) -> Option<SideId> {
        match self {
            Self::Single => None,
            Self::Before => Some(SideId::Before),
            Self::After => Some(SideId::After),
        }
    }
    pub fn key(self, tail: &str) -> String {
        match self {
            Self::Single => format!("flow-{tail}"),
            Self::Before => format!("comparison-before-{tail}"),
            Self::After => format!("comparison-after-{tail}"),
        }
    }
    pub fn session(self, state: &State) -> Option<&Session> {
        match self.side() {
            None => state.flow_workspace.current.as_ref(),
            Some(side) => match state.flow_comparison.current.as_ref()?.side(side) {
                Side::Ready(ready) => Some(&ready.session),
                _ => None,
            },
        }
    }
    pub fn session_mut(self, state: &mut State) -> Option<&mut Session> {
        match self.side() {
            None => state.flow_workspace.current.as_mut(),
            Some(side) => match state.flow_comparison.current.as_mut()?.side_mut(side) {
                Side::Ready(ready) => Some(&mut ready.session),
                _ => None,
            },
        }
    }
    pub fn keys(self, session: &Session) -> SourceKeys {
        match self {
            Self::Single => SourceKeys::Single,
            _ => SourceKeys::Comparison(session.scroll_key.clone()),
        }
    }
    pub fn viewport(self, state: &State) -> [f32; 2] {
        match self.side() {
            None => state.flow_workspace.viewport,
            Some(side) => state.flow_comparison.viewports[side.index()],
        }
    }
    pub fn layout(self, state: &mut State, viewport: [f32; 2]) {
        if let Some(side) = self.side() {
            state.flow_comparison.layout(side, viewport);
        } else {
            state.flow_workspace.viewport = viewport;
            if state.flow_workspace.fit_pending {
                state.fit_flow();
                state.flow_workspace.fit_pending = false;
            }
        }
    }
    pub fn measured(self, state: &mut State, key: &str, viewport: [f32; 2]) {
        if let Some(side) = self.side() {
            state.flow_comparison.measured(side, key, viewport);
        }
    }
    pub fn select(self, state: &mut State, index: usize, tx: &mut rune::Tx) {
        match self.side() {
            None => state.select_flow_step(index, tx),
            Some(side) => state.flow_comparison.select_step(side, index, tx),
        }
    }
    pub fn revision(self, state: &State) -> u64 {
        match self {
            Self::Single => state.flow_workspace_revision(),
            _ => state.flow_comparison.view_revision(),
        }
    }
    pub fn mounted(self, state: &mut State, key: &str, tx: &mut rune::Tx) {
        if let Some(side) = self.side() {
            state.flow_comparison.source_mounted(side, key);
            state.flow_comparison.restore_scroll(tx);
        }
    }
    pub fn target(self, state: &State, symbol: &str) -> Target {
        if let Some(side) = self.side() {
            let node =
                state
                    .flow_comparison
                    .current
                    .as_ref()
                    .and_then(|pair| match pair.side(side) {
                        Side::Ready(ready) => {
                            ready.targets.iter().find(|node| node.symbol == symbol)
                        }
                        _ => None,
                    });
            return node
                .map(|node| Target {
                    name: node.signature.clone(),
                    unavailable: node.body_unavailable,
                    known: true,
                })
                .unwrap_or(Target {
                    name: symbol.into(),
                    unavailable: true,
                    known: false,
                });
        }
        let node = state.graph.as_ref().and_then(|graph| {
            graph
                .node_index(symbol)
                .map(|index| &graph.data.nodes[index as usize])
        });
        node.map(|node| Target {
            name: node.signature.clone(),
            unavailable: node.body_unavailable,
            known: true,
        })
        .unwrap_or(Target {
            name: symbol.into(),
            unavailable: false,
            known: false,
        })
    }
    pub fn target_disabled(self, target: &Target) -> bool {
        self != Self::Single && target.unavailable
    }
    pub fn target_help(self, target: &Target) -> &'static str {
        if !self.target_disabled(target) {
            ""
        } else if target.known {
            "No body was recorded for this endpoint target"
        } else {
            "Target metadata is unavailable in this endpoint"
        }
    }
    pub fn follow(self, state: &mut State, symbol: String) {
        if let Some(side) = self.side() {
            if let Err(error) = state.flow_comparison.follow(side, &symbol) {
                state.flow_comparison.controls.message = error;
            }
        } else {
            state.open_flow(symbol);
        }
    }
}
