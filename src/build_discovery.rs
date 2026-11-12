//! Background build discovery and identity-based selection reconciliation.
use crate::{
    app::{State, TargetChoice},
    discovery, engine_client,
    presentation::Overlay,
};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

pub struct Output {
    pub root: String,
    pub result: Result<Vec<discovery::BuildTarget>, String>,
}

#[derive(Default)]
pub enum Phase {
    #[default]
    Idle,
    Loading,
    Ready,
    Failed(String),
}

#[derive(Default)]
pub struct Workspace {
    pub phase: Phase,
    root: String,
    previous: Option<BTreeMap<discovery::BuildIdentity, bool>>,
}

impl Workspace {
    fn begin(&mut self, root: &str, targets: &[TargetChoice]) {
        if self.root != root {
            self.previous = None;
        } else if !targets.is_empty() || matches!(self.phase, Phase::Ready) {
            self.previous = Some(
                targets
                    .iter()
                    .map(|choice| (choice.target.identity(), choice.selected))
                    .collect(),
            );
        }
        self.root = root.to_owned();
        self.phase = Phase::Loading;
    }

    fn choices(&self, state: &State, targets: Vec<discovery::BuildTarget>) -> Vec<TargetChoice> {
        let defaults = discovery::default_selection(&targets);
        targets
            .into_iter()
            .map(|target| {
                let identity = target.identity();
                let selected = self
                    .previous
                    .as_ref()
                    .and_then(|previous| previous.get(&identity))
                    .copied()
                    .unwrap_or_else(|| {
                        if self.previous.is_some() {
                            defaults.contains(&identity)
                        } else if let Some(saved) = &state.saved_targets {
                            saved.contains(&identity)
                        } else if state.saved_builds.is_empty() {
                            defaults.contains(&identity)
                        } else {
                            target.kind != "maven" && state.saved_builds.contains(&target.path)
                        }
                    });
                TargetChoice { target, selected }
            })
            .collect()
    }
}

impl State {
    pub fn reset_build_discovery(&mut self) {
        self.jobs.discovery.cancel();
        self.clear_discovery_error();
        self.build_discovery = Workspace::default();
        self.targets.clear();
    }

    fn clear_discovery_error(&mut self) {
        if let Phase::Failed(error) = &self.build_discovery.phase
            && self.warning == *error
        {
            self.warning.clear();
        }
    }

    pub fn open_builds(&mut self) {
        self.cancel_graph_press();
        self.close_source();
        self.ui.overlay = Some(Overlay::Builds);
        self.discover_root();
    }

    pub fn discover_root(&mut self) {
        let root = self.root.text.trim().to_owned();
        if self.build_discovery.root != root {
            self.targets.clear();
        }
        self.clear_discovery_error();
        self.build_discovery.begin(&root, &self.targets);
        self.status = "Finding solutions, Java builds and TypeScript projects…".into();
        let runtime = self.runtime.clone();
        self.jobs
            .discovery
            .request(PathBuf::from(root), move |path| {
                let root = path.to_string_lossy().into_owned();
                Output {
                    root,
                    result: (|| {
                        let runtime = match runtime {
                            Some(runtime) => runtime,
                            None => Arc::new(crate::runtime::Runtime::discover()?),
                        };
                        engine_client::discover(&runtime, &path)
                    })()
                    .map_err(|error: anyhow::Error| error.to_string()),
                }
            });
    }
}

pub fn poll(state: &mut State) -> bool {
    let Some(output) = state.jobs.discovery.take() else {
        return false;
    };
    if output.root != state.root.text.trim() {
        return false;
    }
    match output.result {
        Ok(targets) => {
            state.targets = state.build_discovery.choices(state, targets);
            state.build_discovery.phase = Phase::Ready;
            state.status = format!("{} build targets found", state.targets.len());
            state.dirty_session = true;
        }
        Err(error) => {
            state.build_discovery.phase = Phase::Failed(error.clone());
            state.warning = error;
            state.status = "Could not inspect this folder".into();
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target(name: &str) -> discovery::BuildTarget {
        discovery::BuildTarget {
            path: format!("/repo/{name}.csproj").into(),
            kind: "project".into(),
            language: "csharp".into(),
            label: name.into(),
        }
    }
    #[test]
    fn rescan_keeps_deselections_and_defaults_new_targets() {
        let state = State::new();
        let mut workspace = Workspace::default();
        workspace.begin("/repo", &[]);
        let mut choices = workspace.choices(&state, vec![target("A"), target("B")]);
        assert!(choices.iter().all(|c| c.selected));
        workspace.phase = Phase::Ready;
        choices[0].selected = false;
        workspace.begin("/repo", &choices);
        let rescanned = workspace.choices(&state, vec![target("A"), target("C")]);
        assert!(!rescanned[0].selected);
        assert!(rescanned[1].selected);
        workspace.begin("/repo", &choices);
        assert!(!workspace.choices(&state, vec![target("A")])[0].selected);
        workspace.begin("/other", &[]);
        assert!(workspace.choices(&state, vec![target("A")])[0].selected);
    }
    #[test]
    fn empty_rescan_does_not_restore_obsolete_saved_selections() {
        let mut state = State::new();
        state.saved_targets = Some(vec![]);
        let mut workspace = Workspace::default();
        workspace.begin("/repo", &[]);
        assert!(!workspace.choices(&state, vec![target("A")])[0].selected);
        workspace.phase = Phase::Ready;
        workspace.begin("/repo", &[]);
        assert!(workspace.choices(&state, vec![]).is_empty());
        workspace.phase = Phase::Ready;
        workspace.begin("/repo", &[]);
        assert!(workspace.choices(&state, vec![target("B")])[0].selected);
    }
    #[test]
    fn changing_root_discards_old_targets_even_when_discovery_fails() {
        let mut state = State::new();
        state.root = rune::TextEdit::new("/old");
        state.build_discovery.root = "/old".into();
        state.targets = vec![TargetChoice {
            target: target("Old"),
            selected: true,
        }];
        state.root = rune::TextEdit::new("/new");
        state.discover_root();
        assert!(state.targets.is_empty());
        state.jobs.discovery.request((), |_| Output {
            root: "/new".into(),
            result: Err("Unreadable folder".into()),
        });
        for _ in 0..100 {
            if poll(&mut state) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(matches!(state.build_discovery.phase, Phase::Failed(_)));
        assert!(state.targets.is_empty());
        state.discover_root();
        assert!(state.warning.is_empty());
        state.jobs.discovery.cancel();
    }
    #[test]
    fn root_edit_clears_discovery_error_but_keeps_unrelated_warning() {
        let mut state = State::new();
        state.build_discovery.phase = Phase::Failed("Old folder error".into());
        state.warning = "Old folder error".into();
        state.root_edited();
        assert!(state.warning.is_empty());
        assert!(matches!(state.build_discovery.phase, Phase::Idle));
        state.build_discovery.phase = Phase::Failed("Old folder error".into());
        state.warning = "Unrelated warning".into();
        state.root_edited();
        assert_eq!(state.warning, "Unrelated warning");
    }
}
