//! Local input observation for a selected analysis session, before and after publication.
use anyhow::{Context, Result, ensure};
use codecallnavigator_core::{
    artifact::{self, provenance, storage::Captured},
    discovery::BuildTarget,
    fingerprint::{self, ContentObservation, ScanRoot},
    model::FileFingerprint,
};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

pub(super) struct Inputs(Vec<FileFingerprint>);

impl Inputs {
    pub(super) fn saved(
        project: &Path,
        roots: &artifact::Roots,
        expected: Option<&str>,
        targets: &[BuildTarget],
    ) -> Result<(Self, String)> {
        let captured = Captured::open(project)?;
        let digest = blake3::hash(captured.manifest_bytes()).to_hex().to_string();
        ensure!(
            expected.is_none_or(|expected| expected == digest),
            "Saved graph changed before the analysis session opened"
        );
        let saved = captured.structure()?;
        let analysis =
            provenance::read(&saved.graph)?.context("Saved graph omitted input provenance")?;
        ensure!(
            analysis.observation_policy == fingerprint::InputPolicy::ScopedV1,
            "Saved graph uses unsupported input observation rules"
        );
        let selected = artifact::selection::resolve(analysis.selection.as_ref(), roots)?;
        ensure!(
            selection_keys(&selected) == selection_keys(targets),
            "Saved graph build selection differs from the requested session"
        );
        Ok((
            Self::captured(provenance::local_inputs(&analysis, roots)?),
            digest,
        ))
    }

    pub(super) fn bootstrap(project: &Path, targets: &[BuildTarget]) -> Result<Self> {
        let mut paths = BTreeSet::from([
            project.join(".codecallnavigator/layers.yml"),
            project.join(".interactive-review/layers.yml"),
        ]);
        for target in targets {
            let typescript = target.language == "typescript";
            let mut roots = vec![ScanRoot::Required(target.path.clone())];
            if let Some(parent) = target.path.parent()
                && target.language != "java"
            {
                roots.push(ScanRoot::Required(parent.into()));
            }
            let files = fingerprint::collect_inputs(&roots, &[], !typescript)?;
            paths.extend(
                files
                    .into_iter()
                    .filter(|path| !typescript || path == &target.path || typescript_input(path)),
            );
        }
        Ok(Self::captured(fingerprint::capture_excluding(paths, &[])?))
    }

    pub(super) fn captured(mut inputs: Vec<FileFingerprint>) -> Self {
        inputs
            .sort_by(|a, b| (&a.path, &a.excluded_children).cmp(&(&b.path, &b.excluded_children)));
        inputs.dedup_by(|a, b| {
            a.path == b.path
                && a.excluded_children == b.excluded_children
                && a.kind == b.kind
                && a.hash == b.hash
        });
        Self(inputs)
    }

    pub(super) fn expected(&self) -> String {
        let mut digest = blake3::Hasher::new();
        for item in &self.0 {
            hash(&mut digest, item, item.kind, &item.hash);
        }
        digest.finalize().to_hex().to_string()
    }

    pub(super) fn current(&self, cancel: &AtomicBool) -> Result<String> {
        let mut digest = blake3::Hasher::new();
        for item in &self.0 {
            let ContentObservation { kind, hash: value } =
                fingerprint::reobserve(item, Some(cancel))?;
            hash(&mut digest, item, kind, &value);
        }
        Ok(digest.finalize().to_hex().to_string())
    }

    /// Expand scheduling-only observation to files added since the last graph.
    /// Rebuild candidates from the current selection instead of accumulating them.
    pub(super) fn with_candidates(&self, discovered: Self) -> Self {
        let mut known = self
            .0
            .iter()
            .map(|input| input.path.as_str())
            .collect::<BTreeSet<_>>();
        let mut inputs = self.0.clone();
        for input in &discovered.0 {
            if known.insert(&input.path) {
                inputs.push(input.clone());
            }
        }
        Self::captured(inputs)
    }
}

fn selection_keys(targets: &[BuildTarget]) -> BTreeSet<(PathBuf, String, String)> {
    targets
        .iter()
        .map(|target| {
            (
                target.path.clone(),
                target.language.clone(),
                target.kind.clone(),
            )
        })
        .collect()
}

fn typescript_input(path: &Path) -> bool {
    path.is_dir()
        || path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| {
                matches!(
                    ext,
                    "ts" | "tsx" | "js" | "jsx" | "mts" | "cts" | "mjs" | "cjs" | "json"
                )
            })
        || path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                (name.starts_with("tsconfig") && name.ends_with(".json"))
                    || matches!(
                        name,
                        "package.json" | "package-lock.json" | "pnpm-lock.yaml" | "yarn.lock"
                    )
            })
}

fn hash(
    digest: &mut blake3::Hasher,
    item: &FileFingerprint,
    kind: fingerprint::InputKind,
    content: &str,
) {
    fn part(digest: &mut blake3::Hasher, bytes: &[u8]) {
        digest.update(&(bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
    }
    part(digest, item.path.as_bytes());
    digest.update(&[match kind {
        fingerprint::InputKind::Missing => 0,
        fingerprint::InputKind::File => 1,
        fingerprint::InputKind::Directory => 2,
    }]);
    part(digest, content.as_bytes());
}
