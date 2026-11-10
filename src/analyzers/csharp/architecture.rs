//! Roslyn-owned file facts; graph inclusion and architecture classification remain separate.
use super::SourceOwners;
use crate::{
    architecture::{ArchitectureFacts, FileFacts},
    discovery::BuildTarget,
};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AtlasFileFacts {
    assembly: String,
    file: String,
    content: String,
    namespaces: Vec<String>,
    names: Vec<String>,
    kinds: Vec<String>,
    annotations: Vec<String>,
    method_annotations: Vec<String>,
    base_types: Vec<String>,
    annotated_interfaces: Vec<String>,
    is_interface: bool,
}

pub(super) fn normalize(
    build: &BuildTarget,
    root: &Path,
    files: Option<Vec<AtlasFileFacts>>,
    owners: &SourceOwners,
) -> Option<ArchitectureFacts> {
    let files = files?;
    let mut facts = Vec::new();
    for file in files {
        let path = root.join(&file.file);
        let path = path.canonicalize().unwrap_or(path);
        let Some(owner) = owners
            .get(&path)
            .and_then(|owners| owners.get(&file.assembly))
        else {
            continue;
        };
        facts.push(FileFacts {
            build: build.path.to_string_lossy().into_owned(),
            project: owner.project.clone(),
            file: path.to_string_lossy().into_owned(),
            language: "csharp".into(),
            namespaces: file.namespaces,
            names: file.names,
            kinds: file.kinds,
            annotations: file.annotations,
            method_annotations: file.method_annotations,
            base_types: file.base_types,
            annotated_interfaces: file.annotated_interfaces,
            is_interface: file.is_interface,
            content: file.content,
            is_test: owner.is_test,
        });
    }
    facts.sort_by(|a, b| (&a.project, &a.file).cmp(&(&b.project, &b.file)));
    facts.dedup();
    Some(ArchitectureFacts {
        version: 1,
        files: facts,
    })
}
