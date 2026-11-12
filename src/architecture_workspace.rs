//! Background configuration reload and classification, independent of rendering.
use crate::app::State;
use codecallnavigator::architecture::{LayerAssignments, Selection};
use rune::{Latest, Tx};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[derive(Default)]
pub struct Workspace {
    pub job: Latest<Output>,
    next_check: f64,
    signature: String,
    serial: u64,
    cancelled: Arc<AtomicBool>,
}

pub struct Output {
    serial: u64,
    epoch: u64,
    root: String,
    selection: Selection,
    signature: String,
    changed: bool,
    result: Result<Option<LayerAssignments>, String>,
}

impl Workspace {
    pub fn signature(&self) -> Option<String> {
        (!self.signature.is_empty()).then(|| self.signature.clone())
    }
    pub fn invalidate(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        self.cancelled = Arc::new(AtomicBool::new(false));
        self.next_check = 0.0;
        self.signature.clear();
        self.serial += 1;
        self.job.cancel();
    }
}

impl State {
    pub fn set_architecture_selection(&mut self, selection: Selection) {
        self.architecture_selection = Some(selection);
        self.architecture_workspace.invalidate();
        self.dirty_session = true;
    }

    pub fn set_layer_tab(&mut self, tab: Option<String>) {
        self.checkpoint();
        self.nav.layer_tab = tab;
        self.nav.expanded.clear();
        self.nav.overview();
        self.auto_fit = true;
        self.changed();
    }
}

pub fn poll(state: &mut State, time: f64, tx: &mut Tx) -> bool {
    let changed = take_result(state);
    if state.uses_fixture_queries() {
        return changed;
    }
    if state.closing || state.query_context.is_none() {
        return changed;
    }
    if time >= state.architecture_workspace.next_check
        && !state.architecture_workspace.job.is_pending()
    {
        request(state, time);
    }
    tx.wake_after(Duration::from_secs(2));
    changed
}

fn request(state: &mut State, time: f64) {
    let Ok(queries) = state.graph_queries() else {
        return;
    };
    let root = state
        .query_context
        .as_ref()
        .expect("production context")
        .descriptor
        .project
        .to_string_lossy()
        .into_owned();
    let workspace = &mut state.architecture_workspace;
    workspace.next_check = time + 2.0;
    let epoch = state.epoch;
    let serial = workspace.serial;
    let previous = workspace.signature.clone();
    let selection = state.architecture_selection.unwrap_or(Selection::None);
    let cancel = workspace.cancelled.clone();
    workspace.job.request(queries, move |queries| {
        match queries.architecture(Some(previous.clone()), &cancel) {
            Ok(result) => Output {
                serial,
                epoch,
                root,
                selection: result.selection,
                signature: result.signature,
                changed: result.changed,
                result: Ok(result.assignments),
            },
            Err(error) => Output {
                serial,
                epoch,
                root,
                selection,
                signature: previous,
                changed: false,
                result: Err(format!("{error:#}")),
            },
        }
    });
}

#[cfg(test)]
fn load(
    root: &std::path::Path,
    selection: Selection,
    graph: &codecallnavigator::model::GraphData,
    previous: &str,
) -> anyhow::Result<(String, Option<LayerAssignments>)> {
    if selection == Selection::None {
        return Ok(("None".into(), None));
    }
    let loaded = codecallnavigator::architecture::load(root, selection)?;
    let signature = serde_json::to_string(&(selection, &loaded.config, &loaded.origin))?;
    if signature == previous {
        return Ok((signature, None));
    }
    Ok((
        signature,
        Some(codecallnavigator::architecture::classify(
            graph,
            &loaded.config,
            loaded.origin,
        )?),
    ))
}

fn take_result(state: &mut State) -> bool {
    let Some(output) = state.architecture_workspace.job.take() else {
        return false;
    };
    apply_result(state, output)
}

fn apply_result(state: &mut State, output: Output) -> bool {
    let current = (
        state.architecture_workspace.serial,
        state.epoch,
        state
            .query_context
            .as_ref()
            .map(|context| context.descriptor.project.to_str().unwrap_or(""))
            .unwrap_or(state.root.text.trim()),
    );
    if (output.serial, output.epoch, output.root.as_str()) != current {
        return false;
    }
    match output.result {
        Ok(assignments) => {
            if state.warning.starts_with("Layer configuration:") {
                state.warning.clear();
            }
            state.architecture_selection = Some(output.selection);
            if output.changed {
                state.architecture_workspace.signature = output.signature;
                state.architecture = assignments.map(Arc::new);
                retain_tab(state);
                state.auto_fit = !state.has_graph_focus();
                state.changed();
            }
        }
        Err(error) => {
            state.warning =
                format!("Layer configuration: {error}. The last valid layer view is preserved.")
        }
    }
    true
}

