//! Stable grouping identities, independent of aggregation and geometry.
use super::{GroupKind, Grouping, Request};
use codegraph::model::Node;
use std::path::{Path, PathBuf};

pub(super) struct Group {
    pub key: String,
    pub label: String,
    pub kind: GroupKind,
    pub folder_depth: usize,
}

impl Group {
    fn new(parts: &[&str], label: String, kind: GroupKind) -> Self {
        Self {
            key: key(parts),
            label,
            kind,
            folder_depth: 0,
        }
    }
}

fn key(parts: &[&str]) -> String {
    serde_json::to_string(parts).expect("string arrays serialize")
}

pub(super) fn relative_file(node: &Node) -> PathBuf {
    let build = Path::new(&node.build);
    let file_target = build
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "sln" | "slnx" | "csproj" | "json"
            )
        });
    let root = if file_target && !node.language.eq_ignore_ascii_case("java") {
        build.parent().unwrap_or(build)
    } else {
        build
    };
    let file = Path::new(&node.file);
    file.strip_prefix(root).unwrap_or(file).to_path_buf()
}

pub(super) fn groups(node: &Node, request: &Request) -> Vec<Group> {
    let scope = [&*node.build, &*node.project];
    let mut result = vec![Group::new(
        &["project", scope[0], scope[1]],
        node.project.clone(),
        GroupKind::Project,
    )];
    intermediate_groups(node, request.grouping, &mut result);
    result.push(Group::new(
        &["class", scope[0], scope[1], &node.container],
        node.container.clone(),
        GroupKind::Class,
    ));
    if request.grouping == Grouping::Layer {
        let layer = super::layers::assignment(node, request).unwrap_or("Other");
        for group in &mut result {
            group.key = key(&["layer-member", layer, &group.key]);
        }
        result.insert(
            0,
            Group::new(&["layer", layer], layer.to_owned(), GroupKind::Layer),
        );
    }
    result
}

pub(super) fn folder_depth(node: &Node) -> usize {
    relative_file(node)
        .parent()
        .map_or(0, |path| path.components().count())
}

pub(super) fn find(node: &Node, request: &Request, key: &str) -> Option<Group> {
    groups(node, request)
        .into_iter()
        .find(|group| group.key == key)
        .or_else(|| {
            // Root folders own descendants when focused, without adding an extra
            // root level to the ordinary overview hierarchy.
            let root = root_folder(node);
            (request.grouping == Grouping::Folder && root.key == key).then_some(root)
        })
}

pub(super) fn folder_at_depth(node: &Node, request: &Request, depth: usize) -> Group {
    if depth == 0 {
        return root_folder(node);
    }
    let mut folders = groups(node, request)
        .into_iter()
        .filter(|group| group.kind == GroupKind::Folder);
    let mut containing = folders
        .next()
        .expect("folder hierarchy includes project root files");
    for group in folders {
        if group.folder_depth > depth {
            break;
        }
        containing = group;
    }
    containing
}

fn intermediate_groups(node: &Node, grouping: Grouping, result: &mut Vec<Group>) {
    let scope = [&*node.build, &*node.project];
    match grouping {
        Grouping::Namespace => result.push(Group::new(
            &["namespace", scope[0], scope[1], &node.namespace],
            if node.namespace.is_empty() {
                "(global namespace)".into()
            } else {
                node.namespace.clone()
            },
            GroupKind::Namespace,
        )),
        Grouping::Folder => folder_groups(node, result),
        Grouping::Layer | Grouping::Project | Grouping::Method => {}
    }
}

fn folder_groups(node: &Node, result: &mut Vec<Group>) {
    let file = relative_file(node);
    let mut folder = PathBuf::new();
    let before = result.len();
    for (index, part) in file
        .parent()
        .into_iter()
        .flat_map(Path::components)
        .enumerate()
    {
        folder.push(part);
        let name = folder.to_string_lossy();
        let mut group = Group::new(
            &["folder", &node.build, &node.project, &name],
            name.to_string(),
            GroupKind::Folder,
        );
        group.folder_depth = index + 1;
        result.push(group);
    }
    if result.len() == before {
        result.push(root_folder(node));
    }
}

fn root_folder(node: &Node) -> Group {
    Group::new(
        &["folder", &node.build, &node.project, ""],
        "(project root)".into(),
        GroupKind::Folder,
    )
}
