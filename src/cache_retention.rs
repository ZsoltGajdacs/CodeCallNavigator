//! One derived-cache writer; only accepted UI state owns persisted graph addressing.
use crate::{app::State, engine_client::Snapshot, presentation_cache::PresentationCache};
use codecallnavigator::store::SqliteStore;
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
};

struct Request {
    binding: crate::workspace::Binding,
    snapshot: Snapshot,
    cancel: Arc<AtomicBool>,
}
struct Output {
    binding: crate::workspace::Binding,
    result: Result<PresentationCache, String>,
}
#[derive(Default)]
struct Queue {
    pending: Option<Request>,
    active: Option<Arc<AtomicBool>>,
    output: Option<Output>,
    shutdown: bool,
    finished: bool,
}
#[derive(Default)]
pub struct Workspace {
    shared: Option<Arc<(Mutex<Queue>, Condvar)>>,
}

impl Workspace {
    fn request(&mut self, request: Request, writer: Writer) {
        let shared = self.shared.get_or_insert_with(|| {
            let shared = Arc::new((Mutex::new(Queue::default()), Condvar::new()));
            let owner = shared.clone();
            std::thread::spawn(move || writer.run(owner));
            shared
        });
        if let Ok(mut queue) = shared.0.lock() {
            if let Some(active) = &queue.active {
                active.store(true, Ordering::Release);
            }
            queue.pending = Some(request);
            shared.1.notify_one();
        }
    }

    pub fn stop(&mut self) {
        if let Some(shared) = &self.shared
            && let Ok(mut queue) = shared.0.lock()
        {
            queue.shutdown = true;
            queue.pending = None;
            if let Some(active) = &queue.active {
                active.store(true, Ordering::Release);
            }
            shared.1.notify_one();
        }
    }

    pub fn stopped(&self) -> bool {
        self.shared
            .as_ref()
            .is_none_or(|shared| shared.0.lock().is_ok_and(|queue| queue.finished))
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        self.stop();
    }
}

struct Writer {
    store: Arc<SqliteStore>,
    lock: Arc<Mutex<()>>,
    wake: Option<rune::Waker>,
}

impl Writer {
    fn run(self, shared: Arc<(Mutex<Queue>, Condvar)>) {
        let Self { store, lock, wake } = self;
        while let Some(request) = next(&shared) {
            let result = (|| {
                let _guard = lock
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Presentation cache writer failed"))?;
                crate::presentation_cache::retain(&store, &request.snapshot, &request.cancel)
            })()
            .map_err(|error| format!("{error:#}"));
            if let Ok(mut queue) = shared.0.lock() {
                queue.active = None;
                queue.output = Some(Output {
                    binding: request.binding,
                    result,
                });
            }
            if let Some(wake) = &wake {
                wake();
            }
        }
        if let Ok(mut queue) = shared.0.lock() {
            queue.finished = true;
        }
        if let Some(wake) = &wake {
            wake();
        }
    }
}

fn next(shared: &(Mutex<Queue>, Condvar)) -> Option<Request> {
    let mut queue = shared.0.lock().ok()?;
    while !queue.shutdown {
        if let Some(request) = queue.pending.take() {
            queue.active = Some(request.cancel.clone());
            return Some(request);
        }
        queue = shared.1.wait(queue).ok()?;
    }
    None
}

pub(crate) fn request(state: &mut State, navigation: Arc<crate::projection::NavigationInventory>) {
    let (Some(context), Some(graph), Some(store)) = (
        state.query_context.clone(),
        state.graph.clone(),
        state.store.clone(),
    ) else {
        return;
    };
    let request = Request {
        binding: crate::workspace::Binding::capture(state),
        snapshot: Snapshot {
            graph,
            context,
            navigation,
        },
        cancel: Arc::new(AtomicBool::new(false)),
    };
    state.cache_retention.request(
        request,
        Writer {
            store,
            lock: state.jobs.save_lock.clone(),
            wake: state.jobs.waker.clone(),
        },
    );
}

pub(crate) fn poll(state: &mut State) -> bool {
    let output = state
        .cache_retention
        .shared
        .as_ref()
        .and_then(|shared| shared.0.lock().ok()?.output.take());
    output.is_some_and(|output| apply(state, output))
}

