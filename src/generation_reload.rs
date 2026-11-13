//! Explicit adoption loads exactly the advertised saved generation, without analysis.
use crate::{app::State, engine_client::QueryContext, workspace::LoadedGraph};
use rune::Latest;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Default)]
pub struct Workspace {
    pub job: Latest<Output>,
    cancel: Arc<AtomicBool>,
}
pub struct Output {
    binding: crate::workspace::Binding,
    requested: String,
    result: Result<LoadedGraph, String>,
}

impl Workspace {
    pub fn stop(&self) {
        self.cancel.store(true, Ordering::Release);
    }
}

impl State {
    pub fn reload_generation(&mut self) {
        if self.project_job_busy() {
            return;
        }
        let (Some(context), Some(runtime)) = (self.query_context.clone(), self.runtime.clone())
        else {
            return;
        };
        let Some(generation) = self
            .freshness
            .new_generation(self.generation.as_deref())
            .map(str::to_owned)
        else {
            return;
        };
        self.reload.stop();
        self.reload.cancel = Arc::new(AtomicBool::new(false));
        let cancel = self.reload.cancel.clone();
        let binding = crate::workspace::Binding::capture(self);
        let mut target = context.descriptor;
        target.generation = Some(generation.clone());
        self.warning.clear();
        self.status = "Loading the saved graph generation…".into();
        self.reload.job.request(target, move |target| {
            let result = QueryContext::new(&runtime, target)
                .snapshot(&cancel)
                .map(crate::workspace::from_snapshot)
                .map_err(|error| format!("{error:#}"));
            Output {
                binding,
                requested: generation,
                result,
            }
        });
    }
}

pub fn poll(state: &mut State) -> bool {
    let Some(output) = state.reload.job.take() else {
        return false;
    };
    if state.closing || !output.binding.matches(state) {
        return false;
    }
    if state
        .freshness
        .last
        .as_ref()
        .is_some_and(|last| last.generation != output.requested)
    {
        state.status = "Saved graph changed again · reload the latest generation".into();
        return true;
    }
    match output.result {
        Ok(loaded) => {
            crate::graph_install::install(state, loaded, crate::graph_install::Mode::Adopt)
        }
        Err(error) => {
            state.warning =
                format!("Could not reload saved graph; previous graph preserved: {error}")
        }
    }
    true
}
