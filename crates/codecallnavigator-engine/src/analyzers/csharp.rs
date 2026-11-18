mod prepared;
pub(super) use prepared::{PreparedCsharp, extract, prepare};
mod architecture;
mod flow_inventory;
mod framework;
mod http;
mod inputs;
mod ownership;
mod policy;
mod source;
use super::process::run_process;
use super::{AnalysisJob, ensure_unchanged, find_tool, source_snapshot};
use crate::{
    discovery::BuildTarget,
    model::{Edge, EntryPoints, GraphData, Node, Visibility},
};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
#[cfg(test)]
use std::sync::atomic::AtomicBool;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn dotnet_command(dotnet: &Path) -> Command {
    let mut command = Command::new(dotnet);
    // These invocations own their build processes. Do not start reusable
    // servers that may outlive the invocation and serve unrelated clients.
    command.envs([
        ("DOTNET_CLI_USE_MSBUILD_SERVER", "0"),
        ("MSBUILDDISABLENODEREUSE", "1"),
        ("UseSharedCompilation", "false"),
    ]);
    command
}

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
    #[serde(default, rename = "httpFactsVersion")]
    http_facts_version: String,
    #[serde(default, rename = "httpRoutes")]
    http_routes: Option<Vec<http::RawRoute>>,
    #[serde(default, rename = "architectureFiles")]
    architecture_files: Option<Vec<architecture::AtlasFileFacts>>,
    #[serde(default, rename = "flowFactsVersion")]
    flow_facts_version: String,
    #[serde(default, rename = "flowInventory")]
    flow_inventory: Option<crate::flow::FlowInventory>,
    #[serde(default, rename = "sourceFactsVersion")]
    source_facts_version: String,
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
impl AtlasEntries {
    fn normalize(self, build: &BuildTarget) -> EntryPoints {
        let qualify = |ids: Vec<String>| {
            ids.into_iter()
                .map(|id| format!("{}|{id}", build.path.display()))
                .collect()
        };
        EntryPoints {
            framework_entries: Vec::new(),
            candidates: qualify(self.candidates),
            application_starts: qualify(self.application_starts),
        }
    }
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct SourceOwner {
    project: String,
    is_test: bool,
    is_generated: bool,
}

fn normalize_atlas(
    build: &BuildTarget,
    root: &Path,
    mut map: AtlasMap,
    owners: &SourceOwners,
) -> Result<GraphData> {
    validate_atlas(&map)?;
    let framework = map.take_framework_facts()?;
    let http = map.http_routes.take().context("HTTP routes are missing")?;
    let mut graph = GraphData {
        roots: vec![build.path.to_string_lossy().into_owned()],
        ..GraphData::default()
    };
    let mut snapshots = source::Snapshots::default();
    graph.architecture = architecture::normalize(
        map.architecture_files,
        architecture::Context {
            build,
            root,
            owners,
            snapshots: &mut snapshots,
        },
    )?;
    let flow = map
        .flow_inventory
        .take()
        .context("C# function flow inventory is missing")?;
    graph.entry_points = map.entry_points.map(|entries| entries.normalize(build));
    let bindings = append_nodes(
        DeclarationContext {
            build,
            root,
            owners,
        },
        &mut graph,
        map.nodes,
    )?;
    for edge in map.edges {
        append_atlas_edge(&mut graph, &bindings.indices, edge)?;
    }
    framework::append(root, &mut graph, &bindings.indices, framework)?;
    http::append(build, root, &mut graph, http)?;
    flow_inventory::append(
        &mut graph,
        flow,
        flow_inventory::BindingContext {
            root,
            indices: &bindings.indices,
            snapshots: &snapshots,
            owners,
        },
    )?;
    graph.diagnostics.push("CallAtlas omits most source-generator output; cross-assembly interface implementation candidates are limited; operator invocations are not fully reported. Reflection, dynamic calls, unsupported framework callbacks and unsupported DI wiring are not inferred. MediatR targets are compiler-visible possibilities, not proof of dependency-injection registration.".into());
    if !map.unresolved.is_empty() {
        graph.diagnostics.push(format!(
            "CallAtlas reported {} unresolved call sites",
            map.unresolved.len()
        ));
    }
    graph.validate()?;
    crate::method_policy::prune(&mut graph, &bindings.excluded)?;
    policy::finalize_entries(&mut graph);
    graph.validate()?;
    Ok(graph)
}

struct DeclarationContext<'a> {
    build: &'a BuildTarget,
    root: &'a Path,
    owners: &'a SourceOwners,
}
struct NodeBindings {
    indices: HashMap<String, u32>,
    excluded: HashSet<String>,
}
fn append_nodes(
    context: DeclarationContext<'_>,
    graph: &mut GraphData,
    nodes: Vec<AtlasNode>,
) -> Result<NodeBindings> {
    let mut indices = HashMap::new();
    let mut excluded = HashSet::new();
    for node in nodes {
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
            excluded.insert(format!("{}|{}", context.build.path.display(), node.id));
        }
        graph.nodes.push(atlas_node(
            context.build,
            context.root,
            node,
            context.owners,
        ));
    }
    Ok(NodeBindings { indices, excluded })
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
        map.http_facts_version == "1" && map.http_routes.is_some(),
        "CallAtlas HTTP facts are missing or unsupported; run tools/bootstrap.py and refresh the analysis"
    );
    ensure!(
        map.body_errors == 0 && map.diagnostics.is_empty(),
        "CallAtlas reported incomplete compilation"
    );
    ensure!(
        map.flow_facts_version == "2"
            && map
                .flow_inventory
                .as_ref()
                .is_some_and(|inventory| inventory.version == crate::flow::INVENTORY_VERSION),
        "CallAtlas function flow facts are missing or unsupported; run tools/bootstrap.py and refresh the analysis"
    );
    ensure!(
        map.source_facts_version == "1" && map.architecture_files.is_some(),
        "CallAtlas source facts are missing or unsupported; run tools/bootstrap.py and refresh the analysis"
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
    let path = source::path(root, file);
    (path.to_string_lossy().into_owned(), line)
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
            "schema":"callatlas.ir.v3", "declarationFactsVersion":"2", "nodes":[], "edges":[],
            "httpFactsVersion":"1", "httpRoutes":[],
            "flowFactsVersion":"2", "flowInventory":{"version":1,"functions":[]}, "sourceFactsVersion":"1", "architectureFiles":[]
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
    fn http_capability_must_be_current_and_complete() {
        let mut value = serde_json::json!({
            "schema":"callatlas.ir.v3", "declarationFactsVersion":"2", "nodes":[], "edges":[],
            "frameworkFactsVersion":"1", "frameworkEdges":[], "frameworkEntries":[], "frameworkDiagnostics":[],
            "flowFactsVersion":"2", "flowInventory":{"version":1,"functions":[]}, "sourceFactsVersion":"1", "architectureFiles":[]
        });
        let missing: AtlasMap = serde_json::from_value(value.clone()).unwrap();
        assert!(
            validate_atlas(&missing)
                .unwrap_err()
                .to_string()
                .contains("HTTP facts")
        );
        value["httpFactsVersion"] = "1".into();
        value["httpRoutes"] = serde_json::json!([]);
        let current: AtlasMap = serde_json::from_value(value.clone()).unwrap();
        validate_atlas(&current).unwrap();
        value["httpRoutes"] = serde_json::Value::Null;
        let incomplete: AtlasMap = serde_json::from_value(value).unwrap();
        assert!(validate_atlas(&incomplete).is_err());
    }

    #[test]
    fn unpatched_symbol_identities_are_rejected() {
        assert!(!qualified_identity("Program::<top-level>"));
        assert!(qualified_identity("[Application]::Program::<top-level>"));
    }
    #[test]
    fn missing_or_incompatible_flow_capabilities_require_analyzer_refresh() {
        let mut value = serde_json::json!({"schema":"callatlas.ir.v3","declarationFactsVersion":"2","nodes":[],"edges":[],
            "frameworkFactsVersion":"1","frameworkEdges":[],"frameworkEntries":[],"frameworkDiagnostics":[],
            "httpFactsVersion":"1","httpRoutes":[],"sourceFactsVersion":"1","architectureFiles":[]});
        let missing: AtlasMap = serde_json::from_value(value.clone()).unwrap();
        assert!(
            validate_atlas(&missing)
                .unwrap_err()
                .to_string()
                .contains("flow")
        );
        value["flowFactsVersion"] = "2".into();
        value["flowInventory"] = serde_json::json!({"version":1,"functions":[]});
        validate_atlas(&serde_json::from_value(value.clone()).unwrap()).unwrap();
        for invalid in [
            serde_json::Value::Null,
            serde_json::json!({"version":2,"functions":[]}),
        ] {
            value["flowInventory"] = invalid;
            assert!(validate_atlas(&serde_json::from_value(value.clone()).unwrap()).is_err());
        }
        value["flowInventory"] = serde_json::json!({"version":1,"functions":[]});
        for version in ["1", "3"] {
            value["flowFactsVersion"] = version.into();
            assert!(validate_atlas(&serde_json::from_value(value.clone()).unwrap()).is_err());
        }
    }
    #[test]
    fn fresh_source_capability_requires_current_version_and_file_inventory() {
        let mut value = serde_json::json!({"schema":"callatlas.ir.v3","declarationFactsVersion":"2","nodes":[],"edges":[],
            "frameworkFactsVersion":"1","frameworkEdges":[],"frameworkEntries":[],"frameworkDiagnostics":[],
            "httpFactsVersion":"1","httpRoutes":[],"flowFactsVersion":"2","flowInventory":{"version":1,"functions":[]},
            "sourceFactsVersion":"1","architectureFiles":[]});
        validate_atlas(&serde_json::from_value(value.clone()).unwrap()).unwrap();
        for version in ["", "2"] {
            value["sourceFactsVersion"] = version.into();
            assert!(
                validate_atlas(&serde_json::from_value(value.clone()).unwrap())
                    .unwrap_err()
                    .to_string()
                    .contains("source facts")
            );
        }
        value["sourceFactsVersion"] = "1".into();
        value["architectureFiles"] = serde_json::Value::Null;
        assert!(validate_atlas(&serde_json::from_value(value).unwrap()).is_err());
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
            dotnet_command(&find_tool("dotnet").unwrap())
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
