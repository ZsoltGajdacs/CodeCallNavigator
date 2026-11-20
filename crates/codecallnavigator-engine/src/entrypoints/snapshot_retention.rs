//! Attribute the exact analysis observations to an immutable Git revision.
use anyhow::{Context, Result, ensure};
use codecallnavigator_core::{
    artifact::{
        Roots,
        local_snapshots::SnapshotIdentity,
        provenance::Analysis,
        revision::{Repository, TreeEntry},
    },
    discovery::BuildTarget,
    fingerprint::{self, InputKind, InputPolicy},
    model::FileFingerprint,
};
use codecallnavigator_engine::analyzers::evidence::{EvidenceInput, EvidenceRole, TargetEvidence};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

pub(super) struct Candidate {
    repository: Repository,
    commit: String,
}

pub(super) struct Observations<'a> {
    pub analysis: &'a Analysis,
    pub roots: &'a Roots,
    pub targets: &'a [BuildTarget],
    pub project_inputs: &'a [FileFingerprint],
    pub evidence: &'a [TargetEvidence],
}

impl Observations<'_> {
    pub fn inputs(
        &self,
    ) -> impl Iterator<Item = (&FileFingerprint, EvidenceRole, Option<&BuildTarget>)> {
        self.project_inputs
            .iter()
            .map(|input| (input, EvidenceRole::Revision, None))
            .chain(self.evidence.iter().flat_map(|target| {
                target
                    .inputs
                    .iter()
                    .filter_map(|EvidenceInput { fingerprint, role }| {
                        (*role != EvidenceRole::Watch).then_some((
                            fingerprint,
                            *role,
                            Some(&target.target),
                        ))
                    })
            }))
    }

    pub fn dependency_identity(&self, root: &Path) -> Result<String> {
        let mut entries = BTreeSet::new();
        for (input, role, _) in self.inputs() {
            if matches!(role, EvidenceRole::Generated | EvidenceRole::Dependency)
                && input.kind != InputKind::Missing
            {
                let path = Path::new(&input.path);
                entries.insert((path.strip_prefix(root).unwrap_or(path), &input.hash));
            }
        }
        Ok(blake3::hash(&serde_json::to_vec(&entries)?)
            .to_hex()
            .to_string())
    }

    fn require_attribution(&self) -> Result<()> {
        let selected = self
            .targets
            .iter()
            .map(BuildTarget::identity)
            .collect::<BTreeSet<_>>();
        let observed = self
            .evidence
            .iter()
            .map(|entry| entry.target.identity())
            .collect::<BTreeSet<_>>();
        ensure!(
            selected == observed && self.evidence.len() == selected.len(),
            "Analyzed build selection has incomplete attribution evidence"
        );
        for evidence in self.evidence {
            if let Some(limit) = &evidence.attribution_limit {
                anyhow::bail!("{}: {limit}", evidence.target.label);
            }
            ensure!(
                evidence
                    .inputs
                    .iter()
                    .any(|input| input.role != EvidenceRole::Watch),
                "{}: adapter reported no consumed inputs",
                evidence.target.label
            );
        }
        Ok(())
    }
}

pub(super) struct VerifiedSnapshot {
    candidate: Candidate,
    dependency_identity: String,
}

impl Candidate {
    pub(super) fn capture(project: &Path) -> Result<Self> {
        let repository = Repository::detect(project)?
            .context("The analysis project is outside a Git repository")?;
        let commit = repository.resolve("HEAD")?;
        Ok(Self { repository, commit })
    }

