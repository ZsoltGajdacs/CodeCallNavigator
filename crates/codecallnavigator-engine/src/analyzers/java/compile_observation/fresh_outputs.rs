//! A successful compiler journal cannot establish the origin of preexisting outputs.
use super::{JavaBuildMetadata, Observations, Outcome};
use crate::analyzers::CleanSnapshotCheckout;
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeSet,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

pub(in crate::analyzers::java) struct FreshOutputs {
    empty_roots: BTreeSet<PathBuf>,
}

impl FreshOutputs {
    pub fn before(metadata: &JavaBuildMetadata) -> Result<Self> {
        let mut empty_roots = BTreeSet::new();
        for path in metadata
            .output_roots
            .iter()
            .map(PathBuf::as_path)
            .chain(output_roots(metadata).map(|(path, _)| path))
        {
            if !has_entries(path)? {
                require_unlinked(path)?;
                empty_roots.insert(path.to_owned());
            }
        }
        Ok(Self { empty_roots })
    }

    pub fn verify(
        self,
        metadata: &JavaBuildMetadata,
        observed: &Observations,
        checkout: Option<&CleanSnapshotCheckout>,
    ) -> Result<()> {
        for (path, generated) in output_roots(metadata) {
            if !has_entries(path)? {
                continue;
            }
            require_unlinked(path)?;
            // Build plugins can register generated sources after the metadata pass.
            // A clean worktree can prove their origin without a compiler output entry.
            if generated && checkout.is_some_and(|clean| clean.owns_generated_root(path)) {
                continue;
            }
            self.verify_compiler_output(path, generated, observed)?;
        }
        Ok(())
    }

    fn verify_compiler_output(
        &self,
        path: &Path,
        generated: bool,
        observed: &Observations,
    ) -> Result<()> {
        ensure!(
            self.empty_roots.iter().any(|root| path.starts_with(root)),
            "Java outputs predate the observed compiler attempt at {}",
            path.display()
        );
        let found = observed.operations.iter().any(|operation| {
            let roots = if generated {
                &operation.outputs.generated_sources
            } else {
                &operation.outputs.classes
            };
            operation.outcome == Outcome::Success && roots.iter().any(|root| root == path)
        });
        ensure!(
            found,
            "No current Java compiler execution proves the origin of outputs at {}",
            path.display()
        );
        Ok(())
    }
}

fn output_roots(metadata: &JavaBuildMetadata) -> impl Iterator<Item = (&Path, bool)> {
    metadata.source_sets.iter().flat_map(|source| {
        source
            .class_dirs
            .iter()
            .map(|path| (path.as_path(), false))
            .chain(
                source
                    .generated_roots
                    .iter()
                    .map(|path| (path.as_path(), true)),
            )
    })
}

fn has_entries(path: &Path) -> Result<bool> {
    match fs::read_dir(path) {
        Ok(mut entries) => Ok(entries.next().transpose()?.is_some()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error)
            .with_context(|| format!("Cannot inspect Java compiler outputs at {}", path.display())),
    }
}

fn require_unlinked(path: &Path) -> Result<()> {
    ensure!(
        crate::fingerprint::required_link_bindings(path)?.is_empty(),
        "Java compiler output root is linked: {}",
        path.display()
    );
    Ok(())
}
