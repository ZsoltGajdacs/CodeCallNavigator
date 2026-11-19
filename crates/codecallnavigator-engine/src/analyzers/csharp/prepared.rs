//! Retain compiler output and its observed inputs independently of normalization.
//! Private adapter operations guard captured bytes. The owning analysis session
//! evaluates fresh import membership at reuse and completed preparation/extraction
//! boundaries, so helpers must not repeat full MSBuild evaluation internally.
use super::*;
use crate::analyzers::build_worker_count;
use crate::model::FileFingerprint;
use std::{collections::VecDeque, sync::atomic::AtomicBool};

pub(in crate::analyzers) struct PreparedCsharp {
    output: PathBuf,
    output_before: Vec<FileFingerprint>,
    before_metadata: Vec<FileFingerprint>,
    before_engine: Vec<FileFingerprint>,
    sources: CsharpSources,
    dotnet: PathBuf,
    job: PathBuf,
}

impl PreparedCsharp {
    pub(in crate::analyzers) fn verify(&self) -> Result<()> {
        ensure_unchanged(&self.before_metadata)?;
        ensure_unchanged(&self.before_engine)?;
        ensure_unchanged(&self.output_before)?;
        for snapshot in &self.sources.imports {
            snapshot.verify()?;
        }
        Ok(())
    }

    /// Import membership needs fresh MSBuild evaluation, even for cached graphs.
    pub(in crate::analyzers) fn verify_context(
        &self,
        build: &BuildTarget,
        context: &AnalysisJob<'_>,
    ) -> Result<()> {
        crate::analyzers::process::check_cancel(context.cancel)?;
        self.verify()?;
        let preprocessor =
            inputs::Preprocessor::for_build(&self.dotnet, &self.job, context.cancel, build);
        self.sources.verify_imports(&preprocessor)?;
        self.verify()
    }
}

pub(in crate::analyzers) fn prepare(
    build: &BuildTarget,
    context: &AnalysisJob<'_>,
) -> Result<PreparedCsharp> {
    let dotnet = find_tool("dotnet")
        .context("Install the .NET SDK required by this solution; dotnet was not found")?;
    restore(build, context, &dotnet)?;
    let root = build.path.parent().context("Build has no parent folder")?;
    let before_metadata = source_snapshot(&[root.into()])?;
    let sources = {
        let _timer = crate::metrics::stage("csharp.metadata");
        csharp_sources(build, &dotnet, context.path, context.cancel)?
    };
    ensure_unchanged(&before_metadata)?;
    let before_engine = compiler_input_snapshot(&sources.inputs, root)?;
    let output = context.path.join("callatlas.json");
    compiler_map(build, context, &dotnet, &output)?;
    // Validate compiler success before accepting a reusable preparation.
    let map: AtlasMap = serde_json::from_slice(&fs::read(&output)?)?;
    validate_atlas(&map)?;
    let prepared = PreparedCsharp {
        output_before: crate::fingerprint::capture([output.clone()])?,
        output,
        before_metadata,
        before_engine,
        sources,
        dotnet,
        job: context.path.into(),
    };
    prepared.verify()?;
    Ok(prepared)
}

pub(in crate::analyzers) fn extract(
    build: &BuildTarget,
    context: &AnalysisJob<'_>,
    prepared: &PreparedCsharp,
) -> Result<GraphData> {
    crate::analyzers::process::check_cancel(context.cancel)?;
    prepared.verify()?;
    let map: AtlasMap = serde_json::from_slice(&fs::read(&prepared.output)?)?;
    let root = build.path.parent().context("Build has no parent folder")?;
    let mut graph = {
        let _timer = crate::metrics::stage("csharp.normalization");
        normalize_atlas(build, root, map, prepared.sources.owners.resolved())?
    };
    graph.fingerprints = prepared.before_engine.clone();
    crate::analyzers::process::check_cancel(context.cancel)?;
    prepared.verify()?;
    Ok(graph)
}