    pub(super) fn verify(
        self,
        observed: &Observations<'_>,
        cancel: &AtomicBool,
    ) -> Result<VerifiedSnapshot> {
        let analysis = observed.analysis;
        ensure!(
            analysis
                .limitations
                .iter()
                .all(|limit| limit.code == "unmapped_input")
                && analysis.observation_policy == InputPolicy::ScopedV1,
            "Analysis inputs cannot be attributed to this revision"
        );
        ensure!(
            analysis
                .selection
                .as_ref()
                .and_then(|selection| selection["complete"].as_bool())
                == Some(true),
            "Analyzed build selection cannot be fully attributed to this revision"
        );
        observed.require_attribution()?;
        let tree = self.repository.tree(&self.commit, cancel)?;
        let mut proof = InputProof {
            root: &self.repository.root,
            directories: directories(&tree),
            tree,
            hashes: BTreeMap::new(),
        };
        for (input, role, target) in observed.inputs() {
            ensure!(
                !cancel.load(Ordering::Acquire),
                "Snapshot retention cancelled"
            );
            proof.check(input, role).with_context(|| match target {
                Some(target) => format!("{}: Git input attribution failed", target.label),
                None => "Project configuration attribution failed".into(),
            })?;
        }
        self.repository.verify_blobs(&proof.hashes, cancel)?;
        let dependency_identity = observed.dependency_identity(&self.repository.root)?;
        self.require_current()?;
        Ok(VerifiedSnapshot {
            candidate: self,
            dependency_identity,
        })
    }

    fn require_current(&self) -> Result<()> {
        ensure!(
            self.repository.resolve("HEAD")? == self.commit,
            "HEAD changed during analysis"
        );
        Ok(())
    }
}

impl VerifiedSnapshot {
    pub(super) fn require_current(&self) -> Result<()> {
        self.candidate.require_current()
    }

    pub(super) fn dependency_identity(&self) -> &str {
        &self.dependency_identity
    }

    pub(super) fn common_dir(&self) -> &Path {
        &self.candidate.repository.common_dir
    }

    pub(super) fn identity(&self, metadata: &serde_json::Value) -> Result<SnapshotIdentity> {
        let selection = serde_json::to_string(metadata)?;
        Ok(SnapshotIdentity {
            commit: self.candidate.commit.clone(),
            project: self.candidate.repository.project.clone(),
            analyzer: format!(
                "{}:{}",
                env!("CARGO_PKG_VERSION"),
                blake3::hash(selection.as_bytes())
            ),
            selection,
        })
    }
}

struct InputProof<'a> {
    root: &'a Path,
    directories: BTreeMap<PathBuf, BTreeSet<String>>,
    tree: BTreeMap<String, TreeEntry>,
    hashes: BTreeMap<String, String>,
}

impl InputProof<'_> {
    fn check(&mut self, input: &FileFingerprint, role: EvidenceRole) -> Result<()> {
        let path = Path::new(&input.path);
        ensure!(
            path.is_absolute() && path.components().all(|part| part != Component::ParentDir),
            "Input path is not normalized: {}",
            path.display()
        );
        let normalized: PathBuf = path.components().collect();
        let path = normalized.as_path();
        let Ok(relative) = path.strip_prefix(self.root) else {
            ensure!(
                role == EvidenceRole::Dependency,
                "Input outside Git checkout: {}",
                path.display()
            );
            return self.verify_dependency_alias(path);
        };
        ensure!(
            !self.traverses_tracked_link(relative),
            "Tracked link prevents exact input attribution: {}",
            relative.display()
        );
        let key = relative.to_str().context("Git input path is not UTF-8")?;
        if !self.tree.contains_key(key) {
            match role {
                EvidenceRole::Dependency => return self.verify_dependency_alias(path),
                EvidenceRole::Generated => {
                    require_unlinked(path, relative)?;
                    ensure!(
                        input.kind == InputKind::File,
                        "Missing generated product: {}",
                        relative.display()
                    );
                    return Ok(());
                }
                EvidenceRole::Revision | EvidenceRole::Watch => {}
            }
        }
        require_unlinked(path, relative)?;
        self.verify_git_input(relative, key, input)
    }

    fn verify_dependency_alias(&self, path: &Path) -> Result<()> {
        let resolved = match path.canonicalize() {
            Ok(resolved) => resolved,
            // Missing dependency probes are checked by filesystem reobservation.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("Resolve dependency {}", path.display()));
            }
        };
        if let Ok(relative) = resolved.strip_prefix(self.root) {
            let key = relative
                .to_str()
                .context("Git dependency path is not UTF-8")?;
            ensure!(
                !self.tree.contains_key(key),
                "Dependency alias resolves to a tracked path: {} -> {}",
                path.display(),
                relative.display()
            );
        }
        Ok(())
    }

    fn verify_git_input(
        &mut self,
        relative: &Path,
        key: &str,
        input: &FileFingerprint,
    ) -> Result<()> {
        let entry = self.tree.get(key);
        let matches = match input.kind {
            InputKind::File => Self::file_matches(&mut self.hashes, entry, &input.hash),
            InputKind::Directory => {
                (key.is_empty() || entry.is_some_and(TreeEntry::is_directory))
                    && self.directory_matches(relative, input)
            }
            InputKind::Missing => entry.is_none(),
        };
        ensure!(
            matches,
            "Input differs from the Git revision: {}",
            relative.display()
        );
        Ok(())
    }

    fn directory_matches(&self, relative: &Path, input: &FileFingerprint) -> bool {
        let names = self
            .directories
            .get(relative)
            .into_iter()
            .flatten()
            .filter(|name| !input.excluded_children.contains(name))
            .cloned()
            .collect();
        fingerprint::directory_names_hash(names) == input.hash
    }

    fn traverses_tracked_link(&self, relative: &Path) -> bool {
        relative
            .ancestors()
            .filter_map(Path::to_str)
            .filter_map(|path| self.tree.get(path))
            .any(TreeEntry::is_link)
    }

    fn file_matches(
        hashes: &mut BTreeMap<String, String>,
        entry: Option<&TreeEntry>,
        hash: &str,
    ) -> bool {
        let Some(id) = entry.and_then(TreeEntry::regular_blob) else {
            return false;
        };
        let previous = hashes.insert(id.into(), hash.into());
        previous.is_none_or(|previous| previous == hash)
    }
}