fn retain_tab(state: &mut State) {
    let exists = state.architecture.as_ref().is_some_and(|architecture| {
        architecture
            .layers
            .iter()
            .any(|layer| layer.tab == state.nav.layer_tab)
    });
    if !exists {
        state.nav.layer_tab = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codecallnavigator::{
        architecture::{ArchitectureFacts, FACTS_VERSION, FileFacts, file_key},
        model::GraphData,
    };

    #[test]
    fn configuration_reload_reclassifies_cached_source_without_reading_source_files() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join(".codecallnavigator")).unwrap();
        let path = directory.path().join(".codecallnavigator/layers.yml");
        let config = |name: &str| {
            format!(
                "version: 1\nlanguages:\n  csharp:\n    layers:\n      - name: {name}\n        matchers:\n          - content: CachedController\n"
            )
        };
        std::fs::write(&path, config("Controller")).unwrap();
        let file = directory
            .path()
            .join("MissingController.cs")
            .to_string_lossy()
            .to_string();
        let graph = GraphData {
            architecture: Some(ArchitectureFacts {
                version: FACTS_VERSION,
                files: vec![FileFacts {
                    build: "build".into(),
                    project: "project".into(),
                    file: file.clone(),
                    language: "csharp".into(),
                    content: "class CachedController {}".into(),
                    ..Default::default()
                }],
            }),
            ..Default::default()
        };
        let (first_signature, first) =
            load(directory.path(), Selection::Repository, &graph, "").unwrap();
        let key = file_key("build", "project", &file);
        assert_eq!(first.unwrap().files[&key].layer, "Controller");
        assert!(
            load(
                directory.path(),
                Selection::Repository,
                &graph,
                &first_signature
            )
            .unwrap()
            .1
            .is_none()
        );
        std::fs::write(&path, config("Endpoint")).unwrap();
        let (second_signature, second) = load(
            directory.path(),
            Selection::Repository,
            &graph,
            &first_signature,
        )
        .unwrap();
        assert_ne!(first_signature, second_signature);
        assert_eq!(second.unwrap().files[&key].layer, "Endpoint");
        std::fs::write(&path, "version: 999").unwrap();
        assert!(
            load(
                directory.path(),
                Selection::Repository,
                &graph,
                &second_signature
            )
            .is_err()
        );
        assert!(!std::path::Path::new(&file).exists());
    }

    #[test]
    fn preset_selection_is_session_state_while_tabs_follow_navigation_history() {
        let mut state = State::new();
        state.architecture_selection = Some(Selection::Spring);
        state.set_layer_tab(Some("Database".into()));
        state.set_layer_tab(None);
        state.history(false);
        assert_eq!(state.nav.layer_tab.as_deref(), Some("Database"));
        assert_eq!(state.architecture_selection, Some(Selection::Spring));
        state.set_architecture_selection(Selection::React);
        state.history(false);
        assert_eq!(state.architecture_selection, Some(Selection::React));
        let saved = serde_json::to_value(state.session()).unwrap();
        let restored: crate::navigation::SavedSession = serde_json::from_value(saved).unwrap();
        assert_eq!(restored.architecture_selection, Some(Selection::React));
    }
    fn output(state: &State, result: Result<Option<LayerAssignments>, String>) -> Output {
        Output {
            serial: state.architecture_workspace.serial,
            epoch: state.epoch,
            root: state.root.text.clone(),
            selection: Selection::Repository,
            signature: "valid".into(),
            changed: true,
            result,
        }
    }

    #[test]
    fn invalid_config_preserves_last_assignments_and_recovery_installs_new_result() {
        let mut state = State::new();
        state.architecture = Some(Arc::new(LayerAssignments {
            origin: "old".into(),
            ..Default::default()
        }));
        state.nav.layer_tab = Some("Database".into());
        let failed = output(&state, Err("invalid YAML".into()));
        assert!(apply_result(&mut state, failed));
        assert_eq!(state.architecture.as_ref().unwrap().origin, "old");
        assert_eq!(state.nav.layer_tab.as_deref(), Some("Database"));
        let recovered = output(
            &state,
            Ok(Some(LayerAssignments {
                origin: "new".into(),
                ..Default::default()
            })),
        );
        assert!(apply_result(&mut state, recovered));
        assert_eq!(state.architecture.as_ref().unwrap().origin, "new");
        assert!(state.warning.is_empty());
        assert!(state.nav.layer_tab.is_none());
        let none = output(&state, Ok(None));
        assert!(apply_result(&mut state, none));
        assert!(state.architecture.is_none());
    }

    #[test]
    fn changing_graph_epoch_rejects_previous_configuration_job_and_forces_reload() {
        let mut state = State::new();
        state.architecture_workspace.signature = "old graph".into();
        let obsolete = output(&state, Ok(Some(LayerAssignments::default())));
        state.epoch += 1;
        state.architecture_workspace.invalidate();
        assert!(state.architecture_workspace.signature.is_empty());
        assert!(!apply_result(&mut state, obsolete));
        assert!(state.architecture.is_none());
    }
}