fn restore(build: &BuildTarget, context: &AnalysisJob<'_>, dotnet: &Path) -> Result<()> {
    (context.progress)(format!("Restoring {}…", build.label));
    let _timer = crate::metrics::stage("csharp.restore");
    run_process(
        dotnet_command(dotnet)
            .arg("restore")
            .arg(&build.path)
            .arg("--nologo")
            .env("DOTNET_PROCESSOR_COUNT", build_worker_count().to_string())
            .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1"),
        context.path,
        "restore",
        context.cancel,
    )?;
    Ok(())
}

fn compiler_map(
    build: &BuildTarget,
    context: &AnalysisJob<'_>,
    dotnet: &Path,
    output: &Path,
) -> Result<()> {
    (context.progress)(format!("Reading compiler call graph for {}…", build.label));
    let _timer = crate::metrics::stage("csharp.engine");
    run_process(
        dotnet_command(dotnet)
            .arg(context.runtime.callatlas()?)
            .arg("map")
            .arg(&build.path)
            .arg("-o")
            .arg(output)
            .env("DOTNET_PROCESSOR_COUNT", build_worker_count().to_string()),
        context.path,
        "callatlas",
        context.cancel,
    )?;
    Ok(())
}

fn compiler_input_snapshot(
    paths: &[PathBuf],
    root: &Path,
) -> Result<Vec<crate::model::FileFingerprint>> {
    let mut roots = paths
        .iter()
        .cloned()
        .map(crate::fingerprint::ScanRoot::Required)
        .collect::<Vec<_>>();
    roots.push(crate::fingerprint::ScanRoot::Required(root.into()));
    crate::fingerprint::capture_excluding(
        crate::fingerprint::collect_inputs(&roots, &[], true)?,
        &[],
    )
}

fn csharp_projects(
    build: &BuildTarget,
    dotnet: &Path,
    job: &Path,
    cancel: &AtomicBool,
) -> Result<Vec<PathBuf>> {
    if build.kind != "solution" {
        return Ok(vec![build.path.clone()]);
    }
    let output = run_process(
        dotnet_command(dotnet)
            .arg("sln")
            .arg(&build.path)
            .arg("list"),
        job,
        "solution-projects",
        cancel,
    )?;
    let parent = build
        .path
        .parent()
        .context("Solution has no parent folder")?;
    let projects = fs::read_to_string(output)?
        .lines()
        .map(str::trim)
        .filter(|line| line.ends_with(".csproj"))
        .map(|line| parent.join(line.replace('\\', "/")))
        .collect::<Vec<_>>();
    ensure!(!projects.is_empty(), "Solution contains no C# projects");
    Ok(projects)
}

fn csharp_sources(
    build: &BuildTarget,
    dotnet: &Path,
    job: &Path,
    cancel: &AtomicBool,
) -> Result<CsharpSources> {
    let mut sources = CsharpSources {
        queue: VecDeque::from(csharp_projects(build, dotnet, job, cancel)?),
        owners: ownership::Owners::default(),
        assembly_names: HashMap::new(),
        inputs: vec![build.path.clone()],
        imports: Vec::new(),
    };
    let preprocessor = inputs::Preprocessor::for_build(dotnet, job, cancel, build);
    let mut visited = HashSet::new();
    while let Some(project) = sources.queue.pop_front() {
        let project = project.canonicalize()?;
        if !visited.insert(project.clone()) {
            continue;
        }
        let observed = preprocessor.capture(&project, visited.len())?;
        sources.record_metadata(&project, &observed, &preprocessor)?;
        preprocessor.verify(&observed)?;
        sources.inputs.extend(observed.paths.iter().cloned());
        sources.imports.push(observed);
    }
    sources.verify_imports(&preprocessor)?;
    Ok(sources)
}

struct CsharpSources {
    assembly_names: HashMap<String, PathBuf>,
    queue: VecDeque<PathBuf>,
    owners: ownership::Owners,
    inputs: Vec<PathBuf>,
    imports: Vec<inputs::Snapshot>,
}

impl CsharpSources {
    fn record_metadata(
        &mut self,
        project: &Path,
        snapshot: &inputs::Snapshot,
        preprocessor: &inputs::Preprocessor<'_>,
    ) -> Result<()> {
        for (index, context) in snapshot.contexts().iter().enumerate() {
            let metadata = preprocessor.metadata(
                project,
                context,
                &format!("msbuild-{}-{index}", snapshot.sequence()),
            )?;
            let origin = if context.is_configured_normal() {
                ownership::Origin::ConfiguredNormal
            } else {
                ownership::Origin::ContextFallback
            };
            record_csharp_items(project, &metadata, origin, self)?;
        }
        Ok(())
    }

