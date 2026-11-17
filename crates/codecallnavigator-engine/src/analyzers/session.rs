//! Fixed-selection, engine-owned preparation and raw extraction reuse.
use super::{
    AnalysisJob, AnalyzerRuntime, JobDirectory, compose, csharp, engine_versions, ensure_unchanged,
    java, process::check_cancel, typescript,
};
use crate::{discovery::BuildTarget, model::GraphData};
use anyhow::{Context, Result, bail, ensure};
use std::{fs, path::PathBuf, sync::atomic::AtomicBool};

/// Retains compiler preparation independently from graph inference/publication.
/// Selection and runtime are fixed; changing either requires a new session.
pub struct PreparedAnalysis {
    // Drop owned workers before the directory that contains their logs.
    jobs: Vec<SelectedJob>,
    runtime: AnalyzerRuntime,
    // Helper inventories are cleared between extraction passes. A continuous
    // host must also bound retained failed-generation directories.
    directory: JobDirectory,
    generation: u64,
    failed: bool,
}

struct SelectedJob {
    build: BuildTarget,
    path: PathBuf,
    preflight: Option<java::PreparedJava>,
    preparation: Option<Preparation>,
    graph: Option<GraphData>,
    preserve_logs: bool,
}

enum Preparation {
    Csharp(csharp::PreparedCsharp),
    Java(java::CompiledJava),
    TypeScript(typescript::PreparedTypeScript),
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

impl SelectedJob {
    fn verify(&self, context: &AnalysisJob<'_>, mode: ValidationMode) -> Result<()> {
        match &self.preparation {
            Some(Preparation::Csharp(prepared)) => match mode {
                ValidationMode::ReprepareChanged | ValidationMode::RequireCurrent => {
                    prepared.verify_context(&self.build, context)?;
                }
                ValidationMode::AfterNativeProducers | ValidationMode::BeforeExtraction => {
                    prepared.verify()?;
                }
            },
            Some(Preparation::Java(prepared)) => prepared.verify()?,
            Some(Preparation::TypeScript(prepared)) => prepared.verify()?,
            None => return Ok(()),
        }
        if let Some(graph) = &self.graph {
            ensure_unchanged(&graph.fingerprints)?;
        }
        Ok(())
    }

    fn invalidate(&mut self) -> Result<()> {
        self.graph = None;
        // Dropping preparation stops and reaps its owned worker before deletion.
        self.preparation = None;
        self.preflight = None;
        if !self.preserve_logs && !self.path.as_os_str().is_empty() {
            fs::remove_dir_all(&self.path).with_context(|| {
                format!("Cannot remove obsolete preparation {}", self.path.display())
            })?;
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
                if error.is::<crate::fingerprint::InputsChanged>()
                    && check_cancel(context.cancel).is_ok()
                {
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
            })
            .collect();
        Ok(Self {
            jobs,
            runtime,
            directory: JobDirectory::new()?,
            generation: 0,
            failed: false,
        })
    }

    /// Prepare all compiler outputs. Java ownership is checked before compilation;
    /// all Java builds complete before TypeScript captures frontend inputs.
    pub fn prepare(&mut self, cancel: &AtomicBool, progress: &dyn Fn(String)) -> Result<()> {
        let result = self.prepare_inner(cancel, progress);
        self.record_result(&result);
        result
    }

    /// Extract an independent raw aggregate. Cached parts never receive inference
    /// or publication mutations; their only copy is consumed by composition.
    pub fn extract(&mut self, cancel: &AtomicBool, progress: &dyn Fn(String)) -> Result<GraphData> {
        let result = self.extract_inner(cancel, progress, false);
        self.record_result(&result);
        result
    }

    pub fn analyze(&mut self, cancel: &AtomicBool, progress: &dyn Fn(String)) -> Result<GraphData> {
        let _timing = crate::metrics::stage("analysis");
        self.prepare(cancel, progress)?;
        self.extract(cancel, progress)
    }

    pub(super) fn into_graph(
        mut self,
        cancel: &AtomicBool,
        progress: &dyn Fn(String),
    ) -> Result<GraphData> {
        let _timing = crate::metrics::stage("analysis");
        self.prepare(cancel, progress)?;
        let result = self.extract_inner(cancel, progress, true);
        self.record_result(&result);
        result
    }

    fn record_result<T>(&mut self, result: &Result<T>) {
        self.failed |= result.is_err();
        self.directory.remove_on_drop = !self.failed;
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
            let context = self.context(&self.jobs[index].path, cancel, progress);
            if let Err(error) = self.jobs[index].verify(&context, mode) {
                // Cancellation interrupts verification; it does not prove the
                // retained compiler state invalid.
                check_cancel(cancel)?;
                if !error.is::<crate::fingerprint::InputsChanged>() {
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
            if self.jobs[index].preparation.is_some() || self.jobs[index].build.language != language
            {
                continue;
            }
            self.begin(index)?;
            let context = self.context(&self.jobs[index].path, cancel, progress);
            let mut preparation = match self.jobs[index].build.language.as_str() {
                "csharp" => {
                    Preparation::Csharp(csharp::prepare(&self.jobs[index].build, &context)?)
                }
                "typescript" => {
                    Preparation::TypeScript(typescript::prepare(&self.jobs[index].build, &context)?)
                }
                other => bail!("Unsupported language: {other}"),
            };
            if self.jobs.len() > 1
                && let Preparation::TypeScript(prepared) = &mut preparation
            {
                prepared.release_worker();
            }
            self.jobs[index].preparation = Some(preparation);
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
            self.begin(index)?;
            let context = self.context(&self.jobs[index].path, cancel, progress);
            let preflight = java::prepare(&self.jobs[index].build, &context)?;
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
            let prepared = java::compile(&self.jobs[index].build, &context, preflight)?;
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
    ) -> Result<GraphData> {
        let _timing = crate::metrics::stage("extraction");
        check_cancel(cancel)?;
        self.clear_extraction_cache()?;
        self.validate(cancel, progress, ValidationMode::BeforeExtraction)?;
        ensure!(
            self.jobs.iter().all(|job| job.preparation.is_some()),
            "Prepare analysis before extraction"
        );
        for job in &mut self.jobs {
            check_cancel(cancel)?;
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
                .with_context(|| format!("Analysis failed. Logs: {}", job.path.display()))?;
        }
        // Even cached raw graphs require fresh semantic import membership before
        // the aggregate can leave the engine-owned preparation boundary.
        self.validate(cancel, progress, ValidationMode::RequireCurrent)?;
        let mut composition = compose::GraphComposition::default();
        let mut versions = serde_json::Map::new();
        for job in &mut self.jobs {
            let graph = if consume {
                job.graph.take().unwrap()
            } else {
                job.graph.as_ref().unwrap().clone()
            };
            if !graph.engine_versions.is_empty() {
                versions.insert(
                    job.build.path.to_string_lossy().into_owned(),
                    serde_json::from_str(&graph.engine_versions)?,
                );
            }
            composition.append(graph)?;
        }
        let mut graph = composition.finish();
        ensure_unchanged(&graph.fingerprints)?;
        let mut engines: serde_json::Value = serde_json::from_str(&engine_versions()?)?;
        if !versions.is_empty() {
            engines["projects"] = serde_json::Value::Object(versions);
        }
        graph.engine_versions = engines.to_string();
        graph.validate()?;
        check_cancel(cancel)?;
        crate::metrics::count("symbols", graph.nodes.len() as u64);
        crate::metrics::count("relationships", graph.edges.len() as u64);
        Ok(graph)
    }
}
