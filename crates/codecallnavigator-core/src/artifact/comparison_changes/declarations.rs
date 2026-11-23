use super::{
    class::Class,
    methods,
    snapshot::{FileId, Snapshot},
    source,
};
use crate::architecture::FileFacts;
use std::collections::BTreeSet;

pub(super) fn changed_files<'a>(
    before: &Snapshot<'a>,
    after: &Snapshot<'a>,
    changed_methods: &BTreeSet<&str>,
) -> Vec<FileId<'a>> {
    let files: BTreeSet<_> = before
        .files
        .keys()
        .chain(after.files.keys())
        .copied()
        .collect();
    files
        .into_iter()
        .filter(|&file| {
            facts_changed(before.file(file), after.file(file))
                || remaining_source_change(before, after, file, changed_methods)
        })
        .collect()
}

pub(super) fn owner(
    before: &Snapshot<'_>,
    after: &Snapshot<'_>,
    file: FileId<'_>,
) -> Option<Class> {
    let old = before.single_class(file);
    let new = after.single_class(file);
    match (old, new) {
        (Some(old), Some(new)) if old.can_confirm(&new) => Some(new),
        (Some(old), Some(new)) if new.can_confirm(&old) => Some(old),
        (Some(class), None) if after.file(file).is_none() => Some(class),
        (None, Some(class)) if before.file(file).is_none() => Some(class),
        _ => None,
    }
}

fn remaining_source_change(
    before: &Snapshot<'_>,
    after: &Snapshot<'_>,
    file: FileId<'_>,
    changed_methods: &BTreeSet<&str>,
) -> bool {
    let Some((old, new)) = before.text(file).zip(after.text(file)) else {
        return false;
    };
    if old == new {
        return false;
    }
    if !source::fits(old, new) {
        return true;
    }
    let patch = source::patch(old, new);
    [(before, true), (after, false)]
        .into_iter()
        .any(|(snapshot, old_side)| {
            let covered = methods::covered_lines(snapshot, file, changed_methods);
            source::changed_lines(&patch, old_side)
                .any(|line| !covered.iter().any(|range| range.contains(&line)))
        })
}

fn facts_changed(before: Option<&FileFacts>, after: Option<&FileFacts>) -> bool {
    match (before, after) {
        (Some(old), Some(new)) => {
            old.names != new.names
                || old.kinds != new.kinds
                || old.annotations != new.annotations
                || old.base_types != new.base_types
                || old.annotated_interfaces != new.annotated_interfaces
                || old.is_interface != new.is_interface
        }
        (None, None) => false,
        _ => true,
    }
}
