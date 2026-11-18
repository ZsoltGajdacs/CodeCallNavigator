//! Native compilation evidence is attempt-local and never serialized into graphs.
use super::build::{JavaBuildMetadata, JavaSourceSet};
use crate::{fingerprint, model::FileFingerprint};
use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

mod snapshot;
#[cfg(test)]
mod tests;
use snapshot::{Snapshot, absolute, changed};

pub(super) const VERSION: u32 = 1;
static ATTEMPTS: AtomicU64 = AtomicU64::new(0);

pub(super) struct Attempt {
    file: PathBuf,
    identity: String,
}

impl Attempt {
    pub fn new(directory: &Path) -> Result<Self> {
        let file = directory.join("java-compile-observation.json");
        match fs::remove_file(&file) {
            Ok(()) => (),
            Err(error) if error.kind() == ErrorKind::NotFound => (),
            Err(error) => {
                return Err(error).context("Cannot clear previous Java compilation evidence");
            }
        }
        let identity = format!(
            "{}-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos(),
            ATTEMPTS.fetch_add(1, Ordering::Relaxed)
        );
        Ok(Self { file, identity })
    }

    pub fn configure(&self, command: &mut Command) {
        command
            .arg(format!(
                "-Dcodecallnavigator.observation.file={}",
                self.file.display()
            ))
            .arg(format!(
                "-Dcodecallnavigator.observation.attempt={}",
                self.identity
            ));
    }

