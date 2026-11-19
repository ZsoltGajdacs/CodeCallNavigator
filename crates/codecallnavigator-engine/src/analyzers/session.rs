//! Fixed-selection, engine-owned preparation and raw extraction reuse.
use super::{
    AnalysisJob, AnalyzerRuntime, JobDirectory, ObservedAnalysis, compose, csharp, engine_versions,
    ensure_unchanged, java, process::check_cancel, typescript,
};
use crate::process_ownership::CleanupFailure;
use crate::{discovery::BuildTarget, model::GraphData};
use anyhow::{Context, Result, bail, ensure};
use codecallnavigator_core::protocol::{AnalysisPhase, TargetFailure};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};

/// Retains compiler preparation independently from graph inference/publication.
/// Selection and runtime are fixed; changing either requires a new session.
pub struct PreparedAnalysis {
    jobs: Vec<SelectedJob>,
    runtime: AnalyzerRuntime,
    // Helper inventories are cleared between extraction passes. A continuous
    // host must also bound retained failed-generation directories.
    directory: JobDirectory,
    generation: u64,
    failed: bool,
    cleanup_failure: Option<CleanupFailure>,
}

struct SelectedJob {
    build: BuildTarget,
    path: PathBuf,
    preflight: Option<java::PreparedJava>,
    preparation: Option<Preparation>,
    graph: Option<GraphData>,
    preserve_logs: bool,
    maven_sources: Option<java::MavenSources>,
}

enum Preparation {
    Csharp(csharp::PreparedCsharp),
    Java(java::CompiledJava),
    TypeScript(typescript::PreparedTypeScript),
}

impl Drop for PreparedAnalysis {
    fn drop(&mut self) {
        if let Err(error) = self.shutdown() {
            self.directory.remove_on_drop = false;
            eprintln!("{error:#}");
        }
    }
}

#[derive(Clone, Copy)]
enum ValidationMode {
    ReprepareChanged,
    AfterNativeProducers,
    BeforeExtraction,
    RequireCurrent,
}

impl ValidationMode {
    fn permits_reprepare(self, preparation: &Option<Preparation>) -> bool {
        match self {
            Self::ReprepareChanged => true,
            Self::AfterNativeProducers => matches!(preparation, Some(Preparation::TypeScript(_))),
            Self::BeforeExtraction | Self::RequireCurrent => false,
        }
    }
}

impl Preparation {
    /// Return true when native preparation is reusable but extraction changed.
    fn verify(
        &mut self,
        build: &BuildTarget,
        context: &AnalysisJob<'_>,
        mode: ValidationMode,
    ) -> Result<bool> {
        match self {
            Self::Csharp(prepared)
                if matches!(
                    mode,
                    ValidationMode::ReprepareChanged | ValidationMode::RequireCurrent
                ) =>
            {
                prepared.verify_context(build, context)?;
            }
            Self::Csharp(prepared) => prepared.verify()?,
            Self::Java(prepared) if matches!(mode, ValidationMode::ReprepareChanged) => {
                return prepared.reconcile_outputs();
            }
            Self::Java(prepared) => prepared.verify()?,
            Self::TypeScript(prepared) => prepared.verify()?,
        }
        Ok(false)
    }
}

impl SelectedJob {
    fn failure(
        &self,
        error: anyhow::Error,
        phase: AnalysisPhase,
        cancel: &AtomicBool,
    ) -> anyhow::Error {
        crate::failure::attribute(
            error,
            TargetFailure {
                target: self.build.identity(),
                phase,
                logs: self.path.is_absolute().then(|| self.path.clone()),
            },
            cancel.load(Ordering::Acquire),
        )
    }

    fn needs_typescript_update(&self) -> bool {
        matches!(&self.preparation, Some(Preparation::TypeScript(prepared)) if prepared.needs_update())
    }

    fn needs_typescript_extraction(&self) -> bool {
        self.graph.is_none() && matches!(self.preparation, Some(Preparation::TypeScript(_)))
    }

    fn graph_for_composition(&mut self, consume: bool) -> GraphData {
        if consume {
            self.graph.take().unwrap()
        } else {
            self.graph.as_ref().unwrap().clone()
        }
    }

    fn verify(&mut self, context: &AnalysisJob<'_>, mode: ValidationMode) -> Result<()> {
        let Some(preparation) = &mut self.preparation else {
            return Ok(());
        };
        if preparation.verify(&self.build, context, mode)? {
            self.graph = None;
        }
        if let Some(graph) = &self.graph {
            ensure_unchanged(&graph.fingerprints)?;
        }
        Ok(())
    }