fn require_unlinked(path: &Path, relative: &Path) -> Result<()> {
    ensure!(
        fingerprint::required_link_bindings(path)?.is_empty(),
        "Linked input cannot be attributed: {}",
        relative.display()
    );
    Ok(())
}

fn directories(tree: &BTreeMap<String, TreeEntry>) -> BTreeMap<PathBuf, BTreeSet<String>> {
    let mut result = BTreeMap::<PathBuf, BTreeSet<String>>::new();
    for (path, entry) in tree {
        let path = Path::new(path);
        if fingerprint::includes_scoped_tree_entry(path, entry.is_directory(), entry.is_link()) {
            add_directory_member(&mut result, path);
        }
    }
    result
}

fn add_directory_member(members: &mut BTreeMap<PathBuf, BTreeSet<String>>, path: &Path) {
    let parent = path.parent().unwrap_or(Path::new(""));
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    members.entry(parent.into()).or_default().insert(name);
}

#[cfg(test)]
mod tests {
    use super::*;
    use codecallnavigator_core::artifact::provenance;
    use codecallnavigator_engine::analyzers::evidence::AnalyzerDescriptor;

    #[test]
    fn scheduling_watches_cannot_certify_a_selected_target() {
        let target = BuildTarget {
            path: "/repo".into(),
            label: "Selected target".into(),
            language: "fixture".into(),
            kind: "fixture".into(),
        };
        let watch = EvidenceInput::new(
            FileFingerprint {
                path: "/repo".into(),
                kind: InputKind::Directory,
                ..FileFingerprint::default()
            },
            EvidenceRole::Watch,
        );
        let evidence = [TargetEvidence {
            target: target.clone(),
            analyzer: AnalyzerDescriptor {
                backend: "fixture",
                version: "1".into(),
                mode: "fixture",
            },
            inputs: vec![watch],
            attribution_limit: None,
        }];
        let roots = vec![("repo".into(), std::env::current_dir().unwrap())];
        let analysis = provenance::build(&[], &[], "{}", &roots).unwrap();
        let observations = Observations {
            analysis: &analysis,
            roots: &roots,
            targets: std::slice::from_ref(&target),
            project_inputs: &[],
            evidence: &evidence,
        };

        assert_eq!(observations.inputs().count(), 0);
        let error = observations.require_attribution().unwrap_err();
        assert_eq!(
            error.to_string(),
            "Selected target: adapter reported no consumed inputs"
        );
    }
}
