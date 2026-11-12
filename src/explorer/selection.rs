//! One-way graph selection resolution; explorer-only clicks never change navigation.
use super::Workspace;
use crate::{
    app::State,
    projection::{FocusTarget, ViewNode},
};
use std::collections::BTreeSet;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct SelectionToken {
    epoch: u64,
    visual: Option<String>,
    member: Option<String>,
    focus: Option<FocusTarget>,
    layout: Option<(u64, u64)>,
}
impl SelectionToken {
    fn of(state: &State) -> Self {
        Self {
            epoch: state.epoch,
            visual: state.visual_selection.clone(),
            member: state.nav.selected.clone(),
            focus: state.nav.focus_target.clone(),
            layout: state.visible_revision,
        }
    }
    fn same_selection(&self, other: &Self) -> bool {
        self.epoch == other.epoch
            && self.visual == other.visual
            && self.member == other.member
            && self.focus == other.focus
    }
}

/// Resolve files from canonical presentation metadata, without changing navigation.
pub fn selected_files(state: &State) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    if let Some(node) = state
        .visual_selection
        .as_ref()
        .and_then(|key| state.graph_view.nodes.iter().find(|node| &node.key == key))
    {
        add_node_files(state, node, &mut paths);
    }
    if !paths.is_empty() {
        return paths;
    }
    add_selected_member(state, &mut paths);
    if !paths.is_empty() {
        return paths;
    }
    add_focused_files(state, &mut paths);
    paths
}
fn add_selected_member(state: &State, paths: &mut BTreeSet<String>) {
    if let Some(member) = state
        .selected()
        .and_then(|member| state.graph.as_ref()?.data.nodes.get(member as usize))
    {
        add_path(&member.file, paths);
    }
}
fn add_focused_files(state: &State, paths: &mut BTreeSet<String>) {
    if let Some(FocusTarget::FileClasses(path)) = &state.nav.focus_target {
        add_path(path, paths);
        return;
    }
    if !state.view_is_current() || state.nav.focus_target.is_none() {
        return;
    }
    for node in &state.graph_view.nodes {
        if state.graph_view.focus_keys.contains(&node.key) {
            add_node_files(state, node, paths);
        }
    }
}
fn add_path(path: &str, paths: &mut BTreeSet<String>) {
    if !path.is_empty() {
        paths.insert(crate::source_path::identity(path));
    }
}
fn add_node_files(state: &State, node: &ViewNode, paths: &mut BTreeSet<String>) {
    for file in &node.files {
        add_path(&file.path, paths);
    }
    if let Some(member) = node
        .member
        .and_then(|member| state.graph.as_ref()?.data.nodes.get(member as usize))
    {
        add_path(&member.file, paths);
    }
}

pub(super) fn sync(state: &mut State) -> bool {
    let token = SelectionToken::of(state);
    if state.explorer.root.is_empty() || state.explorer.selection_token.as_ref() == Some(&token) {
        return false;
    }
    let files = selected_files(state);
    let changed = should_highlight(&state.explorer, &token, &files);
    state.explorer.selection_token = Some(token);
    state.explorer.resolved_files = files.clone();
    if changed {
        state.explorer.highlight(files);
    }
    changed
}
fn should_highlight(
    workspace: &Workspace,
    token: &SelectionToken,
    files: &BTreeSet<String>,
) -> bool {
    workspace
        .selection_token
        .as_ref()
        .is_none_or(|previous| !previous.same_selection(token))
        || &workspace.resolved_files != files
}
