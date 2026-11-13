//! Graph gestures and navigation actions. Rendering never infers traversal policy.
use crate::{
    app::State,
    navigation::Navigation,
    projection::{FocusTarget, GroupKind, Grouping},
};
use std::time::Duration;

#[derive(Clone, Copy)]
pub enum SelectionAction {
    Inspect,
    Focus,
}

#[derive(Default)]
pub struct Interaction {
    pending: Option<PendingPress>,
    pub center_after_layout: bool,
    pub filter_height: f32,
}

pub struct FileFocusOutput {
    epoch: u64,
    revision: u64,
    path: String,
    found: bool,
    result: Result<bool, String>,
}

pub(crate) fn poll_file_focus(state: &mut State) -> bool {
    let Some(output) = state.jobs.file_focus.take() else {
        return false;
    };
    if output.epoch != state.epoch || output.revision != state.revision {
        return false;
    }
    match output.result {
        Ok(true) => state.finish_file_reveal(output.path),
        Ok(false) => {
            state.status = if output.found {
                "Current filters hide the classes in this file"
            } else {
                "No analyzed classes for this file"
            }
            .into()
        }
        Err(error) => state.warning = error,
    }
    true
}

struct PendingPress {
    key: String,
    member: Option<u32>,
    deadline: Option<f64>,
    interval: f64,
    previous: Navigation,
}

#[cfg(all(target_os = "macos", not(test)))]
pub fn double_click_interval() -> f64 {
    // SAFETY: Graph gestures execute on Rune's main UI thread.
    unsafe { objc2_app_kit::NSEvent::doubleClickInterval() }
}

#[cfg(any(not(target_os = "macos"), test))]
pub fn double_click_interval() -> f64 {
    0.5
}

pub fn poll(state: &mut State, time: f64, tx: &mut rune::Tx) -> bool {
    let Some(pending) = &mut state.graph_interaction.pending else {
        return false;
    };
    let deadline = *pending.deadline.get_or_insert(time + pending.interval);
    if time < deadline {
        tx.wake_after(Duration::from_secs_f64(deadline - time));
        return false;
    }
    let pending = state
        .graph_interaction
        .pending
        .take()
        .expect("pending press");
    state.record_graph_step(pending.previous);
    state.focus_visible(pending.key, pending.member);
    true
}

impl State {
    pub fn cancel_graph_camera(&mut self) {
        self.cancel_graph_press();
        self.graph_interaction.center_after_layout = false;
    }

    pub fn cancel_graph_press(&mut self) {
        self.graph_interaction.pending = None;
    }

    pub fn press_graph_node(&mut self, key: String, member: Option<u32>, interval: f64) {
        // Rune also emits a press before double-press. Keep the pre-gesture snapshot.
        let previous = self
            .graph_interaction
            .pending
            .take()
            .map(|pending| pending.previous)
            .unwrap_or_else(|| self.nav.clone());
        self.apply_visible_action(key.clone(), member, SelectionAction::Inspect);
        self.graph_interaction.pending = Some(PendingPress {
            key,
            member,
            deadline: None,
            interval,
            previous,
        });
    }

    pub fn drill_graph_node(&mut self, key: String, member: Option<u32>) {
        let previous = self
            .graph_interaction
            .pending
            .take()
            .map(|pending| pending.previous)
            .unwrap_or_else(|| self.nav.clone());
        self.record_graph_step(previous);
        if member.is_some() {
            self.focus_visible(key, member);
            return;
        }
        let class = self
            .graph_view
            .nodes
            .iter()
            .any(|node| node.key == key && node.group_kind == Some(GroupKind::Class));
        let target = if class {
            FocusTarget::ClassMethods(key)
        } else {
            FocusTarget::GroupChildren(key)
        };
        self.apply_focus(target);
    }

    pub fn focus_graph_method(&mut self, member: u32) {
        self.cancel_graph_press();
        self.select_method(member, SelectionAction::Focus);
    }

    /// File navigation retains filters and preflights before changing history or camera.
    pub(crate) fn reveal_file_classes(&mut self, path: String) -> bool {
        let path = self
            .explorer
            .graph_path(&crate::source_path::identity(&path));
        let (Some(graph), Some(mut request)) = (&self.graph, self.request()) else {
            self.status = "No analyzed classes for this file".into();
            return false;
        };
        request.scope = crate::projection::Scope::Overview;
        if !self.uses_fixture_queries() {
            return self.request_file_reveal(graph.clone(), request, path);
        }
        if !crate::projection::fixture_file_classes_visible(graph, &request, &path) {
            self.status = if graph
                .data
                .nodes
                .iter()
                .any(|node| crate::source_path::identity(&node.file) == path)
            {
                "Current filters hide the classes in this file"
            } else {
                "No analyzed classes for this file"
            }
            .into();
            return false;
        }
        self.finish_file_reveal(path);
        true
    }

