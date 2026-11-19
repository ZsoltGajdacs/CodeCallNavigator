//! One fixed analysis binding with lazy retained native preparation.
use super::analysis::{self, Attempt, Publication, RootOptions};
use super::session_inputs::Inputs;
use anyhow::{Context, Result, bail, ensure};
use codecallnavigator_core::{discovery::BuildTarget, protocol::Event};
use codecallnavigator_engine::analyzers::{AnalyzerRuntime, PreparedAnalysis};
use std::path::PathBuf;

enum NativeState {
    Pending,
    Prepared(PreparedAnalysis),
    Closed,
}

/// The host owns native state; each attempt still enters normal locked publication.
pub(super) struct AnalysisSession {
    project: PathBuf,
    configured_roots: Vec<(String, PathBuf)>,
    published_roots: Option<Vec<(String, PathBuf)>>,
    targets: Vec<BuildTarget>,
    infer_linked: bool,
    native: NativeState,
}

impl AnalysisSession {
    pub(super) fn bootstrap_inputs(&self) -> Result<Inputs> {
        Inputs::bootstrap(&self.project, &self.targets)
    }

    pub(super) fn latest_saved_inputs(&self) -> Result<(Inputs, String)> {
        let roots = self
            .published_roots
            .as_deref()
            .unwrap_or(&self.configured_roots);
        Inputs::saved(&self.project, roots, None, &self.targets)
    }

    pub fn record_published_roots(&mut self, roots: &[(String, PathBuf)]) {
        self.published_roots = Some(roots.to_vec());
    }

    pub fn new(
        project: PathBuf,
        configured_roots: Vec<(String, PathBuf)>,
        targets: Vec<BuildTarget>,
        infer_linked: bool,
    ) -> Result<Self> {
        let project = project
            .canonicalize()
            .context("Cannot open session project")?;
        ensure!(project.is_dir(), "Session project must be a directory");
        ensure!(!targets.is_empty(), "Select at least one build");
        Ok(Self {
            project,
            configured_roots,
            published_roots: None,
            targets,
            infer_linked,
            native: NativeState::Pending,
        })
    }

    pub fn attempt(
        &mut self,
        expected_generation: Option<String>,
        context: Attempt<'_>,
    ) -> Result<Event> {
        ensure!(
            !matches!(self.native, NativeState::Closed),
            "Analysis session is closed"
        );
        let publication = Publication {
            project: &self.project,
            options: RootOptions {
                configured: self.configured_roots.clone(),
                infer_linked: self.infer_linked,
                expected_generation,
            },
            targets: self.targets.clone(),
        };
        let native = &mut self.native;
        analysis::publish(publication, context, |targets, attempt| {
            // Publication supplies the same sorted/deduplicated selection used
            // for provenance. Resource discovery is fixed at first native use.
            if matches!(native, NativeState::Pending) {
                *native = NativeState::Prepared(PreparedAnalysis::new(
                    targets,
                    AnalyzerRuntime::discover()?,
                )?);
            }
            match native {
                NativeState::Prepared(prepared) => {
                    prepared.analyze_observed(attempt.cancel, attempt.progress)
                }
                NativeState::Closed => bail!("Analysis session is closed"),
                NativeState::Pending => unreachable!("native preparation was initialized above"),
            }
        })
    }

    /// Failure keeps preparation and diagnostics owned; callers must report it.
    pub fn shutdown(&mut self) -> Result<()> {
        if let NativeState::Prepared(prepared) = &mut self.native {
            prepared.shutdown()?;
        }
        self.native = NativeState::Closed;
        Ok(())
    }
}

impl Drop for AnalysisSession {
    fn drop(&mut self) {
        // Explicit shutdown reports errors. PreparedAnalysis retains the final
        // best-effort fallback and failed-cleanup evidence if this fails too.
        let _ = self.shutdown();
    }
}
