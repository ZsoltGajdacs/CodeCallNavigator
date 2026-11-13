//! Desktop scheduling for the engine-owned project cache operation.
use crate::{app::State, engine_client};
use std::path::PathBuf;

pub(crate) const CLEARED: &str = "Cache cleared · start analysis to rebuild.";

pub(crate) struct Output {
    root: String,
    result: Result<u32, String>,
}

impl State {
    pub(crate) fn project_job_busy(&self) -> bool {
        self.analyzing || self.clearing_cache || self.closing || self.reload.job.is_pending()
    }

    pub(crate) fn can_clear_cache(&self) -> bool {
        !self.project_job_busy() && self.runtime.is_some() && !self.root.text.trim().is_empty()
    }

    pub(crate) fn clear_cache(&mut self) {
        if !self.can_clear_cache() {
            return;
        }
        let Some(runtime) = self.runtime.clone() else {
            return;
        };
        let root = self.root.text.trim().to_owned();
        self.clearing_cache = true;
        self.warning.clear();
        self.status = "Clearing project analysis cache…".into();
        self.jobs.cache_clear.request(root, move |root| Output {
            result: engine_client::clear_cache(&runtime, &PathBuf::from(&root))
                .map_err(|error| format!("{error:#}")),
            root,
        });
    }
}

pub(crate) fn poll(state: &mut State) -> bool {
    let Some(output) = state.jobs.cache_clear.take() else {
        return false;
    };
    state.clearing_cache = false;
    // A folder edit must not make another project's cache look cleared.
    if output.root != state.root.text.trim() {
        if let Err(error) = output.result {
            state.warning = format!("Could not clear cache for {}: {error}", output.root);
        }
        return true;
    }
    match output.result {
        Ok(_) => state.status = CLEARED.into(),
        Err(error) => {
            state.warning = format!("Could not clear analysis cache: {error}");
            state.status = "Cache clearing failed · retry when ready".into();
        }
    }
    true
}
