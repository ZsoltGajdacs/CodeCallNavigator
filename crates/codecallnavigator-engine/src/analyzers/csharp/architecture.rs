//! Roslyn-owned file facts; graph inclusion and architecture classification remain separate.
use super::{SourceOwners, source};
use crate::{
    architecture::{ArchitectureFacts, FileFacts},
    discovery::BuildTarget,
};
use anyhow::{Result, ensure};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AtlasFileFacts {
    assembly: String,
    file: String,
    content: String,
    #[serde(default)]
    source_code_page: Option<u32>,
    namespaces: Vec<String>,
    names: Vec<String>,
    kinds: Vec<String>,
    annotations: Vec<String>,
    method_annotations: Vec<String>,
    base_types: Vec<String>,
    annotated_interfaces: Vec<String>,
    is_interface: bool,
}

pub(super) struct Context<'a> {
    pub build: &'a BuildTarget,
    pub root: &'a Path,
    pub owners: &'a SourceOwners,
    pub snapshots: &'a mut source::Snapshots,
}

pub(super) fn normalize(
    files: Option<Vec<AtlasFileFacts>>,
    context: Context<'_>,
) -> Result<Option<ArchitectureFacts>> {
    let Some(files) = files else {
        return Ok(None);
    };
    let mut facts = Vec::new();
    for file in files {
        ensure!(
            file.source_code_page.is_none_or(|page| page > 0),
            "C# source facts contain an invalid compiler code page"
        );
        let path = source::path(context.root, &file.file);
        let Some(owner) = context
            .owners
            .get(&path)
            .and_then(|owners| owners.get(&file.assembly))
        else {
            continue;
        };
        facts.push(FileFacts {
            build: context.build.path.to_string_lossy().into_owned(),
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
            content: context.snapshots.content(
                source::CompilerContext {
                    file: &path,
                    assembly: &file.assembly,
                    code_page: file.source_code_page,
                },
                &file.content,
            )?,
            is_test: owner.is_test,
        });
    }
    facts.sort_by(|a, b| (&a.project, &a.file).cmp(&(&b.project, &b.file)));
    facts.dedup();
    Ok(Some(ArchitectureFacts {
        version: 1,
        files: facts,
    }))
}
