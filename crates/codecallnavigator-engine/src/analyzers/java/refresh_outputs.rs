//! Compiler ownership is narrower than build output roots: sources stay inputs.
use super::build::JavaBuildMetadata;
use anyhow::{Context, Result};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

pub(super) fn ownership(metadata: &JavaBuildMetadata) -> Result<crate::model::CompilerOutputs> {
    let raw = metadata
        .source_sets
        .iter()
        .map(|source| source.raw_class_dirs.as_ref())
        .collect::<Option<Vec<_>>>();
    Ok(crate::model::CompilerOutputs {
        raw_bytecode_directories: raw.map(|sets| {
            sets.into_iter()
                .flatten()
                .map(|path| path.to_string_lossy().into_owned())
                .collect()
        }),
        compile_classpath: aliases(metadata.source_sets.iter().flat_map(|source| {
            source
                .compile_classpath
                .iter()
                .chain(&source.classpath_aliases)
        }))?,
        bytecode_directories: aliases(
            metadata
                .source_sets
                .iter()
                .flat_map(|source| &source.class_dirs),
        )?,
        source_roots: aliases(
            metadata
                .source_sets
                .iter()
                .flat_map(|source| source.source_roots.iter().chain(&source.generated_roots)),
        )?,
        build_inputs: aliases(
            metadata.build_inputs.iter().chain(
                metadata
                    .source_sets
                    .iter()
                    .flat_map(|source| &source.build_inputs),
            ),
        )?,
    })
}

fn aliases<'a>(paths: impl Iterator<Item = &'a PathBuf>) -> Result<Vec<String>> {
    let mut values = BTreeSet::new();
    for path in paths {
        values.insert(path.to_string_lossy().into_owned());
        values.insert(resolved(path)?.to_string_lossy().into_owned());
    }
    Ok(values.into_iter().collect())
}

#[cfg(test)]
fn overlaps(a: &Path, b: &Path) -> bool {
    a.starts_with(b) || b.starts_with(a)
}

// Resolve existing ancestors too: absent source roots below a directory symlink
// must not bypass the conservative overlap rule.
fn resolved(path: &Path) -> Result<PathBuf> {
    let mut ancestor = path;
    let mut suffix = Vec::new();
    while !ancestor.exists() {
        suffix.push(
            ancestor
                .file_name()
                .context("Build path has no existing ancestor")?,
        );
        ancestor = ancestor
            .parent()
            .context("Build path escapes its filesystem")?;
    }
    let mut resolved = ancestor.canonicalize()?;
    for part in suffix.into_iter().rev() {
        resolved.push(part);
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn symlink_aliases_and_missing_source_roots_cannot_hide_sources() {
        let root = tempfile::tempdir().unwrap();
        let sources = root.path().join("sources");
        std::fs::create_dir(&sources).unwrap();
        let alias = root.path().join("alias");
        std::os::unix::fs::symlink(&sources, &alias).unwrap();
        assert!(overlaps(
            &resolved(&alias).unwrap(),
            &resolved(&sources.join("later/java")).unwrap()
        ));
        assert!(overlaps(
            &resolved(root.path()).unwrap(),
            &resolved(&sources).unwrap()
        ));
        assert!(!overlaps(
            &resolved(&root.path().join("classes")).unwrap(),
            &resolved(&sources).unwrap()
        ));
    }
}