    fn invalidate(&mut self) -> Result<()> {
        self.graph = None;
        if let Some(Preparation::TypeScript(prepared)) = &mut self.preparation {
            prepared.invalidate_for_update();
            return Ok(());
        }
        self.preparation = None;
        self.preflight = None;
        if !self.preserve_logs {
            self.retire_directory()?;
        }
        Ok(())
    }

    fn retire_directory(&mut self) -> Result<()> {
        if !self.path.as_os_str().is_empty() {
            fs::remove_dir_all(&self.path).with_context(|| {
                format!("Cannot remove obsolete preparation {}", self.path.display())
            })?;
            self.path.clear();
        }
        Ok(())
    }

    fn extract(&mut self, context: &AnalysisJob<'_>) -> Result<()> {
        if self.graph.is_some() {
            return Ok(());
        }
        let graph = match self
            .preparation
            .as_mut()
            .context("Prepare analysis before extraction")?
        {
            Preparation::Csharp(prepared) => csharp::extract(&self.build, context, prepared),
            Preparation::Java(prepared) => java::extract_java(&self.build, context, prepared),
            Preparation::TypeScript(prepared) => {
                typescript::extract(&self.build, context, prepared)
            }
        };
        let graph = match graph {
            Ok(graph) => graph,
            Err(error) => {
                self.preserve_logs = true;
                // Semantic metadata may have changed beyond the recorded path
                // guards. Do not let a subsequent prepare retain that baseline.
                if compilation_invalidated(&error) && check_cancel(context.cancel).is_ok() {
                    self.invalidate()?;
                }
                return Err(error);
            }
        };
        check_cancel(context.cancel)?;
        self.graph = Some(graph);
        Ok(())
    }
}

fn compilation_invalidated(error: &anyhow::Error) -> bool {
    error.is::<crate::fingerprint::InputsChanged>() && !error.is::<java::ExtractionOutputsChanged>()
}

impl PreparedAnalysis {
    pub fn new(builds: Vec<BuildTarget>, runtime: AnalyzerRuntime) -> Result<Self> {
        ensure!(!builds.is_empty(), "Select at least one build");
        for build in &builds {
            ensure!(
                matches!(build.language.as_str(), "java" | "csharp" | "typescript"),
                "Unsupported language: {}",
                build.language
            );
        }
        let jobs = builds
            .into_iter()
            .map(|build| SelectedJob {
                build,
                path: PathBuf::new(),
                preflight: None,
                preparation: None,
                graph: None,
                preserve_logs: false,
                maven_sources: None,
            })
            .collect();
        Ok(Self {
            jobs,
            runtime,
            directory: JobDirectory::new()?,
            generation: 0,
            failed: false,
            cleanup_failure: None,
        })
    }

    /// Prepare all compiler outputs. Java ownership is checked before compilation;
    /// all Java builds complete before TypeScript captures frontend inputs.
    pub fn prepare(&mut self, cancel: &AtomicBool, progress: &dyn Fn(String)) -> Result<()> {
        self.require_reusable()?;
        let result = self.prepare_inner(cancel, progress);
        self.record_result(&result);
        result
    }

    /// Extract an independent raw aggregate. Cached parts never receive inference
    /// or publication mutations; their only copy is consumed by composition.
    pub fn extract(&mut self, cancel: &AtomicBool, progress: &dyn Fn(String)) -> Result<GraphData> {
        self.require_reusable()?;
        let result = self.extract_inner(cancel, progress, false);
        self.record_result(&result);
        result.map(|observed| observed.graph)
    }

    pub fn analyze(&mut self, cancel: &AtomicBool, progress: &dyn Fn(String)) -> Result<GraphData> {
        Ok(self.analyze_observed(cancel, progress)?.graph)
    }

    pub fn analyze_observed(
        &mut self,
        cancel: &AtomicBool,
        progress: &dyn Fn(String),
    ) -> Result<ObservedAnalysis> {
        let _timing = crate::metrics::stage("analysis");
        self.prepare(cancel, progress)?;
        let result = self.extract_inner(cancel, progress, false);
        self.record_result(&result);
        result
    }

    pub(super) fn into_observed(
        mut self,
        cancel: &AtomicBool,
        progress: &dyn Fn(String),
    ) -> Result<ObservedAnalysis> {
        let _timing = crate::metrics::stage("analysis");
        self.prepare(cancel, progress)?;
        let result = self.extract_inner(cancel, progress, true);
        self.record_result(&result);
        let cleanup = self.shutdown();
        cleanup?;
        result
    }

