//! Attribute the exact analysis observations to an immutable Git revision.
use anyhow::{Context, Result, ensure};
use codecallnavigator_core::{
    artifact::{
        Roots,
        local_snapshots::SnapshotIdentity,
        provenance::Analysis,
        refresh_inputs,
        revision::{Repository, TreeEntry},
    },
    discovery::BuildTarget,
    fingerprint::{self, InputKind, InputPolicy},
    model::FileFingerprint,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

mod dependencies;
use dependencies::Dependencies;

pub(super) fn dependency_identity(observed: &Observations<'_>) -> Result<String> {
    let mut dependencies = Dependencies::new(observed.targets)?;
    for input in observed.analyzer.iter().chain(observed.refresh) {
        dependencies.observe(
            input,
            Path::new(&input.path).strip_prefix(observed.project).ok(),
        );
    }
    dependencies.digest()
}

pub(super) struct Candidate {
    repository: Repository,
    commit: String,
}

pub(super) struct Observations<'a> {
    pub analysis: &'a Analysis,
    pub roots: &'a Roots,
    pub analyzer: &'a [FileFingerprint],
    pub refresh: &'a [FileFingerprint],
    pub targets: &'a [BuildTarget],
    pub project: &'a Path,
    pub generated_sources: &'a [String],
    pub output_attribution_limit: Option<&'a str>,
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
        if let Some(reason) = observed.output_attribution_limit {
            anyhow::bail!("{reason}");
        }
        let tree = self.repository.tree(&self.commit, cancel)?;
        let outputs = refresh_inputs::owned_outputs(analysis, observed.roots).unwrap_or_default();
        let generated_sources =
            generated_roots(&self.repository.root, observed.generated_sources, &tree);
        let mut proof = InputProof {
            root: &self.repository.root,
            directories: directories(&tree),
            tree,
            hashes: BTreeMap::new(),
            outputs,
            dependencies: Dependencies::new(observed.targets)?,
            generated_sources,
        };
        for input in observed.analyzer.iter().chain(observed.refresh) {
            ensure!(
                !cancel.load(Ordering::Acquire),
                "Snapshot retention cancelled"
            );
            proof.check(input)?;
        }
        self.repository.verify_blobs(&proof.hashes, cancel)?;
        let dependency_identity = proof.dependencies.digest()?;
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
    outputs: Vec<PathBuf>,
    dependencies: Dependencies,
    generated_sources: Vec<PathBuf>,
}

impl InputProof<'_> {
    fn check(&mut self, input: &FileFingerprint) -> Result<()> {
        let path = Path::new(&input.path);
        let relative = path.strip_prefix(self.root).ok();
        let Some(relative) = relative else {
            ensure!(
                self.dependencies.observe(input, None),
                "Input outside Git checkout: {}",
                path.display()
            );
            return Ok(());
        };
        ensure!(
            !self.traverses_tracked_link(relative),
            "Tracked link prevents exact input attribution: {}",
            relative.display()
        );
        let key = relative.to_str().context("Git input path is not UTF-8")?;
        if !self.tree.contains_key(key) && self.accepts_untracked(path, relative, input)? {
            return Ok(());
        }
        require_unlinked(path, relative)?;
        self.verify_git_input(relative, key, input)
    }

    fn accepts_untracked(
        &mut self,
        path: &Path,
        relative: &Path,
        input: &FileFingerprint,
    ) -> Result<bool> {
        if self.outputs.iter().any(|output| path.starts_with(output)) {
            require_unlinked(path, relative)?;
            self.dependencies.record(relative, input);
            return Ok(true);
        }
        if self.dependencies.observe(input, Some(relative)) {
            return Ok(true);
        }
        if self
            .generated_sources
            .iter()
            .any(|root| path.starts_with(root))
        {
            require_unlinked(path, relative)?;
            self.dependencies.record(relative, input);
            return Ok(true);
        }
        Ok(false)
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

fn generated_roots(
    repository: &Path,
    outputs: &[String],
    tree: &BTreeMap<String, TreeEntry>,
) -> Vec<PathBuf> {
    // Recursive Git trees include directories. Any generated root with
    // authored contents is ineligible for the generated exemption.
    outputs
        .iter()
        .map(Path::new)
        .filter(|path| {
            path.strip_prefix(repository)
                .ok()
                .and_then(Path::to_str)
                .is_some_and(|relative| !relative.is_empty() && !tree.contains_key(relative))
        })
        .map(Path::to_path_buf)
        .collect()
}

fn directories(tree: &BTreeMap<String, TreeEntry>) -> BTreeMap<PathBuf, BTreeSet<String>> {
    let mut result = BTreeMap::<PathBuf, BTreeSet<String>>::new();
    for (path, entry) in tree {
        let path = Path::new(path);
        if fingerprint::includes_scoped_tree_entry(path, entry.is_directory(), entry.is_link()) {
            let parent = path.parent().unwrap_or(Path::new(""));
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            result
                .entry(parent.into())
                .or_default()
                .insert(name.into_owned());
        }
    }
    result
}
