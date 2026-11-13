//! Presentation state is independent of graph navigation and projection.
use crate::app::State;
use codecallnavigator::{
    graph::Filter,
    model::{RelationshipKind, TargetResolution},
};
use rune_kit::PaneState;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Drawer {
    #[default]
    #[serde(other)]
    Repository,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overlay {
    Depth,
    Builds,
}

#[derive(Clone, Copy)]
pub enum Section {
    Path,
    Relationships,
    Callers,
    Callees,
    Analysis,
}

#[derive(Clone, Copy)]
pub enum FilterKind {
    Tests,
    Generated,
    Unavailable,
    Relationship(RelationshipKind),
    Resolution(TargetResolution),
}

impl FilterKind {
    pub(crate) fn enabled(self, filter: &Filter) -> bool {
        match self {
            Self::Tests => filter.include_tests,
            Self::Generated => filter.include_generated,
            Self::Unavailable => filter.include_unavailable,
            Self::Relationship(kind) => filter.relationship_kinds.contains(&kind),
            Self::Resolution(resolution) => filter.target_resolutions.contains(&resolution),
        }
    }

    fn toggle(self, filter: &mut Filter) {
        match self {
            Self::Tests => filter.include_tests = !filter.include_tests,
            Self::Generated => filter.include_generated = !filter.include_generated,
            Self::Unavailable => filter.include_unavailable = !filter.include_unavailable,
            Self::Relationship(kind) => {
                if !filter.relationship_kinds.remove(&kind) {
                    filter.relationship_kinds.insert(kind);
                }
            }
            Self::Resolution(resolution) => {
                if !filter.target_resolutions.remove(&resolution) {
                    filter.target_resolutions.insert(resolution);
                }
            }
        }
    }
}

/// Only stable preferences are serialized, not transient pane drags or popovers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub sidebar_width: f32,
    pub sidebar_open: bool,
    pub inspector_width: f32,
    pub inspector_open: bool,
    pub drawer: Drawer,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            sidebar_width: 280.0,
            sidebar_open: true,
            inspector_width: 300.0,
            inspector_open: true,
            drawer: Drawer::Repository,
        }
    }
}

pub struct Presentation {
    pub sidebar: PaneState,
    pub inspector: PaneState,
    pub drawer: Drawer,
    pub overlay: Option<Overlay>,
    /// Latest window content width; runtime-only, independent of saved pane preferences.
    pub window_width: f32,
    open_sections: [bool; 5],
}

impl Default for Presentation {
    fn default() -> Self {
        Self {
            sidebar: PaneState::new(280.0).range(240.0, 400.0),
            inspector: PaneState::new(300.0).range(260.0, 420.0),
            drawer: Drawer::Repository,
            overlay: None,
            window_width: 1420.0,
            open_sections: [true; 5],
        }
    }
}

impl Presentation {
    pub fn preferences(&self) -> Preferences {
        Preferences {
            sidebar_width: self.sidebar.width(),
            sidebar_open: self.sidebar.is_open(),
            inspector_width: self.inspector.width(),
            inspector_open: self.inspector.is_open(),
            drawer: self.drawer,
        }
    }

    pub fn restore(&mut self, preferences: Preferences) {
        let window_width = self.window_width;
        *self = Self::default();
        self.window_width = window_width;
        self.sidebar = self.sidebar.with_width(preferences.sidebar_width);
        self.inspector = self.inspector.with_width(preferences.inspector_width);
        self.sidebar.set_open(preferences.sidebar_open);
        self.inspector.set_open(preferences.inspector_open);
        self.drawer = preferences.drawer;
    }

    pub fn section_open(&self, section: Section) -> bool {
        self.open_sections[section as usize]
    }
}

/// Kit calls these accessors only while mutating panes (resize, reset or toggle).
pub fn sidebar(state: &mut State) -> &mut PaneState {
    state.dirty_session = true;
    &mut state.ui.sidebar
}

pub fn inspector(state: &mut State) -> &mut PaneState {
    state.dirty_session = true;
    &mut state.ui.inspector
}

