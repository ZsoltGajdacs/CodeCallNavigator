use super::{Artifact, EncodedArtifact, GRAPH_PATH};
use anyhow::{Context, Result, ensure};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

/// Own this before taking the input snapshot, through final artifact publication.
pub struct AnalysisLock {
    project: PathBuf,
    _file: File,
}

impl AnalysisLock {
    pub fn acquire(project: &Path) -> Result<Self> {
        let project = project
            .canonicalize()
            .context("Cannot open analysis project")?;
        let directory = project.join(".codegraph");
        fs::create_dir_all(&directory)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(directory.join("graph.lock"))?;
        file.try_lock()
            .context("Another analysis owns this project's graph")?;
        Ok(Self {
            project,
            _file: file,
        })
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
    let directory = project.join(".codegraph");
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
