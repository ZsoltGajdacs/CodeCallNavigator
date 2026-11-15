//! A rendered control owns exact choices and presentation identity, never mutable UI indices.
use super::{Session, model::Published};
use crate::{app::State, flow_workspace::Ready};
use codecallnavigator::{flow::OccurrencePath, query::FlowExpansionSelection};
use std::sync::Arc;
struct Presentation {
    ready: Arc<Ready>,
    published: Arc<Published>,
    step: usize,
    epoch: u64,
    serial: u64,
    generation: Option<String>,
}
pub(crate) struct PresentedAction {
    presentation: Presentation,
    edit: Edit,
}
enum Edit {
    Select(FlowExpansionSelection),
    Remove(OccurrencePath),
}
impl Presentation {
    fn capture(state: &State) -> Option<Self> {
        let session = state.flow_workspace.current.as_ref()?;
        let ready = session.ready.as_ref()?.clone();
        let step = session.selected?;
        ready.chart.steps.get(step)?;
        let published = session.expansion.as_ref()?.published.as_ref()?.clone();
        Some(Self {
            ready,
            published,
            step,
            epoch: state.epoch,
            serial: state.flow_workspace.serial,
            generation: state
                .query_context
                .as_ref()
                .and_then(|context| context.descriptor.generation.clone()),
        })
    }
    fn current<'a>(&self, state: &'a State) -> Option<&'a Session> {
        if (self.epoch, self.serial) != (state.epoch, state.flow_workspace.serial) {
            return None;
        }
        let generation = state
            .query_context
            .as_ref()
            .and_then(|context| context.descriptor.generation.as_deref());
        if generation != self.generation.as_deref() {
            return None;
        }
        let session = state.flow_workspace.current.as_ref()?;
        let ready = session.ready.as_ref()?;
        let published = session.expansion.as_ref()?.published.as_ref()?;
        (Arc::ptr_eq(ready, &self.ready)
            && Arc::ptr_eq(published, &self.published)
            && session.selected == Some(self.step)
            && session.symbol == self.published.result.root_symbol)
            .then_some(session)
    }
}
impl PresentedAction {
    pub(crate) fn is_collapse(&self) -> bool {
        matches!(self.edit, Edit::Remove(_))
    }
    pub(super) fn edits(&self, state: &State) -> Option<Vec<FlowExpansionSelection>> {
        if state.flow_workspace.expansion_pending() {
            return None;
        }
        let session = self.presentation.current(state)?;
        if session.expansion.as_ref()?.pending.is_some() {
            return None;
        }
        Some(match &self.edit {
            Edit::Select(selection) => session.replacing_expansion_choice(selection),
            Edit::Remove(path) => session.without_expansion_path(path),
        })
    }
}
impl State {
    pub(crate) fn present_flow_call(&self, call: usize, target: usize) -> Option<PresentedAction> {
        let presentation = Presentation::capture(self)?;
        let session = self.flow_workspace.current.as_ref()?;
        let (caller, choice) = session.expansion_choice(call, target)?;
        let edit = if session.flow_choice_selected(call, target) {
            let mut path = caller;
            path.0.push(choice);
            Edit::Remove(path)
        } else {
            Edit::Select(FlowExpansionSelection { caller, choice })
        };
        Some(PresentedAction { presentation, edit })
    }
    pub(crate) fn present_flow_occurrence(&self) -> Option<PresentedAction> {
        let presentation = Presentation::capture(self)?;
        let path = self
            .flow_workspace
            .current
            .as_ref()?
            .selected_occurrence()?
            .clone();
        if path.0.is_empty() {
            return None;
        }
        Some(PresentedAction {
            presentation,
            edit: Edit::Remove(path),
        })
    }
    pub(crate) fn apply_presented_expansion(&mut self, action: &PresentedAction) {
        if let Some(selections) = action.edits(self) {
            self.request_flow_expansion(selections);
        }
    }
}