impl State {
    /// Preserve a usable canvas when restoring wide panes into a smaller window.
    pub fn constrain_panes(&mut self, window_width: f32) -> bool {
        if !window_width.is_finite() {
            return false;
        }
        self.ui.window_width = window_width;
        let panes = [self.ui.sidebar, self.ui.inspector];
        let shown: f32 = panes.iter().map(PaneState::shown_width).sum();
        let budget = window_width - crate::theme::RAIL_WIDTH - 448.0;
        if shown <= budget {
            return false;
        }
        let headroom: f32 = panes
            .iter()
            .filter(|pane| pane.is_open())
            .map(|pane| pane.width() - pane.min_width())
            .sum();
        if headroom <= 0.0 {
            return false;
        }
        let retained = (1.0 - (shown - budget) / headroom).clamp(0.0, 1.0);
        let mut changed = false;
        for pane in [&mut self.ui.sidebar, &mut self.ui.inspector] {
            if pane.is_open() {
                let before = pane.width();
                let width = pane.min_width() + (before - pane.min_width()) * retained;
                pane.set_width(width.floor());
                changed |= pane.width() != before;
            }
        }
        self.dirty_session |= changed;
        changed
    }

    pub fn toggle_drawer(&mut self, _: Drawer) {
        self.ui.sidebar.toggle();
        self.ui.overlay = None;
        self.dirty_session = true;
    }

    pub fn toggle_inspector(&mut self) {
        self.ui.inspector.toggle();
        self.dirty_session = true;
    }

    pub fn toggle_section(&mut self, section: Section) {
        let open = &mut self.ui.open_sections[section as usize];
        *open = !*open;
    }

    pub fn toggle_depth(&mut self) {
        self.ui.overlay = match self.ui.overlay {
            Some(Overlay::Depth) => None,
            _ => Some(Overlay::Depth),
        };
    }

    pub fn close_overlay(&mut self) {
        self.ui.overlay = None;
    }

    pub fn set_depth(&mut self, depth: u32) {
        self.close_overlay();
        let depth = depth.clamp(1, 64);
        if self.nav.depth != depth {
            self.checkpoint();
            self.nav.depth = depth;
            self.changed();
        }
    }

    pub fn toggle_filter(&mut self, kind: FilterKind) {
        self.checkpoint();
        kind.toggle(&mut self.nav.filter);
        self.changed();
    }

    pub fn toggle_build(&mut self, index: usize) {
        if let Some(choice) = self.targets.get_mut(index) {
            choice.selected = !choice.selected;
            self.dirty_session = true;
        }
    }

    pub fn zoom(&mut self, factor: f32) {
        self.cancel_graph_camera();
        let center = [self.viewport[0] / 2.0, self.viewport[1] / 2.0];
        self.nav.camera.zoom_at(center, factor);
        self.dirty_session = true;
    }

    pub fn fit_graph(&mut self) {
        self.preserve_adoption_camera = false;
        self.cancel_graph_camera();
        self.fit_visible_graph();
        self.dirty_session = true;
    }

    pub fn next_graph_page(&mut self) {
        self.nav.page = (self.nav.page + 1) % self.graph_view.pages.max(1);
        self.auto_fit = !self.has_graph_focus();
        self.changed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app::State, navigation::SavedSession};

    fn graph_state(state: &State) -> serde_json::Value {
        serde_json::json!({
            "navigation": state.nav,
            "back": state.back,
            "forward": state.forward,
            "revision": state.revision,
            "needs_layout": state.needs_layout,
            "auto_fit": state.auto_fit,
        })
    }

    #[test]
    fn narrowing_window_reserves_canvas_and_preserves_graph_state() {
        let mut state = State::new();
        state.ui.sidebar.set_width(400.0);
        state.ui.inspector.set_width(420.0);
        state.nav.camera.pan = [120.0, -40.0];
        state.nav.camera.zoom = 0.7;
        state.auto_fit = false;
        state.needs_layout = false;
        let before = graph_state(&state);
        assert!(state.constrain_panes(1050.0));
        assert!(state.ui.sidebar.width() + state.ui.inspector.width() <= 554.0);
        for pane in [state.ui.sidebar, state.ui.inspector] {
            assert!(pane.width() >= pane.min_width());
        }
        assert_eq!(graph_state(&state), before);
        assert!(state.dirty_session);
        state.dirty_session = false;
        state.constrain_panes(1050.0);
        assert!(!state.dirty_session);
    }

    #[test]
    fn restoring_preferences_keeps_actual_window_width() {
        let mut ui = Presentation {
            window_width: 1050.0,
            ..Presentation::default()
        };
        ui.restore(Preferences::default());
        assert_eq!(ui.window_width, 1050.0);
        let serialized = serde_json::to_value(ui.preferences()).unwrap();
        assert!(serialized.get("window_width").is_none());
    }

