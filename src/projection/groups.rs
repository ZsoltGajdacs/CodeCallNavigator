//! Stable grouping identities, independent of aggregation and geometry.
use super::{GroupKind, Grouping, Request};
use codegraph::model::Node;
use std::path::{Path, PathBuf};

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

pub(super) fn groups(node: &Node, request: &Request) -> Vec<(String, String, GroupKind)> {
    let scope = [&*node.build, &*node.project];
    let mut result = vec![(
        key(&["project", scope[0], scope[1]]),
        node.project.clone(),
        GroupKind::Project,
    )];
    intermediate_groups(node, request.grouping, &mut result);
    result.push((
        key(&["class", scope[0], scope[1], &node.container]),
        node.container.clone(),
        GroupKind::Class,
    ));
    if request.grouping == Grouping::Layer {
        let layer = super::layers::assignment(node, request).unwrap_or("Other");
        for (identity, _, _) in &mut result {
            *identity = key(&["layer-member", layer, identity]);
        }
        result.insert(
            0,
            (key(&["layer", layer]), layer.to_owned(), GroupKind::Layer),
        );
    }
    result
}

fn intermediate_groups(
    node: &Node,
    grouping: Grouping,
    result: &mut Vec<(String, String, GroupKind)>,
) {
    let scope = [&*node.build, &*node.project];
    match grouping {
        Grouping::Namespace => result.push((
            key(&["namespace", scope[0], scope[1], &node.namespace]),
            if node.namespace.is_empty() {
                "(global namespace)".into()
            } else {
                node.namespace.clone()
            },
            GroupKind::Namespace,
        )),
        Grouping::Folder => {
            let file = relative_file(node);
            let mut folder = PathBuf::new();
            for part in file.parent().into_iter().flat_map(Path::components) {
                folder.push(part);
                let name = folder.to_string_lossy();
                result.push((
                    key(&["folder", scope[0], scope[1], &name]),
                    name.into_owned(),
                    GroupKind::Folder,
                ));
            }
        }
        Grouping::Layer | Grouping::Project | Grouping::Method => {}
    }
}