    fn verify_imports(&self, preprocessor: &inputs::Preprocessor<'_>) -> Result<()> {
        for snapshot in &self.imports {
            preprocessor.verify(snapshot)?;
        }
        Ok(())
    }
}

fn record_csharp_items(
    project: &Path,
    metadata: &serde_json::Value,
    origin: ownership::Origin,
    sources: &mut CsharpSources,
) -> Result<()> {
    record_project_properties(project, metadata, sources)?;
    let project_name = project
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let is_test = metadata
        .pointer("/Properties/IsTestProject")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .eq_ignore_ascii_case("true");
    let items = metadata
        .pointer("/Items/Compile")
        .and_then(|value| value.as_array())
        .context("MSBuild returned no Compile item list")?;
    for item in items {
        let path = PathBuf::from(
            item["FullPath"]
                .as_str()
                .context("Compile item has no FullPath")?,
        );
        let owner = SourceOwner {
            project: project_name.clone(),
            is_test,
            is_generated: item["AutoGen"]
                .as_str()
                .unwrap_or("")
                .eq_ignore_ascii_case("true"),
        };
        let assembly = metadata["Properties"]["AssemblyName"]
            .as_str()
            .context("MSBuild omitted AssemblyName")?;
        sources.owners.record(
            ownership::Claim {
                file: path.clone(),
                assembly: assembly.into(),
                owner,
            },
            origin,
        )?;
        if let Some(root) = compile_source_root(item, &path) {
            sources.inputs.push(root);
        }
        sources.inputs.push(path);
    }
    sources.inputs.push(project.into());
    if let Some(references) = metadata
        .pointer("/Items/ProjectReference")
        .and_then(|value| value.as_array())
    {
        for reference in references {
            if let Some(path) = reference["FullPath"].as_str() {
                sources.queue.push_back(path.into());
            }
        }
    }
    Ok(())
}

fn compile_source_root(item: &serde_json::Value, path: &Path) -> Option<PathBuf> {
    let recursive = item["RecursiveDir"]
        .as_str()
        .unwrap_or("")
        .replace('\\', "/");
    let mut root = path.parent()?;
    for _ in Path::new(&recursive).components() {
        root = root.parent()?;
    }
    Some(root.into())
}

fn record_project_properties(
    project: &Path,
    metadata: &serde_json::Value,
    sources: &mut CsharpSources,
) -> Result<()> {
    let assembly = metadata
        .pointer("/Properties/AssemblyName")
        .and_then(|value| value.as_str())
        .context("MSBuild omitted AssemblyName")?;
    if let Some(previous) = sources
        .assembly_names
        .insert(assembly.into(), project.into())
    {
        ensure!(
            previous == project,
            "Projects {} and {} share AssemblyName {assembly}; their analyzer identities cannot be distinguished",
            previous.display(),
            project.display()
        );
    }
    if let Some(additional) = metadata
        .pointer("/Items/AdditionalFiles")
        .and_then(|value| value.as_array())
    {
        sources.inputs.extend(
            additional
                .iter()
                .filter_map(|item| item["FullPath"].as_str())
                .map(PathBuf::from)
                .filter(|path| path.is_file()),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linked_glob_root_detects_new_files_outside_the_project_directory() {
        let directory = tempfile::tempdir().unwrap();
        let core = directory.path().join("core");
        fs::create_dir_all(core.join("campaign")).unwrap();
        let file = core.join("campaign/Existing.cs");
        fs::write(&file, "class Existing {}").unwrap();
        let item = serde_json::json!({"RecursiveDir": "campaign/"});
        let derived = compile_source_root(&item, &file).unwrap();
        assert_eq!(derived, core);
        let before = source_snapshot(&[derived]).unwrap();
        fs::create_dir_all(core.join("new-domain")).unwrap();
        fs::write(core.join("new-domain/Added.cs"), "class Added {}").unwrap();
        assert!(ensure_unchanged(&before).is_err());
    }
}
