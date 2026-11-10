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