    /// Confirm all owned workers stopped before retiring this preparation.
    /// Uncertain cleanup remains typed and preserves the attempt's diagnostics.
    pub fn shutdown(&mut self) -> Result<()> {
        let result = self.stop_workers().and_then(|()| self.require_reusable());
        self.record_result(&result);
        result
    }

    fn stop_workers(&mut self) -> Result<()> {
        let mut failure = None;
        for job in &mut self.jobs {
            if let Some(Preparation::TypeScript(prepared)) = &mut job.preparation {
                let result = prepared.release_worker().with_context(|| {
                    format!(
                        "Cannot stop {} preparation; logs: {}",
                        job.build.label,
                        job.path.display()
                    )
                });
                if let Err(error) = result {
                    failure.get_or_insert(error);
                }
            }
        }
        failure.map_or(Ok(()), Err)
    }

    fn record_result<T>(&mut self, result: &Result<T>) {
        if let Err(error) = result
            && let Some(failure) = error.downcast_ref::<CleanupFailure>()
        {
            self.cleanup_failure = Some(failure.clone());
        }
        self.failed |= result.is_err();
        self.directory.remove_on_drop = !self.failed;
    }

    fn require_reusable(&self) -> Result<()> {
        match &self.cleanup_failure {
            Some(failure) => Err(failure.clone().into()),
            None => Ok(()),
        }
    }

