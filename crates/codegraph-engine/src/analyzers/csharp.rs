mod architecture;
mod framework;
mod policy;
use super::process::run_process;
use super::{AnalysisJob, ensure_unchanged, find_tool, source_snapshot};
use crate::{
    discovery::BuildTarget,
    model::{Edge, EntryPoints, GraphData, Node, Visibility},
};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
};

#[derive(Deserialize)]
struct AtlasMap {
    schema: String,
    #[serde(rename = "declarationFactsVersion")]
    declaration_facts_version: String,
    #[serde(default, rename = "frameworkFactsVersion")]
    framework_facts_version: String,
    #[serde(default, rename = "frameworkEdges")]
    framework_edges: Option<Vec<framework::DispatchFact>>,
    #[serde(default, rename = "frameworkEntries")]
    framework_entries: Option<Vec<String>>,
    #[serde(default, rename = "frameworkDiagnostics")]
    framework_diagnostics: Option<Vec<String>>,
    #[serde(default, rename = "architectureFiles")]
    architecture_files: Option<Vec<architecture::AtlasFileFacts>>,
    nodes: Vec<AtlasNode>,
    edges: Vec<AtlasEdge>,
    #[serde(default)]
    diagnostics: Vec<serde_json::Value>,
    #[serde(default, rename = "bodyErrorCount")]
    body_errors: u64,
    #[serde(default)]
    unresolved: Vec<serde_json::Value>,
    #[serde(default, rename = "entryPoints")]
    entry_points: Option<AtlasEntries>,
}

impl AtlasMap {
    fn take_framework_facts(&mut self) -> Result<framework::Facts> {
        Ok(framework::Facts {
            edges: self
                .framework_edges
                .take()
                .context("MediatR facts are missing")?,
            entries: self
                .framework_entries
                .take()
                .context("MediatR entries are missing")?,
            diagnostics: self
                .framework_diagnostics
                .take()
                .context("MediatR diagnostics are missing")?,
        })
    }
}

