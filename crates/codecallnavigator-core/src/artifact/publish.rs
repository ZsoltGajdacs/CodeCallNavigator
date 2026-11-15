use super::{Artifact, EncodedArtifact, GRAPH_PATH};
use anyhow::{Context, Result, ensure};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

/// A competing publisher owns the advisory project lock; this is retryable.
#[derive(Debug)]
pub struct LockBusy {
    pub owner: LockOwner,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockOwner {
    Analysis,
    Coordinator,
}
pub const COORDINATOR_LOCK_NAME: &str = "refresh.lock";
impl std::fmt::Display for LockBusy {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self.owner {
            LockOwner::Analysis => "Another analysis owns this project's graph",
            LockOwner::Coordinator => "Another automatic-refresh coordinator owns this project",
        })
    }
}
impl std::error::Error for LockBusy {}

#[derive(Debug)]
pub struct GenerationChanged;
impl std::fmt::Display for GenerationChanged {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Saved graph changed before the analysis acquired its publication lock")
    }
}
impl std::error::Error for GenerationChanged {}

/// Own this before taking the input snapshot, through final artifact publication.
pub struct AnalysisLock {
    project: PathBuf,
    _file: LockedFile,
}

impl AnalysisLock {
    pub fn acquire(project: &Path) -> Result<Self> {
        let (project, file) = acquire_file(project, "graph.lock", LockOwner::Analysis)?;
        Ok(Self {
            project,
            _file: file,
        })
    }

    pub fn require_generation(&self, expected: &str) -> Result<()> {
        let bytes = match fs::read(self.project.join(GRAPH_PATH)) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(GenerationChanged.into());
            }
            Err(error) => return Err(error).context("Cannot check locked graph generation"),
        };
        if blake3::hash(&bytes).to_hex().as_str() != expected {
            return Err(GenerationChanged.into());
        }
        Ok(())
    }

    /// Publish the exact canonical buffer used for the protocol digest.
    pub fn publish_encoded(&self, bytes: &EncodedArtifact, cancel: &AtomicBool) -> Result<()> {
        replace(&self.project, bytes.as_ref(), cancel)
    }

    pub fn project(&self) -> &Path {
        &self.project
    }

    pub fn publish(&self, artifact: &Artifact, cancel: &AtomicBool) -> Result<()> {
        replace(&self.project, &artifact.bytes()?, cancel)
    }
}

/// This lease bounds coordinators only; analysis children acquire graph.lock.
pub struct CoordinatorLease {
    _file: LockedFile,
}
impl CoordinatorLease {
    pub fn acquire(project: &Path) -> Result<Self> {
        let (_, file) = acquire_file(project, COORDINATOR_LOCK_NAME, LockOwner::Coordinator)?;
        Ok(Self { _file: file })
    }
}

// Constructed only after successful acquisition: failed contenders must never
// unlock a lock held by another guard.
struct LockedFile {
    file: File,
}

impl Drop for LockedFile {
    fn drop(&mut self) {
        // Explicit unlock releases the shared OS lock even if a cloned or
        // inherited descriptor remains open. Drop cannot return unlock errors;
        // closing this File remains the OS fallback if explicit unlock fails.
        let _ = self.file.unlock();
    }
}

fn acquire_file(project: &Path, name: &str, owner: LockOwner) -> Result<(PathBuf, LockedFile)> {
    let project = project
        .canonicalize()
        .context("Cannot open analysis project")?;
    let directory = project.join(".codecallnavigator");
    fs::create_dir_all(&directory)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(directory.join(name))?;
    match file.try_lock() {
        Ok(()) => Ok((project, LockedFile { file })),
        Err(std::fs::TryLockError::WouldBlock) => Err(LockBusy { owner }.into()),
        Err(error) => Err(error).context("Cannot acquire project lock"),
    }
}

pub fn load(project: &Path) -> Result<Artifact> {
    let path = project.join(GRAPH_PATH);
    Artifact::from_bytes(
        &fs::read(&path).with_context(|| format!("Cannot read {}", path.display()))?,
    )
}

/// One project artifact, replaced only after complete validation and serialization.
pub fn publish(project: &Path, artifact: &Artifact, cancel: &AtomicBool) -> Result<()> {
    AnalysisLock::acquire(project)?.publish(artifact, cancel)
}