    #[test]
    fn polling_constrains_panes_expanded_after_initial_layout() {
        let mut state = State::new();
        state.scene = true;
        state.needs_layout = false;
        state.auto_fit = false;
        state.constrain_panes(1050.0);
        let mut runtime = rune::Runtime::new(
            state,
            |_, _| rune::el().w_full().h_full(),
            (1050.0, 680.0),
            rune::Env::new(true, 1.0),
        );
        runtime.poll = Some(crate::jobs::poll);
        runtime.frame(0.0);
        let before = graph_state(&runtime.state);
        runtime.state.ui.sidebar.set_width(400.0);
        runtime.state.ui.inspector.set_width(420.0);
        runtime.frame(1.0 / 60.0);
        assert!(runtime.state.ui.sidebar.width() + runtime.state.ui.inspector.width() <= 554.0);
        assert_eq!(graph_state(&runtime.state), before);
        assert_eq!(runtime.state.ui.window_width, 1050.0);
    }

    #[test]
    fn pane_constraint_leaves_closed_pane_preference_untouched() {
        let mut state = State::new();
        state.ui.sidebar.set_width(400.0);
        state.ui.sidebar.set_open(false);
        state.ui.inspector.set_width(420.0);
        state.constrain_panes(1050.0);
        assert_eq!(state.ui.sidebar.width(), 400.0);
        assert_eq!(state.ui.inspector.width(), 420.0);
        assert!(!state.dirty_session);
    }

    #[test]
    fn legacy_session_opens_default_graph_studio_panes() {
        let session: SavedSession =
            serde_json::from_str(r#"{"root":"/repo","selected_builds":[],"navigation":{}}"#)
                .unwrap();
        let mut ui = Presentation::default();
        ui.restore(session.presentation);
        assert_eq!(ui.preferences(), Preferences::default());
    }

    #[test]
    fn preferences_round_trip_without_transient_overlay_or_disclosures() {
        let mut state = State::new();
        state.toggle_drawer(Drawer::Repository);
        state.ui.sidebar.set_width(344.0);
        state.toggle_inspector();
        state.toggle_depth();
        state.toggle_section(Section::Analysis);
        let saved = serde_json::to_string(&state.session()).unwrap();
        let session: SavedSession = serde_json::from_str(&saved).unwrap();
        let mut restored = Presentation::default();
        restored.restore(session.presentation);
        assert_eq!(restored.preferences(), state.ui.preferences());
        assert_eq!(restored.overlay, None);
        assert!(restored.section_open(Section::Analysis));
    }

    #[test]
    fn legacy_search_and_filter_drawers_restore_repository() {
        for value in ["filters", "search"] {
            let prefs: Preferences =
                serde_json::from_value(serde_json::json!({ "drawer": value })).unwrap();
            let mut ui = Presentation::default();
            ui.restore(prefs);
            assert_eq!(ui.drawer, Drawer::Repository);
        }
    }

    #[test]
    fn restore_clamps_widths_and_keeps_factory_reset_widths() {
        let prefs = Preferences {
            sidebar_width: 900.0,
            inspector_width: -20.0,
            ..Preferences::default()
        };
        let mut ui = Presentation::default();
        ui.restore(prefs);
        assert_eq!(ui.sidebar.width(), 400.0);
        assert_eq!(ui.inspector.width(), 260.0);
        ui.sidebar.reset_width();
        ui.inspector.reset_width();
        assert_eq!(ui.sidebar.width(), 280.0);
        assert_eq!(ui.inspector.width(), 300.0);
    }

    #[test]
    fn presentation_changes_do_not_change_graph_camera_or_history() {
        let mut state = State::new();
        state.auto_fit = false;
        state.needs_layout = false;
        state.nav.camera.pan = [123.0, -45.0];
        state.nav.camera.zoom = 0.72;
        let before = graph_state(&state);
        state.toggle_drawer(Drawer::Repository);
        state.toggle_drawer(Drawer::Repository);
        state.open_search();
        state.toggle_inspector();
        sidebar(&mut state).set_width(322.0);
        inspector(&mut state).set_width(278.0);
        assert_eq!(graph_state(&state), before);
        assert!(state.ui.sidebar.is_open());
        assert!(state.search.input.focused);
        assert!(state.dirty_session);
    }
}
