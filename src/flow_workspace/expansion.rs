//! Explicit single-flow expansion requests, publication and occurrence-owned navigation.
mod actions;
mod badges;
mod model;
mod publication;
pub(super) use publication::poll;
mod worker;
use crate::{
    app::State,
    flow_workspace::{Session, SourceKeys},
};
use codecallnavigator::{
    flow,
    query::{FlowExpansionRequest, FlowExpansionSelection},
};
pub(super) use model::{Mode, Work};
use model::{Mount, Output};
use std::{
    cell::Cell,
    rc::Rc,
    sync::{Arc, atomic::AtomicBool},
};
impl State {
    pub(crate) fn flow_expansion_supported(&self) -> bool {
        self.query_context
            .as_ref()
            .is_some_and(|context| context.flow_expansion_supported())
    }
    pub(crate) fn start_flow_expansion(&mut self) {
        if !self.flow_expansion_supported() || self.flow_workspace.expansion_pending() {
            return;
        }
        let Some(session) = self
            .flow_workspace
            .current
            .as_mut()
            .filter(|session| session.ready.is_some() && !session.expansion_mode())
        else {
            return;
        };
        let original = Box::new(session.clone());
        session.scroll = Rc::new(Cell::new(session.scroll.get()));
        session.selected = None;
        session.expansion = Some(Box::new(Mode {
            original,
            published: None,
            selections: vec![],
            pending: None,
            scrolls: Default::default(),
            mounted: None,
            active: None,
        }));
        self.request_flow_expansion(vec![]);
    }
    pub(crate) fn request_flow_expansion(&mut self, selections: Vec<FlowExpansionSelection>) {
        let Some(descriptor) = self
            .query_context
            .as_ref()
            .map(|context| context.descriptor.clone())
        else {
            return;
        };
        let Some(runtime) = self.runtime.clone() else {
            return;
        };
        let Some(session) = self
            .flow_workspace
            .current
            .as_mut()
            .filter(|session| session.expansion_mode())
        else {
            return;
        };
        let request = FlowExpansionRequest {
            expansion_version: 1,
            symbol: session.symbol.clone(),
            selections: selections.clone(),
            limits: Default::default(),
        };
        let generation = descriptor.generation.clone().unwrap_or_default();
        session.message = "Preparing expanded flow…".into();
        session.expansion.as_mut().unwrap().pending = Some(selections.clone());
        let workspace = &mut self.flow_workspace;
        workspace.expansion.cancel();
        workspace.serial += 1;
        workspace.expansion.cancel = Arc::new(AtomicBool::new(false));
        let cancel = workspace.expansion.cancel.clone();
        let serial = workspace.serial;
        let epoch = self.epoch;
        let symbol = request.symbol.clone();
        let input = workspace
            .expansion
            .workers
            .track((runtime, descriptor, request, cancel));
        workspace.expansion.job.request(input, move |input| {
            input.map(|(runtime, descriptor, request, cancel)| {
                let result = worker::prepare(runtime, descriptor, request, &cancel)
                    .map_err(|error| format!("{error:#}"));
                Output {
                    serial,
                    epoch,
                    symbol,
                    generation,
                    selections,
                    result,
                }
            })
        });
    }
    pub(crate) fn cancel_flow_expansion(&mut self) {
        self.flow_workspace.expansion.cancel();
        let Some(mode) = self
            .flow_workspace
            .current
            .as_ref()
            .and_then(|session| session.expansion.as_ref())
        else {
            return;
        };
        let unpublished = mode.published.is_none();
        self.flow_workspace.serial += 1;
        if unpublished {
            self.restore_ordinary_flow();
            return;
        }
        let session = self.flow_workspace.current.as_mut().unwrap();
        session.expansion.as_mut().unwrap().pending = None;
        session.message = "Expansion cancelled; previous chart retained.".into();
    }
    pub(crate) fn collapse_all_flow_expansion(&mut self) {
        if self
            .flow_workspace
            .current
            .as_ref()
            .is_none_or(|session| !session.expansion_mode())
        {
            return;
        }
        self.cancel_flow_expansion();
        self.restore_ordinary_flow();
    }
    fn restore_ordinary_flow(&mut self) {
        let Some(session) = self.flow_workspace.current.as_mut() else {
            return;
        };
        let Some(mode) = session.expansion.take() else {
            return;
        };
        *session = *mode.original;
        self.flow_workspace.presentation += 1;
        self.flow_workspace.fit_pending = false;
        self.flow_workspace.preserve_current_scroll();
    }
    pub(crate) fn expansion_source_mounted(&mut self, key: &str, tx: &mut rune::Tx) {
        self.flow_workspace.restore_mounted_source(key, tx);
        if let Some(session) = self.flow_workspace.current.as_mut() {
            session.restore_expansion_mount(key, tx);
        }
    }
}
impl Session {
    pub(crate) fn expansion_choice(
        &self,
        call: usize,
        target: usize,
    ) -> Option<(flow::OccurrencePath, flow::CallChoice)> {
        let result = self.expanded_flow()?;
        let selected = self.selected?;
        let origin = result.step_origins.get(selected)?;
        let site = origin.calls.get(call)?.clone();
        let target = self
            .ready
            .as_ref()?
            .chart
            .steps
            .get(selected)?
            .calls
            .get(call)?
            .targets
            .get(target)?
            .clone();
        let caller = result.occurrences.get(origin.occurrence.0)?.path.clone();
        Some((caller, flow::CallChoice { site, target }))
    }
    pub(crate) fn selected_occurrence(&self) -> Option<&flow::OccurrencePath> {
        let result = self.expanded_flow()?;
        let selected = self.selected?;
        let mut markers = result
            .occurrences
            .iter()
            .filter(|occurrence| occurrence.boundaries.marker == Some(selected));
        if let Some(marker) = markers.next() {
            return markers.next().is_none().then_some(&marker.path);
        }
        let origin = result.step_origins.get(selected)?;
        let occurrence = result.occurrences.get(origin.occurrence.0)?;
        occurrence.parent.map(|_| &occurrence.path)
    }
    pub(crate) fn flow_choice_selected(&self, call: usize, target: usize) -> bool {
        let Some((caller, choice)) = self.expansion_choice(call, target) else {
            return false;
        };
        self.expansion.as_ref().is_some_and(|mode| {
            mode.selections
                .iter()
                .any(|selection| selection.caller == caller && selection.choice == choice)
        })
    }
    fn replacing_expansion_choice(
        &self,
        selection: &FlowExpansionSelection,
    ) -> Vec<FlowExpansionSelection> {
        let mode = self.expansion.as_ref().unwrap();
        let caller = selection.caller.clone();
        let choice = selection.choice.clone();
        let old = mode
            .selections
            .iter()
            .find(|selection| selection.caller == caller && selection.choice.site == choice.site);
        let mut selections = if let Some(old) = old {
            let mut path = old.caller.clone();
            path.0.push(old.choice.clone());
            self.without_expansion_path(&path)
        } else {
            mode.selections.clone()
        };
        selections.push(FlowExpansionSelection { caller, choice });
        selections
    }
    fn without_expansion_path(&self, path: &flow::OccurrencePath) -> Vec<FlowExpansionSelection> {
        let mut parent = path.0.clone();
        let choice = parent.pop();
        self.expansion
            .as_ref()
            .unwrap()
            .selections
            .iter()
            .filter(|selection| {
                !selection.caller.0.starts_with(&path.0)
                    && !(selection.caller.0 == parent && Some(&selection.choice) == choice.as_ref())
            })
            .cloned()
            .collect()
    }
    fn restore_expansion_mount(&mut self, key: &str, tx: &mut rune::Tx) {
        let Some(mode) = self.expansion.as_mut() else {
            return;
        };
        let Some(mount) = mode.mounted.take() else {
            return;
        };
        if mount.key != key || self.scroll_key != key {
            mode.mounted = Some(mount);
            return;
        }
        if mount.center {
            self.center_selected(&SourceKeys::Single, tx);
        } else {
            self.scroll.set(mount.offset);
            tx.scroll_to_instant(key, mount.offset[0], mount.offset[1]);
        }
    }
    pub(super) fn mount_expansion_source(&mut self, center: bool) {
        let Some(key) = self.expansion_key().map(String::from) else {
            return;
        };
        let mode = self.expansion.as_mut().unwrap();
        if self.scroll_key == key && mode.mounted.is_none() {
            return;
        }
        let offset = mode.scrolls.get(&key).copied().unwrap_or([0.0, 0.0]);
        self.scroll = Rc::new(Cell::new(offset));
        self.scroll_key = key.clone();
        mode.mounted = Some(Mount {
            key,
            offset,
            center,
        });
    }
}

