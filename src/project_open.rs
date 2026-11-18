//! Commit a folder selection and open its saved graph independently of build discovery.
use crate::{app::State, engine_client::Descriptor, workspace::LoadedGraph};
use rune::{Latest, TextEdit, Waker};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Default)]
pub(crate) struct Workspace {
    pub folder: Option<String>,
    pub linked_roots: BTreeMap<String, Vec<(String, PathBuf)>>,
    job: Latest<anyhow::Result<Option<LoadedGraph>>>,
    cancel: Option<Arc<AtomicBool>>,
}

impl Workspace {
    pub fn set_waker(&self, waker: Waker) {
        self.job.set_waker(waker);
    }

    pub fn loading(&self) -> bool {
        self.cancel.is_some()
    }

    pub fn stop(&self) {
        if let Some(cancel) = &self.cancel {
            cancel.store(true, Ordering::Release);
        }
    }

    fn supersede(&mut self) {
        self.stop();
        self.cancel = None;
        self.job.cancel();
    }

    pub fn worker_pending(&self) -> bool {
        self.job.is_pending()
    }
}

impl State {
    pub(crate) fn open_project(&mut self, folder: String) -> bool {
        let folder = folder.trim();
        if folder.is_empty() || self.project_job_busy() {
            return false;
        }
        let identity = crate::source_path::identity(folder);
        self.root = TextEdit::new(folder);
        self.dirty_session = true;
        self.close_overlay();
        if self
            .project_open
            .folder
            .as_deref()
            .is_some_and(|active| crate::source_path::identity(active) == identity)
        {
            self.discover_root();
            if self.graph.is_none() && !self.project_open.loading() {
                self.open_saved_project_graph();
            }
            return true;
        }

        self.project_open.supersede();
        self.project_open.folder = Some(folder.to_owned());
        self.root_was_edited = true;
        self.warning.clear();
        self.reset_build_discovery();
        self.saved_builds.clear();
        self.saved_targets = None;
        crate::graph_install::clear(self);
        self.discover_root();
        self.open_saved_project_graph();
        true
    }

    pub(crate) fn open_saved_project_graph(&mut self) {
        let (Some(folder), Some(runtime)) = (&self.project_open.folder, self.runtime.clone())
        else {
            return;
        };
        let target = Descriptor {
            project: folder.into(),
            roots: self
                .project_open
                .linked_roots
                .get(&crate::source_path::identity(folder))
                .cloned()
                .unwrap_or_default(),
            generation: None,
        };
        let cancel = Arc::new(AtomicBool::new(false));
        self.project_open.cancel = Some(cancel.clone());
        self.project_open.job.request(target, move |target| {
            crate::workspace::load_saved_graph(&runtime, target, &cancel)
        });
    }

    pub(crate) fn remember_linked_roots(&mut self) {
        let Some(context) = &self.query_context else {
            return;
        };
        let descriptor = &context.descriptor;
        let folder = self
            .project_open
            .folder
            .as_deref()
            .unwrap_or_else(|| descriptor.project.to_str().unwrap_or_default());
        let key = crate::source_path::identity(folder);
        if descriptor.roots.iter().any(|(name, _)| name != "repo") {
            self.project_open
                .linked_roots
                .insert(key, descriptor.roots.clone());
        } else {
            self.project_open.linked_roots.remove(&key);
        }
    }
}

pub(crate) fn poll(state: &mut State) -> bool {
    let Some(result) = state.project_open.job.take() else {
        return false;
    };
    state.project_open.cancel = None;
    if state.closing {
        return true;
    }
    match result {
        Ok(Some(graph)) => {
            crate::graph_install::install(state, graph, crate::graph_install::Mode::NewWorkspace);
        }
        Ok(None) => {}
        Err(error) => {
            if !matches!(
                state.build_discovery.phase,
                crate::build_discovery::Phase::Failed(_)
            ) {
                state.warning = format!("Could not open the saved graph: {error:#}");
            }
        }
    }
    true
}