    /// A reaped compiler failure remains authoritative unless the current native
    /// observer supplied a valid, typed consistency diagnostic. Cancellation wins.
    pub fn finish(self, native: Result<()>, cancelled: bool) -> Result<Observations> {
        if cancelled {
            native?;
            bail!("Analysis cancelled");
        }
        let evidence = fs::read(&self.file)
            .with_context(|| {
                format!(
                    "Java compiler did not export current input evidence: {}",
                    self.file.display()
                )
            })
            .and_then(|bytes| decode(&bytes, &self.identity));
        if let Err(native_error) = native {
            if native_error
                .downcast_ref::<crate::analyzers::process::NativeExitFailure>()
                .is_some_and(|failure| failure.exited())
                && let Ok(observed) = &evidence
            {
                observed.require_no_failure()?;
            }
            return Err(native_error);
        }
        let observed = evidence?;
        observed.require_no_failure()?;
        observed.require_supported()?;
        Ok(observed)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Observations {
    version: u32,
    attempt: String,
    operations: Vec<Operation>,
    failure: Option<Failure>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Operation {
    project: PathBuf,
    source_set: String,
    operation: String,
    outcome: Outcome,
    before: Snapshot,
    consumed_after: Snapshot,
    after: Snapshot,
    outputs: Outputs,
    detail: Option<String>,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Outcome {
    Success,
    UpToDate,
    FromCache,
    NoSource,
    NotApplicable,
    Unsupported,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Outputs {
    classes: Vec<PathBuf>,
    generated_sources: Vec<PathBuf>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Failure {
    path: PathBuf,
    change: Change,
    operation: String,
}

#[derive(Deserialize)]
enum Change {
    #[serde(rename = "content_changed")]
    Content,
    #[serde(rename = "membership_changed")]
    Membership,
    #[serde(rename = "kind_changed")]
    Kind,
    #[serde(rename = "link_changed")]
    Link,
}

fn decode(bytes: &[u8], attempt: &str) -> Result<Observations> {
    let observed: Observations = serde_json::from_slice(bytes)
        .context("Invalid Java compilation input evidence; rebuild matching analyzer resources")?;
    ensure!(
        observed.version == VERSION,
        "Unsupported Java compile observation version; rebuild matching analyzer resources"
    );
    ensure!(
        observed.attempt == attempt,
        "Java compilation evidence belongs to a different attempt"
    );
    let mut operations = BTreeSet::new();
    for operation in &observed.operations {
        operation.validate()?;
        ensure!(
            operations.insert((
                &operation.project,
                &operation.source_set,
                &operation.operation
            )),
            "Duplicate Java compiler operation: {}",
            operation.operation
        );
    }
    if let Some(failure) = &observed.failure {
        absolute(&failure.path)?;
        ensure!(
            observed
                .operations
                .iter()
                .any(|operation| operation.operation == failure.operation),
            "Java consistency diagnostic has no compiler operation"
        );
    }
    Ok(observed)
}

impl Operation {
    fn validate(&self) -> Result<()> {
        absolute(&self.project)?;
        ensure!(
            !self.source_set.is_empty() && !self.operation.is_empty(),
            "Java compilation observation lacks native ownership"
        );
        self.before.validate()?;
        self.consumed_after.validate()?;
        self.after.validate()?;
        for output in self
            .outputs
            .classes
            .iter()
            .chain(&self.outputs.generated_sources)
        {
            absolute(output)?;
        }
        Ok(())
    }

    fn require_supported(&self) -> Result<()> {
        ensure!(
            self.outcome != Outcome::Unsupported,
            "Unsupported Java compiler observation for {}: {}",
            self.operation,
            self.detail
                .as_deref()
                .unwrap_or("native compiler inputs could not be observed")
        );
        Ok(())
    }

    fn verify(&self) -> Result<()> {
        self.require_supported()?;
        self.before.require_same_consumed(&self.consumed_after)?;
        self.after
            .verify(&self.outputs.classes)
            .with_context(|| format!("Java compiler input consistency for {}", self.operation))
    }

    fn owns(&self, project: &Path, source_set: &str) -> Result<bool> {
        Ok(self.source_set == source_set
            && self.project.canonicalize().with_context(|| {
                format!(
                    "Cannot resolve Java operation project {}",
                    self.project.display()
                )
            })? == project)
    }
}

impl Observations {
    fn require_supported(&self) -> Result<()> {
        for operation in &self.operations {
            operation.require_supported()?;
        }
        Ok(())
    }

    fn require_no_failure(&self) -> Result<()> {
        if let Some(failure) = &self.failure {
            let reason = match failure.change {
                Change::Content => "compiler input content changed",
                Change::Membership => "compiler input membership changed",
                Change::Kind => "compiler input kind changed",
                Change::Link => "required link target changed",
            };
            return Err(changed(&failure.path, reason))
                .with_context(|| format!("Java compiler operation {}", failure.operation));
        }
        Ok(())
    }

    pub fn verify(&self, metadata: &JavaBuildMetadata) -> Result<()> {
        for operation in &self.operations {
            operation.verify()?;
        }
        for source in &metadata.source_sets {
            self.require_coverage(source)?;
        }
        Ok(())
    }

    fn require_coverage(&self, source: &JavaSourceSet) -> Result<()> {
        for operation in &self.operations {
            if operation.outcome != Outcome::NotApplicable
                && operation.owns(&source.project_dir, &source.source_set)?
            {
                return Ok(());
            }
        }
        ensure!(
            !has_compile_content(source)?,
            "Java compiler supplied no input observation for {} / {}; unsupported compiler configuration",
            source.project,
            source.source_set
        );
        Ok(())
    }

    pub fn fingerprints(&self, metadata: &JavaBuildMetadata) -> Result<Vec<FileFingerprint>> {
        self.verify(metadata)?;
        let paths = self
            .operations
            .iter()
            .flat_map(|operation| operation.after.paths().cloned());
        let fingerprints = fingerprint::capture_excluding(paths, &metadata.output_roots)?;
        // Bridge native SHA-256 observations to core's existing BLAKE3/link policy
        // without accepting a newly captured baseline after a concurrent edit.
        self.verify(metadata)?;
        Ok(fingerprints)
    }
}

fn has_compile_content(source: &JavaSourceSet) -> Result<bool> {
    let files = fingerprint::source_paths(&source.source_roots)?;
    if files.iter().any(|path| {
        path.extension()
            .is_some_and(|extension| extension == "java")
    }) {
        return Ok(true);
    }
    for directory in &source.class_dirs {
        if super::contains_class(directory)? {
            return Ok(true);
        }
    }
    Ok(false)
}