    fn request_file_reveal(
        &mut self,
        graph: std::sync::Arc<codecallnavigator::graph::GraphIndex>,
        request: crate::projection::Request,
        path: String,
    ) -> bool {
        let queries = match self.graph_queries() {
            Ok(queries) => queries,
            Err(error) => {
                self.warning = error;
                return false;
            }
        };
        self.jobs
            .file_focus_cancel
            .store(true, std::sync::atomic::Ordering::Release);
        self.jobs.file_focus_cancel =
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancel = self.jobs.file_focus_cancel.clone();
        let epoch = self.epoch;
        let revision = self.revision;
        self.jobs
            .file_focus
            .request(path, move |path| FileFocusOutput {
                epoch,
                revision,
                found: graph
                    .data
                    .nodes
                    .iter()
                    .any(|node| crate::source_path::identity(&node.file) == path),
                result: queries
                    .file_visible(&path, &request, &cancel)
                    .map_err(|error| format!("{error:#}")),
                path,
            });
        true
    }

    fn finish_file_reveal(&mut self, path: String) {
        self.checkpoint();
        self.visual_selection = None;
        self.nav.selected = None;
        self.apply_focus(FocusTarget::FileClasses(path));
    }

    pub fn select_method(&mut self, member: u32, action: SelectionAction) {
        self.cancel_graph_press();
        let Some(identity) = self.method_identity(member) else {
            return;
        };
        self.checkpoint();
        self.apply_visible_action(identity, Some(member), action);
    }

    pub(crate) fn reveal_search_target(&mut self, target: codecallnavigator::search::SearchTarget) {
        use codecallnavigator::search::SearchTarget;
        match target {
            SearchTarget::Method { identity, .. } => {
                if let Some(member) = self.graph.as_ref().and_then(|g| g.node_index(&identity)) {
                    self.reveal(member);
                }
            }
            SearchTarget::Class {
                identity, members, ..
            } => {
                self.checkpoint();
                if let Some(graph) = &self.graph {
                    for &member in members.iter() {
                        if let Some(node) = graph.data.nodes.get(member as usize) {
                            self.nav.filter.include_tests |= node.is_test;
                            self.nav.filter.include_generated |= node.is_generated;
                            self.nav.filter.include_unavailable |= node.body_unavailable;
                        }
                    }
                }
                // Start collapsed so a class result selects its class card from any level.
                self.nav.set_grouping(Grouping::Project);
                let key = crate::projection::class_key(&identity);
                self.cancel_graph_press();
                self.inspect_visible(key.clone(), None);
                self.apply_focus(FocusTarget::Group(key));
            }
        }
    }

    pub fn reveal(&mut self, member: u32) {
        let Some(item) = self
            .graph
            .as_ref()
            .and_then(|g| g.data.nodes.get(member as usize))
        else {
            return;
        };
        let (identity, test, generated, unavailable) = (
            item.id.clone(),
            item.is_test,
            item.is_generated,
            item.body_unavailable,
        );
        self.checkpoint();
        if self.nav.grouping != Grouping::Method {
            self.nav.set_grouping(Grouping::Method);
        }
        self.nav.filter.include_tests |= test;
        self.nav.filter.include_generated |= generated;
        self.nav.filter.include_unavailable |= unavailable;
        self.inspect_visible(identity.clone(), Some(member));
        self.apply_focus(FocusTarget::Method(identity));
    }

    pub(crate) fn record_graph_step(&mut self, previous: Navigation) {
        self.preserve_adoption_camera = false;
        self.back.push(previous);
        if self.back.len() > 100 {
            self.back.remove(0);
        }
        self.forward.clear();
    }

    fn method_identity(&self, member: u32) -> Option<String> {
        Some(
            self.graph
                .as_ref()?
                .data
                .nodes
                .get(member as usize)?
                .id
                .clone(),
        )
    }

    fn inspect_visible(&mut self, key: String, member: Option<u32>) {
        self.explorer.graph_selected();
        self.visual_selection = Some(key);
        self.nav.selected = member.and_then(|member| self.method_identity(member));
        self.neighbor_pages = [0, 0];
        self.relationship_page = 0;
        self.selected_edge = None;
        self.dirty_session = true;
        self.refresh_neighbors();
    }

    fn apply_visible_action(&mut self, key: String, member: Option<u32>, action: SelectionAction) {
        match action {
            SelectionAction::Inspect => self.inspect_visible(key, member),
            SelectionAction::Focus => self.focus_visible(key, member),
        }
    }

    fn focus_visible(&mut self, key: String, member: Option<u32>) {
        self.inspect_visible(key.clone(), member);
        let target = match member.and_then(|member| self.method_identity(member)) {
            Some(identity) => FocusTarget::Method(identity),
            None => FocusTarget::Group(key),
        };
        self.apply_focus(target);
    }

    fn apply_focus(&mut self, target: FocusTarget) {
        self.nav.focus = match &target {
            FocusTarget::Method(identity) => Some(identity.clone()),
            _ => None,
        };
        self.nav.focus_target = Some(target);
        self.nav.show_path = false;
        self.nav.page = 0;
        self.auto_fit = false;
        self.changed();
        self.graph_interaction.center_after_layout = true;
    }

    pub fn reconcile_visual_selection(&mut self) {
        let visible = self
            .visual_selection
            .as_ref()
            .is_some_and(|key| self.graph_view.nodes.iter().any(|node| &node.key == key));
        if visible {
            return;
        }
        self.visual_selection = self.selected().and_then(|selected| {
            self.graph_view
                .nodes
                .iter()
                .find(|node| node.member == Some(selected))
                .map(|node| node.key.clone())
        });
    }
}
