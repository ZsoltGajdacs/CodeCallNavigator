//! Merge compiler file inventories without changing symbol or relationship identities.
use crate::architecture::{ArchitectureFacts, FACTS_VERSION};
use anyhow::{Result, ensure};
use std::collections::BTreeMap;

pub(super) fn empty() -> Option<ArchitectureFacts> {
    Some(ArchitectureFacts {
        version: FACTS_VERSION,
        files: Vec::new(),
    })
}

pub(super) fn merge(
    target: &mut Option<ArchitectureFacts>,
    part: Option<ArchitectureFacts>,
) -> Result<()> {
    let Some(current) = target else {
        return Ok(());
    };
    let Some(part) = part else {
        *target = None;
        return Ok(());
    };
    ensure!(
        current.version == FACTS_VERSION && part.version == FACTS_VERSION,
        "Unsupported architecture declaration facts version"
    );
    let mut files = BTreeMap::new();
    for file in current.files.drain(..).chain(part.files) {
        let key = (
            file.build.clone(),
            file.project.clone(),
            file.language.clone(),
            file.file.clone(),
        );
        if let Some(previous) = files.insert(key, file.clone()) {
            ensure!(
                previous == file,
                "Architecture source facts changed between compile contexts: {}",
                file.file
            );
        }
    }
    current.files = files.into_values().collect();
    Ok(())
}
