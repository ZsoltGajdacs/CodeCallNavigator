//! Persisted navigation and its transitions; derived graph roots stay in projection.
use crate::{
    camera::Camera,
    projection::{FlowDirection, FocusTarget, Grouping, Scope},
};
use codegraph::graph::{Filter, GraphIndex};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::PathBuf};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Navigation {
    pub grouping: Grouping,
    pub layer_tab: Option<String>,
    pub expanded: HashSet<String>,
    pub focus: Option<String>,
    pub focus_target: Option<FocusTarget>,
    pub selected: Option<String>,
    pub direction: FlowDirection,
    pub depth: u32,
    pub filter: Filter,
    pub path_from: Option<String>,
    pub path_to: Option<String>,
    pub show_path: bool,
    pub page: usize,
    pub camera: Camera,
}

impl Default for Navigation {
    fn default() -> Self {
        Self {
            grouping: Grouping::Project,
            layer_tab: None,
            expanded: HashSet::new(),
            focus: None,
            focus_target: None,
            selected: None,
            direction: FlowDirection::Callees,
            depth: 1,
            filter: Filter::default(),
            path_from: None,
            path_to: None,
            show_path: false,
            page: 0,
            camera: Camera::default(),
        }
    }
}

impl Navigation {
    pub fn overview(&mut self) {
        self.focus = None;
        self.focus_target = None;
        self.show_path = false;
        self.page = 0;
    }

    pub fn set_grouping(&mut self, grouping: Grouping) {
        self.grouping = grouping;
        self.expanded.clear();
        self.overview();
        if grouping == Grouping::Method {
            self.direction = FlowDirection::Callees;
        }
    }

    pub fn set_direction(&mut self, direction: FlowDirection) {
        self.direction = direction;
        self.show_path = false;
        self.page = 0;
    }

    pub fn focus_method(&mut self, identity: String) {
        self.focus_target = Some(FocusTarget::Method(identity.clone()));
        self.focus = Some(identity);
        self.show_path = false;
        self.page = 0;
    }

    pub fn scope(&self, graph: &GraphIndex) -> Scope {
        let resolve = |value: &Option<String>| value.as_deref().and_then(|id| graph.node_index(id));
        if self.show_path
            && let Some((from, to)) = resolve(&self.path_from).zip(resolve(&self.path_to))
        {
            return Scope::Path(from, to);
        }
        resolve(&self.focus)
            .map(Scope::Focus)
            .unwrap_or(Scope::Overview)
    }

    pub fn retain_known(&mut self, graph: &GraphIndex) {
        let known = |identity: &String| graph.node_index(identity).is_some();
        self.selected = self.selected.take().filter(known);
        if let Some(FocusTarget::Method(identity)) = &self.focus_target
            && !known(identity)
        {
            self.focus_target = None;
        }
        self.focus = self.focus.take().filter(known);
        self.path_from = self.path_from.take().filter(known);
        self.path_to = self.path_to.take().filter(known);
        self.show_path &= self.path_from.is_some() && self.path_to.is_some();
        self.depth = self.depth.clamp(1, 64);
        self.camera.sanitize();
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SavedSession {
    pub root: String,
    pub selected_builds: Vec<PathBuf>,
    pub selected_targets: Option<Vec<crate::discovery::BuildIdentity>>,
    pub navigation: Navigation,
    pub presentation: crate::presentation::Preferences,
    pub architecture_selection: Option<codegraph::architecture::Selection>,
}