#[derive(Deserialize)]
struct AtlasEntries {
    candidates: Vec<String>,
    #[serde(rename = "applicationStarts")]
    application_starts: Vec<String>,
}
#[derive(Deserialize)]
struct AtlasNode {
    id: String,
    kind: String,
    container: String,
    name: String,
    location: String,
    declaration: Option<DeclarationFacts>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeclarationFacts {
    has_source_declaration: bool,
    is_implicit: bool,
    callable_kind: String,
    is_object_member: bool,
    #[serde(default)]
    visibility: Option<Visibility>,
    #[serde(default)]
    container_visibility: Option<Visibility>,
}
#[derive(Deserialize)]
struct AtlasEdge {
    from: String,
    to: String,
    kind: String,
    count: u32,
    #[serde(default)]
    tier: String,
    #[serde(default, rename = "virtualTargets")]
    virtual_targets: Vec<String>,
}
type SourceOwners = HashMap<PathBuf, HashMap<String, SourceOwner>>;

#[derive(Clone)]
struct SourceOwner {
    project: String,
    is_test: bool,
    is_generated: bool,
}

pub(super) fn analyze_csharp(build: &BuildTarget, context: &AnalysisJob<'_>) -> Result<GraphData> {
    let AnalysisJob {
        runtime,
        path: job,
        cancel,
        progress,
        ..
    } = context;
    let dotnet = find_tool("dotnet")
        .context("Install the .NET SDK required by this solution; dotnet was not found")?;
    progress(format!("Restoring {}…", build.label));
    {
        let _timer = crate::metrics::stage("csharp.restore");
        run_process(
            Command::new(&dotnet)
                .arg("restore")
                .arg(&build.path)
                .arg("--nologo")
                .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1"),
            job,
            "restore",
            cancel,
        )?;
    }
    let root = build.path.parent().context("Build has no parent folder")?;
    let before_metadata = source_snapshot(&[root.into()])?;
    let (owners, mut inputs) = {
        let _timer = crate::metrics::stage("csharp.metadata");
        csharp_sources(build, &dotnet, job, cancel)?
    };
    ensure_unchanged(&before_metadata)?;
    inputs.push(root.into());
    let before_engine = source_snapshot(&inputs)?;
    let output = job.join("callatlas.json");
    progress(format!("Reading compiler call graph for {}…", build.label));
    {
        let _timer = crate::metrics::stage("csharp.engine");
        run_process(
            Command::new(&dotnet)
                .arg(runtime.callatlas()?)
                .arg("map")
                .arg(&build.path)
                .arg("-o")
                .arg(&output),
            job,
            "callatlas",
            cancel,
        )?;
    }
    let map: AtlasMap = serde_json::from_slice(&fs::read(output)?)?;
    ensure_unchanged(&before_metadata)?;
    ensure_unchanged(&before_engine)?;
    let mut graph = {
        let _timer = crate::metrics::stage("csharp.normalization");
        normalize_atlas(build, root, map, &owners)?
    };
    graph.fingerprints = before_engine;
    Ok(graph)
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
        Command::new(dotnet).arg("sln").arg(&build.path).arg("list"),
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
) -> Result<(SourceOwners, Vec<PathBuf>)> {
    let mut sources = CsharpSources {
        queue: VecDeque::from(csharp_projects(build, dotnet, job, cancel)?),
        owners: HashMap::new(),
        assembly_names: HashMap::new(),
        inputs: vec![build.path.clone()],
    };
    let mut visited = HashSet::new();
    while let Some(project) = sources.queue.pop_front() {
        let project = project.canonicalize()?;
        if !visited.insert(project.clone()) {
            continue;
        }
        let output = run_process(
            Command::new(dotnet).arg("msbuild").arg(&project).args([
                "-getItem:Compile,ProjectReference,AdditionalFiles",
                "-getProperty:MSBuildProjectFullPath,IsTestProject,MSBuildAllProjects,AssemblyName",
                "-nologo",
            ]),
            job,
            &format!("msbuild-{}", visited.len()),
            cancel,
        )?;
        let metadata: serde_json::Value = serde_json::from_slice(&fs::read(output)?)?;
        record_csharp_items(&project, &metadata, &mut sources)?;
    }
    Ok((sources.owners, sources.inputs))
}

struct CsharpSources {
    assembly_names: HashMap<String, PathBuf>,
    queue: VecDeque<PathBuf>,
    owners: SourceOwners,
    inputs: Vec<PathBuf>,
}

fn record_csharp_items(
    project: &Path,
    metadata: &serde_json::Value,
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
        sources
            .owners
            .entry(path.clone())
            .or_default()
            .insert(assembly.into(), owner);
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
    if let Some(imports) = metadata
        .pointer("/Properties/MSBuildAllProjects")
        .and_then(|value| value.as_str())
    {
        sources.inputs.extend(
            imports
                .split(';')
                .map(PathBuf::from)
                .filter(|path| path.is_file()),
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

fn normalize_atlas(
    build: &BuildTarget,
    root: &Path,
    mut map: AtlasMap,
    owners: &SourceOwners,
) -> Result<GraphData> {
    validate_atlas(&map)?;
    let framework = map.take_framework_facts()?;
    let mut graph = GraphData {
        roots: vec![build.path.to_string_lossy().into_owned()],
        ..GraphData::default()
    };
    graph.architecture = architecture::normalize(build, root, map.architecture_files, owners);
    graph.entry_points = map.entry_points.map(|entries| {
        let qualify = |ids: Vec<String>| {
            ids.into_iter()
                .map(|id| format!("{}|{id}", build.path.display()))
                .collect()
        };
        EntryPoints {
            framework_entries: Vec::new(),
            candidates: qualify(entries.candidates),
            application_starts: qualify(entries.application_starts),
        }
    });
    let mut indices = HashMap::new();
    let mut excluded = HashSet::new();
    for node in map.nodes {
        ensure!(
            qualified_identity(&node.id),
            "CallAtlas is missing the pinned assembly identity patch; run tools/bootstrap.py before analyzing solutions"
        );
        ensure!(
            node.declaration.is_some()
                || matches!(node.kind.as_str(), "fieldinit" | "field" | "delegate"),
            "CallAtlas omitted declaration facts for {}",
            node.id
        );
        let index = u32::try_from(graph.nodes.len())?;
        ensure!(
            indices.insert(node.id.clone(), index).is_none(),
            "Duplicate CallAtlas identity: {}",
            node.id
        );
        if policy::excluded(&node) {
            excluded.insert(format!("{}|{}", build.path.display(), node.id));
        }
        graph.nodes.push(atlas_node(build, root, node, owners));
    }
    for edge in map.edges {
        append_atlas_edge(&mut graph, &indices, edge)?;
    }
    framework::append(root, &mut graph, &indices, framework)?;
    graph.diagnostics.push("CallAtlas omits most source-generator output; cross-assembly interface implementation candidates are limited; operator invocations are not fully reported. Reflection, dynamic calls, unsupported framework callbacks and unsupported DI wiring are not inferred. MediatR targets are compiler-visible possibilities, not proof of dependency-injection registration.".into());
    if !map.unresolved.is_empty() {
        graph.diagnostics.push(format!(
            "CallAtlas reported {} unresolved call sites",
            map.unresolved.len()
        ));
    }
    graph.validate()?;
    crate::method_policy::prune(&mut graph, &excluded)?;
    policy::finalize_entries(&mut graph);
    graph.validate()?;
    Ok(graph)
}

fn validate_atlas(map: &AtlasMap) -> Result<()> {
    ensure!(
        map.schema == "callatlas.ir.v3",
        "Unsupported CallAtlas schema: {}",
        map.schema
    );
    ensure!(
        map.declaration_facts_version == "2",
        "CallAtlas declaration facts are missing or unsupported; run tools/bootstrap.py"
    );
    ensure!(
        map.framework_facts_version == "1"
            && map.framework_edges.is_some()
            && map.framework_entries.is_some()
            && map.framework_diagnostics.is_some(),
        "CallAtlas framework facts are missing or unsupported; run tools/bootstrap.py and refresh the analysis"
    );
    ensure!(
        map.body_errors == 0 && map.diagnostics.is_empty(),
        "CallAtlas reported incomplete compilation"
    );
    Ok(())
}

fn qualified_identity(identity: &str) -> bool {
    identity
        .strip_prefix('[')
        .and_then(|value| value.split_once("]::"))
        .is_some()
}

fn atlas_node(build: &BuildTarget, root: &Path, node: AtlasNode, owners: &SourceOwners) -> Node {
    let (file, line) = location(&node.location, root);
    let owner = owners.get(Path::new(&file)).and_then(|owners| {
        owners
            .iter()
            .find(|(assembly, _)| node.id.starts_with(&format!("[{assembly}]::")))
            .map(|(_, owner)| owner)
    });
    Node {
        id: format!("{}|{}", build.path.display(), node.id),
        build: build.path.to_string_lossy().into_owned(),
        project: owner
            .map(|owner| owner.project.clone())
            .unwrap_or_else(|| "Unlocated compiler symbols".into()),
        language: "csharp".into(),
        visibility: node.declaration.as_ref().and_then(|facts| facts.visibility),
        container_visibility: node
            .declaration
            .as_ref()
            .and_then(|facts| facts.container_visibility),
        kind: node.kind,
        label: node.name,
        signature: node.id,
        namespace: node
            .container
            .rsplit_once('.')
            .map(|(namespace, _)| namespace.to_owned())
            .unwrap_or_default(),
        container: node.container,
        body_unavailable: file.is_empty(),
        file,
        line,
        is_test: owner.is_some_and(|owner| owner.is_test),
        is_generated: owner.is_some_and(|owner| owner.is_generated),
    }
}

fn location(value: &str, root: &Path) -> (String, u32) {
    let Some((file, line)) = value.rsplit_once(':') else {
        return (String::new(), 0);
    };
    let Ok(line) = line.parse() else {
        return (String::new(), 0);
    };
    let path = root.join(file);
    (
        path.canonicalize()
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned(),
        line,
    )
}

fn append_atlas_edge(
    graph: &mut GraphData,
    indices: &HashMap<String, u32>,
    edge: AtlasEdge,
) -> Result<()> {
    let from = *indices
        .get(&edge.from)
        .context("CallAtlas caller is missing")?;
    let to = *indices
        .get(&edge.to)
        .context("CallAtlas target is missing")?;
    let certainty = if edge.tier.is_empty() {
        "analyzer-reported".into()
    } else {
        edge.tier
    };
    graph.edges.push(Edge {
        evidence: Vec::new(),
        provenance: String::new(),
        from,
        to,
        kind: edge.kind.clone(),
        certainty,
        count: edge.count,
    });
    for candidate in edge.virtual_targets {
        if let Some(target) = indices.get(&candidate) {
            graph.edges.push(Edge {
                evidence: Vec::new(),
                provenance: String::new(),
                from,
                to: *target,
                kind: edge.kind.clone(),
                certainty: "possible".into(),
                count: edge.count,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_or_incomplete_framework_capabilities_require_refresh() {
        let mut value = serde_json::json!({
            "schema":"callatlas.ir.v3", "declarationFactsVersion":"2", "nodes":[], "edges":[]
        });
        let stale: AtlasMap = serde_json::from_value(value.clone()).unwrap();
        assert!(
            validate_atlas(&stale)
                .unwrap_err()
                .to_string()
                .contains("refresh")
        );
        value["frameworkFactsVersion"] = "1".into();
        let incomplete: AtlasMap = serde_json::from_value(value.clone()).unwrap();
        assert!(validate_atlas(&incomplete).is_err());
        for key in ["frameworkEdges", "frameworkEntries", "frameworkDiagnostics"] {
            value[key] = serde_json::json!([]);
        }
        let current: AtlasMap = serde_json::from_value(value.clone()).unwrap();
        validate_atlas(&current).unwrap();
        value["frameworkFactsVersion"] = "2".into();
        let incompatible: AtlasMap = serde_json::from_value(value).unwrap();
        assert!(validate_atlas(&incompatible).is_err());
    }

    #[test]
    fn unpatched_symbol_identities_are_rejected() {
        assert!(!qualified_identity("Program::<top-level>"));
        assert!(qualified_identity("[Application]::Program::<top-level>"));
    }
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

#[cfg(test)]
mod acceptance {
    use super::*;
    #[test]
    #[ignore = "requires pinned CallAtlas and installed .NET SDK"]
    fn real_roslyn_folder_declarations_are_pruned_and_entries_recomputed() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("Program.cs");
        fs::write(
            &source,
            include_str!("../../../../fixtures/dotnet-policy/Program.cs"),
        )
        .unwrap();
        let runtime = super::super::AnalyzerRuntime::discover().unwrap();
        let output = root.path().join("graph.json");
        run_process(
            Command::new(find_tool("dotnet").unwrap())
                .arg(runtime.callatlas().unwrap())
                .arg("map")
                .arg(root.path())
                .arg("-o")
                .arg(&output),
            root.path(),
            "callatlas",
            &AtomicBool::new(false),
        )
        .unwrap();
        let map = serde_json::from_slice(&fs::read(output).unwrap()).unwrap();
        let build = BuildTarget {
            path: root.path().into(),
            language: "csharp".into(),
            kind: "project".into(),
            label: "policy".into(),
        };
        let owners = HashMap::from([(
            source,
            HashMap::from([(
                "callatlas.snapshot".into(),
                SourceOwner {
                    project: "policy".into(),
                    is_test: false,
                    is_generated: false,
                },
            )]),
        )]);
        let graph = normalize_atlas(&build, root.path(), map, &owners).unwrap();
        assert_policy_graph(&graph);
    }

    fn assert_policy_graph(graph: &GraphData) {
        assert!(graph.nodes.iter().all(|node| !matches!(
            node.kind.as_str(),
            "constructor" | "cctor" | "fieldinit" | "field" | "delegate"
        )));
        for suffix in [
            "IObjectLike::ToString(System.Int32)",
            "Explicit::IObjectLike.ToString(System.Int32)",
            "Entry::Authored()",
            "Worker::Private()",
            "Worker::Package()",
            "Worker::Protected()",
            "Worker::ToString(System.Int32)",
            "Worker::GetHashCode(System.Int32)",
            "Worker::Equals(System.Int32)",
            "Worker::get_Value()",
            "Worker::get_Automatic()",
            "Worker::set_Automatic(System.Int32)",
        ] {
            assert!(
                graph.nodes.iter().any(|node| node.id.ends_with(suffix)),
                "Missing {suffix}"
            );
        }
        for suffix in [
            "IObjectLike::ToString()",
            "IObjectLike::GetHashCode()",
            "IObjectLike::Equals(System.Object)",
            "Explicit::IObjectLike.ToString()",
            "Explicit::IObjectLike.GetHashCode()",
            "Explicit::IObjectLike.Equals(System.Object)",
            "Entry::get_Number()",
            "Entry::Equals(Entry)",
            "Entry::ToString()",
            "Entry::GetHashCode()",
            "Worker::ToString()",
            "Worker::GetHashCode()",
            "Worker::Equals(System.Object)",
        ] {
            assert!(
                !graph.nodes.iter().any(|node| node.id.ends_with(suffix)),
                "Unexpected {suffix}"
            );
        }
        let only = graph
            .nodes
            .iter()
            .find(|node| node.id.ends_with("Worker::ConstructorOnly()"))
            .unwrap();
        assert!(
            graph
                .entry_points
                .as_ref()
                .unwrap()
                .candidates
                .contains(&only.id)
        );
    }
}
