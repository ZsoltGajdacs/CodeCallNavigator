//! Bounded lexical project history; opening and validation stay in workspace jobs.
const LIMIT: usize = 8;

pub(crate) fn remember(projects: &mut Vec<String>, root: &str) {
    if root.trim().is_empty() {
        return;
    }
    let root = crate::source_path::identity(root.trim());
    projects.retain(|existing| crate::source_path::identity(existing) != root);
    projects.insert(0, root);
    projects.truncate(LIMIT);
}

pub(crate) fn restore(projects: Vec<String>, root: &str) -> Vec<String> {
    let mut normalized = Vec::new();
    for project in projects.into_iter().rev() {
        remember(&mut normalized, &project);
    }
    remember(&mut normalized, root);
    normalized
}

impl crate::app::State {
    pub(crate) fn select_recent_project(&mut self, root: String) {
        self.open_project(root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_deduplicates_normalizes_and_bounds_recent_roots() {
        let mut projects = Vec::new();
        for index in 0..12 {
            remember(&mut projects, &format!("/projects/{index}"));
        }
        remember(&mut projects, "/projects/11/../10");
        assert_eq!(projects.len(), LIMIT);
        assert_eq!(projects[0], "/projects/10");
        assert_eq!(projects.iter().filter(|p| *p == "/projects/10").count(), 1);
        let previous = projects.clone();
        remember(&mut projects, "  ");
        assert_eq!(projects, previous);
    }
    #[test]
    fn legacy_restore_adds_current_root_without_reordering_history() {
        assert_eq!(restore(Vec::new(), "/current"), vec!["/current"]);
        assert_eq!(
            restore(vec!["/two".into(), "/one".into()], "/two"),
            vec!["/two", "/one"]
        );
    }
}

#[cfg(test)]
mod session_tests {
    #[test]
    fn recent_projects_round_trip_with_legacy_defaults() {
        let legacy: crate::navigation::SavedSession = serde_json::from_str("{}").unwrap();
        assert!(legacy.recent_projects.is_empty());
        let mut state = crate::app::State::new();
        state.recent_projects = vec!["/project".into()];
        let restored: crate::navigation::SavedSession =
            serde_json::from_value(serde_json::to_value(state.session()).unwrap()).unwrap();
        assert_eq!(restored.recent_projects, state.recent_projects);
    }
}
