//! Presentation identities prepared with a snapshot, keeping adoption off graph scans.
use super::{FocusTarget, Grouping, Request, Scope, groups};
use codecallnavigator::graph::GraphIndex;
use std::collections::HashSet;

pub(crate) struct NavigationInventory {
    files: HashSet<String>,
    ordinary: HashSet<String>,
    namespaces: HashSet<String>,
    folders: HashSet<String>,
}

impl NavigationInventory {
    pub fn new(graph: &GraphIndex) -> Self {
        let mut inventory = Self {
            files: HashSet::new(),
            ordinary: HashSet::new(),
            namespaces: HashSet::new(),
            folders: HashSet::new(),
        };
        let mut request = Request {
            focus_target: None,
            grouping: Grouping::Project,
            expanded: HashSet::new(),
            scope: Scope::Overview,
            direction: Default::default(),
            depth: 1,
            filter: Default::default(),
            page: 0,
            architecture: None,
            layer_tab: None,
        };
        for node in &graph.data.nodes {
            inventory
                .files
                .insert(crate::source_path::identity(&node.file));
            request.grouping = Grouping::Project;
            inventory.ordinary.extend(
                groups::groups(node, &request)
                    .into_iter()
                    .map(|group| group.key),
            );
            request.grouping = Grouping::Namespace;
            inventory.namespaces.extend(
                groups::groups(node, &request)
                    .into_iter()
                    .map(|group| group.key),
            );
            request.grouping = Grouping::Folder;
            inventory.folders.extend(
                groups::groups(node, &request)
                    .into_iter()
                    .map(|group| group.key),
            );
            inventory
                .folders
                .insert(groups::folder_at_depth(node, &request, 0).key);
        }
        inventory
    }

    pub fn group(&self, grouping: Grouping, key: &str) -> bool {
        match grouping {
            // Architecture reload owns validation once its assignments arrive.
            Grouping::Layer => true,
            Grouping::Namespace => self.namespaces.contains(key),
            Grouping::Folder => self.folders.contains(key),
            Grouping::Project | Grouping::Method => self.ordinary.contains(key),
        }
    }

    pub fn focus(&self, graph: &GraphIndex, grouping: Grouping, target: &FocusTarget) -> bool {
        match target {
            FocusTarget::Method(identity) => graph.node_index(identity).is_some(),
            FocusTarget::FileClasses(path) => self.files.contains(path),
            FocusTarget::Group(key)
            | FocusTarget::GroupChildren(key)
            | FocusTarget::ClassMethods(key) => self.group(grouping, key),
        }
    }
}

pub(crate) struct LayerInventory {
    groups: HashSet<String>,
    tabs: HashSet<Option<String>>,
}

impl LayerInventory {
    pub fn new(
        graph: &GraphIndex,
        assignments: std::sync::Arc<codecallnavigator::architecture::LayerAssignments>,
    ) -> Self {
        let tabs = assignments
            .layers
            .iter()
            .map(|layer| layer.tab.clone())
            .collect();
        let request = Request {
            focus_target: None,
            grouping: Grouping::Layer,
            expanded: HashSet::new(),
            scope: Scope::Overview,
            direction: Default::default(),
            depth: 1,
            filter: Default::default(),
            page: 0,
            architecture: Some(assignments),
            layer_tab: None,
        };
        let groups = graph
            .data
            .nodes
            .iter()
            .flat_map(|node| {
                groups::groups(node, &request)
                    .into_iter()
                    .map(|group| group.key)
            })
            .collect();
        Self { groups, tabs }
    }

    pub fn group(&self, key: &str) -> bool {
        self.groups.contains(key)
    }
    pub fn tab(&self, tab: &Option<String>) -> bool {
        tab.is_none() || self.tabs.contains(tab)
    }
}