fn replace(project: &Path, bytes: &[u8], cancel: &AtomicBool) -> Result<()> {
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "Analysis cancelled before graph publication"
    );
    let directory = project.join(".codecallnavigator");
    fs::create_dir_all(&directory)?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".graph-publish-")
        .tempfile_in(&directory)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "Analysis cancelled before graph replacement"
    );
    temporary
        .persist(project.join(GRAPH_PATH))
        .map_err(|e| e.error)?;
    File::open(&directory)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coordinator_lease_is_separate_and_never_changes_observed_input_revision() {
        let project = tempfile::tempdir().unwrap();
        let roots = [("repo".into(), project.path().into())];
        let inputs = crate::fingerprint::capture_excluding([project.path().into()], &[]).unwrap();
        let mut artifact =
            super::super::canonical(&crate::model::GraphData::default(), &roots).unwrap();
        artifact.graph["analysis"] = serde_json::to_value(
            super::super::provenance::build(&inputs, &[], "{}", &roots).unwrap(),
        )
        .unwrap();
        let observe = || {
            super::super::verification::observe_artifact(
                &artifact,
                "generation",
                &roots,
                &AtomicBool::new(false),
            )
            .unwrap()
        };
        let before = observe();
        let owner = CoordinatorLease::acquire(project.path()).unwrap();
        let error = CoordinatorLease::acquire(project.path()).err().unwrap();
        assert_eq!(
            error.downcast_ref::<LockBusy>().unwrap().owner,
            LockOwner::Coordinator
        );
        let _analysis = AnalysisLock::acquire(project.path()).unwrap();
        let after = observe();
        assert_eq!(before.input_revision, after.input_revision);
        assert_eq!(
            after.report.input_status,
            super::super::verification::InputStatus::Match
        );
        drop(owner);
        let _next_owner = CoordinatorLease::acquire(project.path()).unwrap();
    }

    // File clones share an OS lock description. Only a real filesystem test can
    // prove guard-drop release and that an old descriptor cannot unlock a new owner.
    #[test]
    fn analysis_guard_drop_releases_lock_with_a_retained_file_clone() {
        let project = tempfile::tempdir().unwrap();
        let owner = AnalysisLock::acquire(project.path()).unwrap();
        let retained = owner._file.file.try_clone().unwrap();
        for _ in 0..2 {
            let error = AnalysisLock::acquire(project.path()).err().unwrap();
            assert_eq!(
                error.downcast_ref::<LockBusy>().unwrap().owner,
                LockOwner::Analysis
            );
        }
        drop(owner);
        let next = AnalysisLock::acquire(project.path()).unwrap();
        drop(retained);
        for _ in 0..2 {
            let error = AnalysisLock::acquire(project.path()).err().unwrap();
            assert_eq!(
                error.downcast_ref::<LockBusy>().unwrap().owner,
                LockOwner::Analysis
            );
        }
        drop(next);
        let _final_owner = AnalysisLock::acquire(project.path()).unwrap();
    }

    #[test]
    fn coordinator_guard_drop_releases_lock_with_a_retained_file_clone() {
        let project = tempfile::tempdir().unwrap();
        let owner = CoordinatorLease::acquire(project.path()).unwrap();
        let retained = owner._file.file.try_clone().unwrap();
        for _ in 0..2 {
            let error = CoordinatorLease::acquire(project.path()).err().unwrap();
            assert_eq!(
                error.downcast_ref::<LockBusy>().unwrap().owner,
                LockOwner::Coordinator
            );
        }
        drop(owner);
        let next = CoordinatorLease::acquire(project.path()).unwrap();
        drop(retained);
        for _ in 0..2 {
            let error = CoordinatorLease::acquire(project.path()).err().unwrap();
            assert_eq!(
                error.downcast_ref::<LockBusy>().unwrap().owner,
                LockOwner::Coordinator
            );
        }
        drop(next);
        let _final_owner = CoordinatorLease::acquire(project.path()).unwrap();
    }

    #[test]
    fn competing_analysis_lock_is_a_typed_retryable_failure() {
        let project = tempfile::tempdir().unwrap();
        let _owner = AnalysisLock::acquire(project.path()).unwrap();
        let error = AnalysisLock::acquire(project.path()).err().unwrap();
        assert!(
            error.downcast_ref::<LockBusy>().is_some(),
            "Lock contention must be classified without message matching: {error:#}"
        );
    }
}
