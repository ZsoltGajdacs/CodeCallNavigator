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
    _file: File,
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
    _file: File,
}
impl CoordinatorLease {
    pub fn acquire(project: &Path) -> Result<Self> {
        let (_, file) = acquire_file(project, COORDINATOR_LOCK_NAME, LockOwner::Coordinator)?;
        Ok(Self { _file: file })
    }
}

fn acquire_file(project: &Path, name: &str, owner: LockOwner) -> Result<(PathBuf, File)> {
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
        Ok(()) => Ok((project, file)),
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
        // Other parallel tests can fork while this lease is held. The inherited
        // descriptor closes at exec; reacquisition is eventual, not synchronous
        // with dropping this process's last descriptor.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            match CoordinatorLease::acquire(project.path()) {
                Ok(_) => break,
                Err(error) => {
                    assert!(error.is::<LockBusy>(), "{error:#}");
                    assert!(
                        std::time::Instant::now() < deadline,
                        "Lease remained held: {error:#}"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        }
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
