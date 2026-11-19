//! Argument types validate wire shape and bounds, never graph meaning.
use crate::wire::validate_generation;
use anyhow::{Result, bail, ensure};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GraphArgs {
    pub expected_generation: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompareArgs {
    pub base: String,
    #[serde(default = "comparison_head")]
    pub head: String,
    pub base_snapshot: Option<String>,
    pub head_snapshot: Option<String>,
    #[serde(default)]
    pub offset: u32,
    #[serde(default = "comparison_limit")]
    #[schemars(range(min = 1, max = 500))]
    pub limit: u32,
    pub expected_comparison: Option<String>,
    pub expected_generation: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FlowComparisonArgs {
    pub base: String,
    #[serde(default = "comparison_head")]
    pub head: String,
    pub base_snapshot: Option<String>,
    pub head_snapshot: Option<String>,
    pub expected_comparison: Option<String>,
    pub selection: FlowComparisonSelection,
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FlowComparisonSelection {
    Catalog {
        #[serde(default)]
        text: String,
        #[serde(default)]
        offset: u32,
        #[serde(default = "flow_catalog_limit")]
        #[schemars(range(min = 1, max = 500))]
        limit: u32,
    },
    Detail {
        symbol: String,
        head_symbol: Option<String>,
    },
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SnapshotStatusArgs {
    pub revision: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PrepareSnapshotArgs {
    pub revision: String,
    /// Project-relative build paths at the requested revision. Empty uses CLI defaults.
    #[serde(default)]
    pub targets: Vec<String>,
}
impl PrepareSnapshotArgs {
    pub fn validate(&self) -> Result<()> {
        text(&self.revision)?;
        ensure!(self.targets.len() <= 1000, "Too many snapshot build paths");
        for target in &self.targets {
            text(target)?;
            ensure!(
                !target.contains('\0')
                    && std::path::Path::new(target)
                        .components()
                        .all(|part| matches!(
                            part,
                            std::path::Component::Normal(_) | std::path::Component::CurDir
                        )),
                "Snapshot build paths must be project-relative"
            );
        }
        Ok(())
    }
}
fn comparison_head() -> String {
    "WORKTREE".into()
}
fn comparison_limit() -> u32 {
    100
}
fn flow_catalog_limit() -> u32 {
    50
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchArgs {
    pub text: String,
    #[serde(default)]
    pub category: Category,
    #[serde(default)]
    pub offset: u32,
    #[serde(default = "search_limit")]
    #[schemars(range(min = 1, max = 200))]
    pub limit: u32,
    pub expected_generation: Option<String>,
}
#[derive(Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    #[default]
    All,
    Classes,
    Methods,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SymbolArgs {
    pub symbol: String,
    pub expected_generation: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceArgs {
    pub symbol: String,
    #[serde(default)]
    pub origin: Origin,
    pub expected_generation: Option<String>,
}
#[derive(Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    #[default]
    Analyzed,
    Live,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DirectoryArgs {
    pub path: String,
    #[serde(default)]
    pub offset: u32,
    #[serde(default = "directory_limit")]
    #[schemars(range(min = 1, max = 1000))]
    pub limit: u32,
    pub expected_generation: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SectionArgs {
    pub selection: Selection,
    pub filter: Option<Filter>,
    pub limits: Option<Limits>,
    pub expected_generation: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NeighborArgs {
    pub symbol: String,
    pub filter: Option<Filter>,
    pub limits: Option<Limits>,
    pub expected_generation: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Empty {}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RefreshArgs {
    /// Omit to use exactly the CLI's default_selection identities.
    pub targets: Option<Vec<BuildIdentity>>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BuildIdentity {
    pub path: String,
    pub kind: String,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Selection {
    Overview,
    EntryPoints {
        direction: Direction,
        #[schemars(range(max = 64))]
        depth: u32,
    },
    Neighborhood {
        symbols: Vec<String>,
        direction: Direction,
        #[schemars(range(max = 64))]
        depth: u32,
    },
    Path {
        from: String,
        to: String,
    },
    File {
        file: String,
    },
    Class {
        build: String,
        project: String,
        container: String,
    },
}
#[derive(Deserialize, Serialize, JsonSchema)]
pub enum Direction {
    Incoming,
    Outgoing,
    Both,
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Filter {
    pub version: u32,
    #[serde(default)]
    pub include_tests: bool,
    #[serde(default)]
    pub include_generated: bool,
    #[serde(default)]
    pub include_unavailable: bool,
    pub relationship_kinds: Vec<Relationship>,
    pub target_resolutions: Vec<Resolution>,
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Relationship {
    Call,
    FunctionReference,
    FrameworkLifecycle,
    HttpNavigation,
    Other,
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    Direct,
    VirtualDispatch,
    Possible,
    Other,
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    #[schemars(range(min = 1, max = 1500))]
    pub nodes: u32,
    #[schemars(range(min = 1, max = 6000))]
    pub edges: u32,
}
fn search_limit() -> u32 {
    50
}
fn directory_limit() -> u32 {
    1000
}

pub struct Query {
    pub query: Value,
    pub generation: Option<String>,
}
impl Query {
    fn new(query: Value, generation: Option<String>) -> Result<Self> {
        validate_generation(generation.as_deref())?;
        Ok(Self { query, generation })
    }
}

pub fn parse(name: &str, value: Value) -> Result<Query> {
    match name {
        "compare_graphs" => comparison(serde_json::from_value(value)?),
        "compare_function_flows" => flow_comparison(serde_json::from_value(value)?),
        "graph_snapshot_status" => {
            let args: SnapshotStatusArgs = serde_json::from_value(value)?;
            text(&args.revision)?;
            Query::new(
                json!({"kind":"snapshot_status","revision":args.revision}),
                None,
            )
        }
        "graph_status" | "graph_freshness" => {
            let args: GraphArgs = serde_json::from_value(value)?;
            let query = if name == "graph_freshness" {
                json!({"kind":"freshness","observation_version":1})
            } else {
                json!({"kind":"status"})
            };
            Query::new(query, args.expected_generation)
        }
        "find_symbols" => search(serde_json::from_value(value)?),
        "symbol_details" | "function_flow" => {
            let args: SymbolArgs = serde_json::from_value(value)?;
            text(&args.symbol)?;
            let kind = if name == "symbol_details" {
                "symbol"
            } else {
                "flow"
            };
            Query::new(
                json!({"kind":kind,"symbol":args.symbol}),
                args.expected_generation,
            )
        }
        "symbol_source" => {
            let args: SourceArgs = serde_json::from_value(value)?;
            text(&args.symbol)?;
            Query::new(
                json!({"kind":"source","symbol":args.symbol,"origin":args.origin}),
                args.expected_generation,
            )
        }
        "explore_directory" => directory(serde_json::from_value(value)?),
        "graph_section" => section(serde_json::from_value(value)?),
        "neighbors" => neighbors(serde_json::from_value(value)?),
        _ => bail!("Unknown graph tool: {name}"),
    }
}
fn comparison(args: CompareArgs) -> Result<Query> {
    text(&args.base)?;
    text(&args.head)?;
    ensure!(
        (1..=500).contains(&args.limit),
        "Comparison limit must be 1..500"
    );
    ensure!(
        args.offset == 0 || args.expected_comparison.is_some(),
        "Paged comparisons require expected_comparison"
    );
    validate_generation(args.expected_comparison.as_deref())?;
    let mut query = json!({"kind":"compare","base":args.base,"head":args.head,"offset":args.offset,"limit":args.limit,"expected_comparison":args.expected_comparison});
    snapshot_selectors(&mut query, args.base_snapshot, args.head_snapshot)?;
    Query::new(query, args.expected_generation)
}
fn flow_comparison(args: FlowComparisonArgs) -> Result<Query> {
    text(&args.base)?;
    text(&args.head)?;
    validate_generation(args.expected_comparison.as_deref())?;
    match &args.selection {
        FlowComparisonSelection::Catalog {
            text,
            offset,
            limit,
        } => {
            ensure!(text.len() <= 1024, "Flow catalog text exceeds 1024 bytes");
            ensure!(
                (1..=500).contains(limit),
                "Flow catalog limit must be 1..500"
            );
            ensure!(
                *offset == 0 || args.expected_comparison.is_some(),
                "Paged flow catalogs require expected_comparison"
            );
        }
        FlowComparisonSelection::Detail {
            symbol,
            head_symbol,
        } => {
            flow_symbol(symbol)?;
            if let Some(symbol) = head_symbol {
                flow_symbol(symbol)?;
            }
        }
    }
    let mut query = json!({"kind":"flow_comparison","base":args.base,"head":args.head,"expected_comparison":args.expected_comparison,"selection":args.selection});
    snapshot_selectors(&mut query, args.base_snapshot, args.head_snapshot)?;
    Query::new(query, None)
}
fn flow_symbol(symbol: &str) -> Result<()> {
    text(symbol)?;
    ensure!(!symbol.contains(['\0', '\n', '\r']), "Invalid flow symbol");
    Ok(())
}
fn snapshot_selectors(query: &mut Value, base: Option<String>, head: Option<String>) -> Result<()> {
    for (field, selector) in [("base_snapshot", base), ("head_snapshot", head)] {
        if let Some(selector) = selector {
            ensure!(crate::wire::generation(&selector), "Invalid {field} digest");
            query[field] = json!(selector);
        }
    }
    Ok(())
}
fn search(args: SearchArgs) -> Result<Query> {
    ensure!(args.text.len() <= 16384, "Search text exceeds 16 KiB");
    ensure!(
        (1..=200).contains(&args.limit),
        "Search limit must be 1..200"
    );
    Query::new(
        json!({"kind":"search","text":args.text,"category":args.category,"offset":args.offset,"limit":args.limit}),
        args.expected_generation,
    )
}
fn directory(args: DirectoryArgs) -> Result<Query> {
    text(&args.path)?;
    ensure!(
        (1..=1000).contains(&args.limit),
        "Directory limit must be 1..1000"
    );
    Query::new(
        json!({"kind":"explorer","path":args.path,"offset":args.offset,"limit":args.limit}),
        args.expected_generation,
    )
}
fn section(args: SectionArgs) -> Result<Query> {
    validate_selection(&args.selection)?;
    let mut query = json!({"kind":"selection","selection":args.selection});
    options(&mut query, args.filter, args.limits)?;
    Query::new(query, args.expected_generation)
}
fn neighbors(args: NeighborArgs) -> Result<Query> {
    text(&args.symbol)?;
    let mut query = json!({"kind":"neighbors","symbol":args.symbol});
    options(&mut query, args.filter, args.limits)?;
    Query::new(query, args.expected_generation)
}
fn options(query: &mut Value, filter: Option<Filter>, limits: Option<Limits>) -> Result<()> {
    if let Some(filter) = filter {
        ensure!(filter.version == 2, "Explicit filters require version 2");
        query["filter"] = serde_json::to_value(filter)?;
    }
    if let Some(limits) = limits {
        ensure!(
            (1..=1500).contains(&limits.nodes) && (1..=6000).contains(&limits.edges),
            "Graph limits exceed CLI bounds"
        );
        query["limits"] = serde_json::to_value(limits)?;
    }
    Ok(())
}
fn text(value: &str) -> Result<()> {
    ensure!(
        !value.trim().is_empty() && value.len() <= 16384,
        "Identity/path must contain 1..16384 bytes"
    );
    Ok(())
}
fn validate_selection(selection: &Selection) -> Result<()> {
    match selection {
        Selection::Overview => Ok(()),
        Selection::EntryPoints { depth, .. } => depth_limit(*depth),
        Selection::Neighborhood { symbols, depth, .. } => {
            ensure!(
                !symbols.is_empty() && symbols.len() <= 1500,
                "Neighborhood requires 1..1500 symbols"
            );
            for symbol in symbols {
                text(symbol)?;
            }
            depth_limit(*depth)
        }
        Selection::Path { from, to } => {
            text(from)?;
            text(to)
        }
        Selection::File { file } => text(file),
        Selection::Class {
            build,
            project,
            container,
        } => {
            text(build)?;
            text(project)?;
            text(container)
        }
    }
}
fn depth_limit(depth: u32) -> Result<()> {
    ensure!(depth <= 64, "Traversal depth exceeds 64");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unknown_fields_bad_generations_limits_and_legacy_filters() {
        for (tool, value) in [
            ("graph_status", json!({"project":"override"})),
            (
                "symbol_details",
                json!({"symbol":"s","expected_generation":"bad"}),
            ),
            ("find_symbols", json!({"text":"x","limit":201})),
            (
                "graph_section",
                json!({"selection":{"kind":"overview"},"filter":{"version":1,"relationship_kinds":[],"target_resolutions":[]}}),
            ),
            (
                "graph_section",
                json!({"selection":{"kind":"overview"},"limits":{"nodes":1501,"edges":1}}),
            ),
        ] {
            assert!(parse(tool, value).is_err());
        }
    }
    #[test]
    fn independent_relationship_arrays_and_generation_are_forwarded() {
        let generation = "a".repeat(64);
        let request = parse("neighbors",json!({"symbol":"s","expected_generation":generation,"filter":{"version":2,"relationship_kinds":[],"target_resolutions":["possible"]}})).unwrap();
        assert_eq!(request.query["filter"]["relationship_kinds"], json!([]));
        assert_eq!(request.generation.as_deref(), Some(generation.as_str()));
    }
}
