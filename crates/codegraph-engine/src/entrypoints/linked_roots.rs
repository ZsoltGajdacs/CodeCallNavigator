//! Stable aliases for sources the compiler owns outside the desktop's project.
use anyhow::{Context, Result, ensure};
use codegraph_core::{artifact, model::GraphData};
use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};

pub fn infer(
    graph: &GraphData,
    project: &Path,
    mut roots: Vec<(String, PathBuf)>,
) -> Result<Vec<(String, PathBuf)>> {
    let mut directories = owned_directories(graph)?;
    directories.sort_by_key(|path| (path.components().count(), path.clone()));
    for directory in directories {
        if roots.iter().any(|(_, root)| directory.starts_with(root)) {
            continue;
        }
        let relative = relative_directory(project, &directory)?;
        let hash = blake3::hash(relative.as_bytes()).to_hex();
        roots.push((format!("linked-{}", &hash.as_str()[..12]), directory));
    }
    roots.sort_by(|a, b| a.0.cmp(&b.0));
    artifact::validate_roots(&roots)?;
    Ok(roots)
}

fn owned_directories(graph: &GraphData) -> Result<Vec<PathBuf>> {
    let mut directories = BTreeSet::new();
    for root in &graph.identity_roots {
        let root = Path::new(root);
        ensure!(root.is_absolute(), "Compiler identity root is not absolute");
        directories.insert(root.to_path_buf());
    }
    for node in &graph.nodes {
        owned_paths(&node.file, &node.build, &mut directories)?;
    }
    for file in graph.architecture.iter().flat_map(|facts| &facts.files) {
        owned_paths(&file.file, &file.build, &mut directories)?;
    }
    for function in graph
        .flow_inventory
        .iter()
        .flat_map(|inventory| &inventory.functions)
    {
        owned_paths(&function.file, &function.build, &mut directories)?;
    }
    if let Some(inventory) = &graph.boundaries {
        for route in &inventory.routes {
            owned_paths(&route.site.file, &route.build, &mut directories)?;
        }
        for usage in &inventory.uses {
            owned_paths(&usage.site.file, &usage.build, &mut directories)?;
        }
    }
    Ok(directories.into_iter().collect())
}

fn owned_paths(file: &str, build: &str, directories: &mut BTreeSet<PathBuf>) -> Result<()> {
    if !file.is_empty() {
        directories.insert(parent(Path::new(file))?);
    }
    if !build.is_empty() {
        let build = Path::new(build);
        ensure!(
            build.is_absolute(),
            "Compiler-owned build path is not absolute"
        );
        directories.insert(if build.is_dir() {
            build.into()
        } else {
            parent(build)?
        });
    }
    Ok(())
}

fn parent(path: &Path) -> Result<PathBuf> {
    ensure!(
        path.is_absolute(),
        "Compiler-owned source path is not absolute"
    );
    Ok(path
        .parent()
        .context("Compiler-owned path has no parent")?
        .into())
}

fn relative_directory(project: &Path, directory: &Path) -> Result<String> {
    let from: Vec<_> = project.components().collect();
    let to: Vec<_> = directory.components().collect();
    let shared = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    ensure!(
        shared > 0,
        "Linked source root has no common filesystem root"
    );
    let mut relative = PathBuf::new();
    for component in &from[shared..] {
        ensure!(
            matches!(component, Component::Normal(_)),
            "Project root is not normalized"
        );
        relative.push("..");
    }
    for component in &to[shared..] {
        relative.push(component.as_os_str());
    }
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use codegraph_core::model::Node;

    fn graph(root: &str) -> GraphData {
        GraphData {
            nodes: vec![Node {
                file: format!("{root}/shared/Shared.cs"),
                build: format!("{root}/app/App.csproj"),
                ..Node::default()
            }],
            ..GraphData::default()
        }
    }

    #[test]
    fn compiler_identity_context_roots_cover_external_java_modules() {
        let mut value = serde_json::to_value(GraphData::default()).unwrap();
        value["identity_roots"] = serde_json::json!(["/first/library"]);
        let graph: GraphData = serde_json::from_value(value).unwrap();
        let roots = infer(
            &graph,
            Path::new("/first/app"),
            vec![("repo".into(), "/first/app".into())],
        )
        .unwrap();
        assert!(
            roots
                .iter()
                .any(|(_, root)| root == Path::new("/first/library"))
        );
    }

    #[test]
    fn compiler_owned_boundary_blockers_without_symbols_get_portable_roots() {
        let graph = GraphData {
            boundaries: Some(codegraph_core::boundary::BoundaryInventory {
                version: 1,
                uses: vec![],
                routes: vec![codegraph_core::boundary::RouteFact {
                    symbol: None,
                    build: "/first/app/tsconfig.json".into(),
                    application: "app".into(),
                    framework: "next".into(),
                    kind: "navigation".into(),
                    path: None,
                    methods: vec![],
                    site: codegraph_core::model::SourceSite {
                        file: "/first/shared/Blocked.ts".into(),
                        line: 1,
                        column: None,
                        span: None,
                    },
                    restrictions: vec!["unsupported".into()],
                    order: None,
                }],
            }),
            ..Default::default()
        };
        let roots = infer(
            &graph,
            Path::new("/first/app"),
            vec![("repo".into(), "/first/app".into())],
        )
        .unwrap();
        assert!(
            roots
                .iter()
                .any(|(_, path)| path == Path::new("/first/shared"))
        );
        codegraph_core::artifact::canonical(&graph, &roots).unwrap();
    }
    #[test]
    fn equivalent_sibling_layouts_have_stable_aliases_and_keep_explicit_roots() {
        let first = infer(
            &graph("/first"),
            Path::new("/first/app"),
            vec![("repo".into(), "/first/app".into())],
        )
        .unwrap();
        let second = infer(
            &graph("/second"),
            Path::new("/second/app"),
            vec![("repo".into(), "/second/app".into())],
        )
        .unwrap();
        assert_eq!(first[0].0, second[0].0);
        assert!(first[0].0.starts_with("linked-"));
        let explicit = vec![
            ("repo".into(), "/first/app".into()),
            ("shared".into(), "/first/shared".into()),
        ];
        assert_eq!(
            infer(&graph("/first"), Path::new("/first/app"), explicit)
                .unwrap()
                .len(),
            2
        );
    }
}
