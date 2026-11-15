//! A completed request publishes only into the same function, generation, epoch and edit serial.
use super::super::Workspace;
use crate::app::State;
use std::sync::Arc;
pub(crate) fn poll(state: &mut State) -> bool {
    let Some(output) = state.flow_workspace.expansion.job.take() else {
        return false;
    };
    publish(state, output.value)
}
pub(super) fn publish(state: &mut State, output: super::model::Output) -> bool {
    if !matches(state, &output) {
        return false;
    }
    let workspace = &mut state.flow_workspace;
    let Some(session) = workspace
        .current
        .as_mut()
        .filter(|session| session.symbol == output.symbol && session.expansion_mode())
    else {
        return false;
    };
    session.expansion.as_mut().unwrap().pending = None;
    match output.result {
        Ok(prepared) => {
            if let Some(key) = session.expansion_key().map(String::from) {
                session
                    .expansion
                    .as_mut()
                    .unwrap()
                    .scrolls
                    .insert(key, session.scroll.get());
            }
            let mode = session.expansion.as_mut().unwrap();
            if mode.published.is_none() {
                mode.scrolls.insert(
                    prepared.published.keys[prepared.published.root_source.0].clone(),
                    mode.original.scroll.get(),
                );
            }
            mode.active = Some(prepared.published.root_source);
            mode.published = Some(prepared.published);
            mode.selections = output.selections;
            session
                .camera
                .fit(prepared.ready.chart.size, workspace.viewport);
            session.ready = Some(Arc::new(prepared.ready));
            session.selected = None;
            session.message.clear();
            session.mount_expansion_source(false);
            workspace.fit_pending = true;
            workspace.presentation += 1;
            workspace.restore_scroll = None;
            workspace.mount_restore = None;
        }
        Err(error) => session.message = error,
    }
    true
}
fn matches(state: &State, output: &super::model::Output) -> bool {
    let generation = state
        .query_context
        .as_ref()
        .and_then(|context| context.descriptor.generation.as_deref());
    output.serial == state.flow_workspace.serial
        && output.epoch == state.epoch
        && generation == Some(output.generation.as_str())
}
impl Workspace {
    pub(super) fn restore_mounted_source(&mut self, key: &str, tx: &mut rune::Tx) {
        let current = self
            .current
            .as_ref()
            .is_some_and(|session| session.scroll_key == key);
        let pending = self
            .mount_restore
            .as_ref()
            .is_some_and(|restore| restore.key == key);
        if current && pending {
            let restore = self.mount_restore.take().unwrap();
            self.current.as_mut().unwrap().scroll.set(restore.offset);
            tx.scroll_to_instant(key, restore.offset[0], restore.offset[1]);
        }
    }
    pub(crate) fn set_expansion_waker(&self, waker: rune::Waker) {
        self.expansion.set_waker(waker);
    }
    pub(crate) fn expansion_pending(&self) -> bool {
        self.expansion.pending()
    }
    pub(crate) fn expansion_stopped(&self) -> bool {
        self.expansion.workers.stopped()
    }
}