fn apply(state: &mut State, output: Output) -> bool {
    if state.closing || !output.binding.matches(state) {
        return false;
    }
    match output.result {
        Ok(marker) => {
            state.presentation_cache = Some(marker);
            state.dirty_session = true;
        }
        Err(error) => {
            state.warning = format!("Graph loaded; profiling cache retention failed: {error}")
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{engine_client::QueryContext, graph_install, workspace};
    use std::{
        fs,
        path::Path,
        time::{Duration, Instant},
    };

    fn snapshot(root: &Path, note: &str) -> Snapshot {
        let data = codecallnavigator::model::GraphData {
            diagnostics: vec![note.into()],
            ..Default::default()
        };
        let artifact =
            codecallnavigator::artifact::canonical(&data, &[("repo".into(), root.into())]).unwrap();
        fs::create_dir_all(root.join(".codecallnavigator")).unwrap();
        fs::write(
            root.join(codecallnavigator::artifact::GRAPH_PATH),
            artifact.bytes().unwrap(),
        )
        .unwrap();
        QueryContext::for_project(&crate::runtime::Runtime::discover().unwrap(), root.into())
            .snapshot(&AtomicBool::new(false))
            .unwrap()
    }

    fn accepted(state: &mut State, snapshot: Snapshot) {
        graph_install::install(
            state,
            workspace::from_snapshot(snapshot),
            graph_install::Mode::Adopt,
        );
    }

    fn wait(state: &mut State, ready: impl Fn(&State) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !ready(state) && Instant::now() < deadline {
            poll(state);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(ready(state), "Retention did not finish: {}", state.warning);
    }

    #[test]
    fn latest_accepted_snapshot_is_only_session_owner_and_preserves_new_settings() {
        let project = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let store = Arc::new(SqliteStore::open(cache.path()).unwrap());
        store
            .save_session(
                &serde_json::json!({"root":"pending-folder","opaque":{"future":"keep-me"}}),
            )
            .unwrap();
        let mut state = State::new();
        state.store = Some(store.clone());
        state.root = rune::TextEdit::new("pending-folder");
        let lock = state.jobs.save_lock.clone();
        let guard = lock.lock().unwrap();
        accepted(&mut state, snapshot(project.path(), "first"));
        let shared = state.cache_retention.shared.as_ref().unwrap().clone();
        let deadline = Instant::now() + Duration::from_secs(2);
        while !shared.0.lock().unwrap().active.is_some() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(shared.0.lock().unwrap().active.is_some());
        accepted(&mut state, snapshot(project.path(), "superseded pending"));
        accepted(&mut state, snapshot(project.path(), "latest accepted"));
        let expected = state.generation.clone().unwrap();
        state.nav.camera.pan = [177.0, -52.0];
        workspace::save_session(&store, serde_json::to_value(state.session()).unwrap()).unwrap();
        drop(guard);
        wait(&mut state, |state| state.presentation_cache.is_some());
        assert_eq!(
            state.presentation_cache.as_ref().unwrap().source_generation,
            expected
        );
        workspace::save_session(&store, serde_json::to_value(state.session()).unwrap()).unwrap();
        let saved = store.load_session().unwrap().unwrap();
        assert_eq!(saved["graph_context"]["generation"], expected);
        assert_eq!(saved["presentation_cache"]["source_generation"], expected);
        assert_eq!(
            saved["navigation"]["camera"]["pan"],
            serde_json::json!([177.0, -52.0])
        );
        assert_eq!(saved["opaque"]["future"], "keep-me");
        assert_eq!(saved["root"], "pending-folder");
        let retained = store.load_active().unwrap().unwrap();
        assert_eq!(retained.diagnostics, vec!["latest accepted"]);
        state.cache_retention.stop();
        wait(&mut state, |state| state.cache_retention.stopped());
    }

    fn linked_snapshots(
        project: &Path,
        first_root: &Path,
        second_root: &Path,
    ) -> (Snapshot, Snapshot) {
        use crate::engine_client::Descriptor;
        use codecallnavigator::{
            artifact,
            model::{GraphData, Node},
        };
        for (root, text) in [
            (first_root, "first location"),
            (second_root, "second location"),
        ] {
            fs::write(root.join("Shared.java"), text).unwrap();
        }
        let data = GraphData {
            nodes: vec![Node {
                id: "shared-method".into(),
                kind: "method".into(),
                language: "java".into(),
                label: "leaf".into(),
                signature: "leaf()".into(),
                project: "Shared".into(),
                build: first_root.join("pom.xml").to_string_lossy().into_owned(),
                file: first_root
                    .join("Shared.java")
                    .to_string_lossy()
                    .into_owned(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let artifact = artifact::canonical(
            &data,
            &[
                ("repo".into(), project.into()),
                ("shared".into(), first_root.into()),
            ],
        )
        .unwrap();
        fs::create_dir(project.join(".codecallnavigator")).unwrap();
        fs::write(
            project.join(artifact::GRAPH_PATH),
            artifact.bytes().unwrap(),
        )
        .unwrap();
        let runtime = crate::runtime::Runtime::discover().unwrap();
        let load = |root: &Path| {
            QueryContext::new(
                &runtime,
                Descriptor {
                    project: project.into(),
                    roots: vec![("shared".into(), root.into())],
                    generation: None,
                },
            )
            .snapshot(&AtomicBool::new(false))
            .unwrap()
        };
        (load(first_root), load(second_root))
    }

    fn binding_after_remap_check(
        state: &mut State,
        next: &crate::engine_client::Descriptor,
    ) -> workspace::Binding {
        let binding = workspace::Binding::capture(state);
        let previous = state.query_context.as_ref().unwrap().descriptor.clone();
        state.query_context.as_mut().unwrap().descriptor.roots = next.roots.clone();
        assert!(
            !binding.matches(state),
            "Root remapping alone invalidates a pending reply"
        );
        state.query_context.as_mut().unwrap().descriptor = previous;
        binding
    }

    #[test]
    fn linked_root_remap_keeps_primary_workspace_and_rejects_old_root_jobs_and_cache() {
        let project = tempfile::tempdir().unwrap();
        let first_root = tempfile::tempdir().unwrap();
        let second_root = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let (first, second) =
            linked_snapshots(project.path(), first_root.path(), second_root.path());
        assert_eq!(
            first.context.descriptor.generation,
            second.context.descriptor.generation
        );
        assert_eq!(first.graph.data.nodes[0].id, second.graph.data.nodes[0].id);
        let symbol = first.graph.data.nodes[0].id.clone();
        let store = Arc::new(SqliteStore::open(cache.path()).unwrap());
        let mut state = State::new();
        state.store = Some(store.clone());
        accepted(&mut state, first);
        wait(&mut state, |state| state.presentation_cache.is_some());
        let old_marker = state.presentation_cache.clone().unwrap();
        workspace::save_session(&store, serde_json::to_value(state.session()).unwrap()).unwrap();
        state.nav.selected = Some(symbol.clone());
        state.nav.camera.pan = [128.0, -38.0];
        state.nav.camera.zoom = 0.72;
        state.back.push(state.nav.clone());
        let old_epoch = state.epoch;
        let old_binding = binding_after_remap_check(&mut state, &second.context.descriptor);
        accepted(&mut state, second);
        assert!(state.epoch > old_epoch);
        assert!(!apply(
            &mut state,
            Output {
                binding: old_binding,
                result: Ok(old_marker.clone())
            }
        ));
        assert_eq!(state.nav.selected.as_deref(), Some(symbol.as_str()));
        assert_eq!(state.back.len(), 1);
        assert_eq!(state.nav.camera.pan, [128.0, -38.0]);
        assert_eq!(state.nav.camera.zoom, 0.72);
        assert_eq!(
            state.graph.as_ref().unwrap().data.nodes[0].file,
            second_root.path().join("Shared.java").to_string_lossy()
        );
        wait(&mut state, |state| state.presentation_cache.is_some());
        let marker = state.presentation_cache.as_ref().unwrap();
        assert_ne!(
            marker.cache_generation, old_marker.cache_generation,
            "Identical portable bytes must not reuse the former physical-root cache"
        );
        assert_eq!(
            store
                .load_generation(&marker.cache_generation)
                .unwrap()
                .nodes[0]
                .file,
            second_root.path().join("Shared.java").to_string_lossy()
        );
        workspace::save_session(&store, serde_json::to_value(state.session()).unwrap()).unwrap();
        assert_eq!(
            store.load_session().unwrap().unwrap()["graph_context"]["roots"][0][1],
            second_root.path().to_string_lossy().as_ref()
        );
        state.cache_retention.stop();
        wait(&mut state, |state| state.cache_retention.stopped());
    }

    #[test]
    fn retention_failure_preserves_displayed_cli_graph_and_reports_no_marker() {
        let project = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let store = Arc::new(SqliteStore::open(cache.path()).unwrap());
        fs::remove_dir_all(cache.path()).unwrap();
        let mut state = State::new();
        state.store = Some(store);
        accepted(
            &mut state,
            snapshot(project.path(), "valid displayed graph"),
        );
        wait(&mut state, |state| {
            state.warning.contains("retention failed")
        });
        assert_eq!(
            state.graph.as_ref().unwrap().data.diagnostics,
            vec!["valid displayed graph"]
        );
        assert!(state.query_context.is_some());
        assert!(state.presentation_cache.is_none());
        state.cache_retention.stop();
        wait(&mut state, |state| state.cache_retention.stopped());
    }
}