    fn context<'a>(
        &'a self,
        path: &'a std::path::Path,
        cancel: &'a AtomicBool,
        progress: &'a dyn Fn(String),
    ) -> AnalysisJob<'a> {
        AnalysisJob {
            runtime: &self.runtime,
            path,
            shared_path: &self.directory.path,
            cancel,
            progress,
        }
    }

    fn validate(
        &mut self,
        cancel: &AtomicBool,
        progress: &dyn Fn(String),
        mode: ValidationMode,
    ) -> Result<()> {
        check_cancel(cancel)?;
        let mut failure = None;
        for index in 0..self.jobs.len() {
            let path = self.jobs[index].path.clone();
            let context = AnalysisJob {
                runtime: &self.runtime,
                path: &path,
                shared_path: &self.directory.path,
                cancel,
                progress,
            };
            if let Err(error) = self.jobs[index].verify(&context, mode) {
                let error = self.jobs[index].failure(error, AnalysisPhase::Validation, cancel);
                // Cancellation interrupts verification; it does not prove the
                // retained compiler state invalid.
                check_cancel(cancel)?;
                if !error.is::<crate::fingerprint::InputsChanged>() {
                    return Err(error);
                }
                if error.is::<java::ExtractionOutputsChanged>() {
                    self.jobs[index].graph = None;
                    return Err(error);
                }
                let reprepare = mode.permits_reprepare(&self.jobs[index].preparation);
                self.jobs[index].invalidate()?;
                if !reprepare && failure.is_none() {
                    failure = Some(error);
                }
            }
        }
        if let Some(error) = failure {
            return Err(error);
        }
        Ok(())
    }

    fn begin(&mut self, index: usize) -> Result<()> {
        // Every preceding operation either confirmed cleanup or made this
        // session terminal. Keep the last failed directory until a replacement.
        self.jobs[index].retire_directory()?;
        self.generation += 1;
        let path = self
            .directory
            .path
            .join(format!("{index}-{}", self.generation));
        fs::create_dir(&path)?;
        self.jobs[index].path = path;
        self.jobs[index].preserve_logs = false;
        Ok(())
    }

    fn prepare_inner(&mut self, cancel: &AtomicBool, progress: &dyn Fn(String)) -> Result<()> {
        let _timing = crate::metrics::stage("preparation");
        self.validate(cancel, progress, ValidationMode::ReprepareChanged)?;
        self.java_preflight(cancel, progress)?;
        self.compile_java(cancel, progress)?;
        // Producer builds can invalidate previously prepared frontend snapshots.
        self.validate(cancel, progress, ValidationMode::AfterNativeProducers)?;
        self.prepare_language("csharp", cancel, progress)?;
        self.validate(cancel, progress, ValidationMode::AfterNativeProducers)?;
        self.prepare_language("typescript", cancel, progress)?;
        typescript::reject_overlaps(self.jobs.iter().filter_map(|job| match &job.preparation {
            Some(Preparation::TypeScript(prepared)) => Some(prepared),
            _ => None,
        }))?;
        self.validate(cancel, progress, ValidationMode::RequireCurrent)
    }

    fn prepare_language(
        &mut self,
        language: &str,
        cancel: &AtomicBool,
        progress: &dyn Fn(String),
    ) -> Result<()> {
        for index in 0..self.jobs.len() {
            check_cancel(cancel)?;
            if self.jobs[index].build.language != language {
                continue;
            }
            self.prepare_selected(index, cancel, progress)?;
        }
        Ok(())
    }

    fn prepare_selected(
        &mut self,
        index: usize,
        cancel: &AtomicBool,
        progress: &dyn Fn(String),
    ) -> Result<()> {
        if self.jobs[index].needs_typescript_update() {
            return self.update_typescript(index, cancel, progress);
        }
        if self.jobs[index].preparation.is_some() {
            return Ok(());
        }
        if self.jobs[index].build.language == "typescript" {
            self.release_other_typescript_workers(index)?;
        }
        self.begin(index)
            .map_err(|error| self.jobs[index].failure(error, AnalysisPhase::Preparation, cancel))?;
        let context = self.context(&self.jobs[index].path, cancel, progress);
        let preparation = match self.jobs[index].build.language.as_str() {
            "csharp" => csharp::prepare(&self.jobs[index].build, &context).map(Preparation::Csharp),
            "typescript" => {
                typescript::prepare(&self.jobs[index].build, &context).map(Preparation::TypeScript)
            }
            other => bail!("Unsupported language: {other}"),
        }
        .map_err(|error| self.jobs[index].failure(error, AnalysisPhase::Preparation, cancel))?;
        self.jobs[index].preparation = Some(preparation);
        Ok(())
    }

    fn update_typescript(
        &mut self,
        index: usize,
        cancel: &AtomicBool,
        progress: &dyn Fn(String),
    ) -> Result<()> {
        self.release_other_typescript_workers(index)?;
        let job = &mut self.jobs[index];
        let context = AnalysisJob {
            runtime: &self.runtime,
            path: &job.path,
            shared_path: &self.directory.path,
            cancel,
            progress,
        };
        let Some(Preparation::TypeScript(prepared)) = &mut job.preparation else {
            unreachable!("only retained TypeScript state can be updated");
        };
        let result = prepared.update(&job.build, &context);
        result.map_err(|error| job.failure(error, AnalysisPhase::Preparation, cancel))
    }

    /// At most one live Node compiler per analysis session, including mixed builds.
    fn release_other_typescript_workers(&mut self, retained: usize) -> Result<()> {
        for (index, job) in self.jobs.iter_mut().enumerate() {
            if index != retained
                && let Some(Preparation::TypeScript(prepared)) = &mut job.preparation
            {
                prepared.release_worker()?;
            }
        }
        Ok(())
    }

    fn java_preflight(&mut self, cancel: &AtomicBool, progress: &dyn Fn(String)) -> Result<()> {
        for index in 0..self.jobs.len() {
            check_cancel(cancel)?;
            let job = &self.jobs[index];
            if job.build.language != "java" || job.preparation.is_some() {
                continue;
            }
            if let Some(sources) = &job.maven_sources {
                sources.require_newer_edits(cancel)?;
            }
            self.begin(index).map_err(|error| {
                self.jobs[index].failure(error, AnalysisPhase::Preparation, cancel)
            })?;
            let context = self.context(&self.jobs[index].path, cancel, progress);
            let preflight = java::prepare(&self.jobs[index].build, &context).map_err(|error| {
                self.jobs[index].failure(error, AnalysisPhase::Preparation, cancel)
            })?;
            self.jobs[index].preflight = Some(preflight);
        }
        java::reject_overlaps(self.jobs.iter().filter_map(|job| {
            let preflight = match &job.preparation {
                Some(Preparation::Java(compiled)) => Some(compiled.preflight()),
                _ => job.preflight.as_ref(),
            };
            preflight.map(|prepared| (&job.build, prepared))
        }))
    }

    fn compile_java(&mut self, cancel: &AtomicBool, progress: &dyn Fn(String)) -> Result<()> {
        for index in 0..self.jobs.len() {
            check_cancel(cancel)?;
            let Some(preflight) = self.jobs[index].preflight.take() else {
                continue;
            };
            let context = self.context(&self.jobs[index].path, cancel, progress);
            let prepared =
                java::compile(&self.jobs[index].build, &context, preflight).map_err(|error| {
                    self.jobs[index].failure(error, AnalysisPhase::Compilation, cancel)
                })?;
            if self.jobs[index].build.kind == "maven" {
                self.jobs[index].maven_sources = Some(prepared.maven_sources());
            }
            self.jobs[index].preparation = Some(Preparation::Java(prepared));
        }
        Ok(())
    }

    fn clear_extraction_cache(&self) -> Result<()> {
        // The bytecode helper's inventory keys contain paths, not content. Its
        // immutable view is valid only within one extraction pass, never across
        // compilation/retry boundaries. No helper is active between passes.
        let cache = self.directory.path.join("artifact-inventories");
        match fs::remove_dir_all(&cache) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error).with_context(|| {
                format!(
                    "Cannot remove obsolete extraction cache {}",
                    cache.display()
                )
            }),
        }
    }

    fn extract_inner(
        &mut self,
        cancel: &AtomicBool,
        progress: &dyn Fn(String),
        consume: bool,
    ) -> Result<ObservedAnalysis> {
        let _timing = crate::metrics::stage("extraction");
        check_cancel(cancel)?;
        self.clear_extraction_cache()?;
        self.validate(cancel, progress, ValidationMode::BeforeExtraction)?;
        ensure!(
            self.jobs.iter().all(|job| job.preparation.is_some()),
            "Prepare analysis before extraction"
        );
        self.extract_selected_jobs(cancel, progress)?;
        // Even cached raw graphs require fresh semantic import membership before
        // the aggregate can leave the engine-owned preparation boundary.
        self.validate(cancel, progress, ValidationMode::RequireCurrent)?;
        let graph = self.compose_graphs(consume)?;
        graph.validate()?;
        check_cancel(cancel)?;
        crate::metrics::count("symbols", graph.nodes.len() as u64);
        crate::metrics::count("relationships", graph.edges.len() as u64);
        let output_attribution_limit = self.jobs.iter().find_map(|job| match &job.preparation {
            Some(Preparation::Java(compiled)) => {
                compiled.output_attribution_limit().map(str::to_owned)
            }
            Some(Preparation::TypeScript(_) | Preparation::Csharp(_)) | None => None,
        });
        Ok(ObservedAnalysis {
            graph,
            output_attribution_limit,
        })
    }

    fn extract_selected_jobs(
        &mut self,
        cancel: &AtomicBool,
        progress: &dyn Fn(String),
    ) -> Result<()> {
        for index in 0..self.jobs.len() {
            check_cancel(cancel)?;
            if self.jobs[index].needs_typescript_extraction() {
                self.release_other_typescript_workers(index)?;
            }
            self.extract_selected(index, cancel, progress)?;
        }
        Ok(())
    }

    fn extract_selected(
        &mut self,
        index: usize,
        cancel: &AtomicBool,
        progress: &dyn Fn(String),
    ) -> Result<()> {
        let job = &mut self.jobs[index];
        progress(format!("Extracting {}", job.build.label));
        let path = job.path.clone();
        let context = AnalysisJob {
            runtime: &self.runtime,
            path: &path,
            shared_path: &self.directory.path,
            cancel,
            progress,
        };
        job.extract(&context)
            .map_err(|error| job.failure(error, AnalysisPhase::Extraction, cancel))
            .with_context(|| format!("Analysis failed. Logs: {}", job.path.display()))
    }

    fn compose_graphs(&mut self, consume: bool) -> Result<GraphData> {
        let mut composition = compose::GraphComposition::default();
        let mut versions = serde_json::Map::new();
        for job in &mut self.jobs {
            let graph = job.graph_for_composition(consume);
            record_project_versions(&mut versions, &job.build, &graph)?;
            composition.append(graph)?;
        }
        let mut graph = composition.finish();
        ensure_unchanged(&graph.fingerprints)?;
        let mut engines: serde_json::Value = serde_json::from_str(&engine_versions()?)?;
        if !versions.is_empty() {
            engines["projects"] = serde_json::Value::Object(versions);
        }
        graph.engine_versions = engines.to_string();
        Ok(graph)
    }
}

fn record_project_versions(
    versions: &mut serde_json::Map<String, serde_json::Value>,
    build: &BuildTarget,
    graph: &GraphData,
) -> Result<()> {
    if !graph.engine_versions.is_empty() {
        versions.insert(
            build.path.to_string_lossy().into_owned(),
            serde_json::from_str(&graph.engine_versions)?,
        );
    }
    Ok(())
}