impl Session {
    pub(super) fn select_expanded_step(&mut self, index: usize, tx: &mut rune::Tx) -> Option<bool> {
        if !self.expansion_mode() {
            return None;
        }
        let old = self.expansion_key().map(String::from);
        let offset = self.scroll.get();
        if !self.select_index(index) {
            return Some(false);
        }
        if self.selected_source_span().is_some()
            && let Some(source) = self
                .expanded_flow()
                .and_then(|result| result.step_origins.get(index))
                .and_then(|origin| origin.source)
        {
            self.expansion.as_mut().unwrap().active = Some(source);
        }
        let changed = old.as_deref() != self.expansion_key();
        if changed {
            if let Some(key) = old {
                self.expansion.as_mut().unwrap().scrolls.insert(key, offset);
            }
            self.mount_expansion_source(true);
        } else if let Some(mount) = self.expansion.as_mut().unwrap().mounted.as_mut() {
            mount.center = true;
        } else {
            self.center_selected(&SourceKeys::Single, tx);
        }
        Some(changed)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod policy_tests;

impl Session {
    pub(super) fn finish_expansion_history(mut self) -> Self {
        let Some(mut mode) = self.expansion.take() else {
            return self;
        };
        if mode.published.is_none() {
            return *mode.original;
        }
        if mode.pending.take().is_some() {
            self.message.clear();
        }
        self.expansion = Some(mode);
        self
    }
}
