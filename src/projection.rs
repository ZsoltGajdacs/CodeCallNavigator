//! Bounded presentation of an existing graph. No source-code relationships are inferred here.
use anyhow::{Context, Result, bail};
use codegraph::{
    graph::{Direction, Filter, GraphIndex, Neighborhood},
    model::Node,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const NODE_LIMIT: usize = 1_500;
const EDGE_LIMIT: usize = 6_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Grouping {
    #[default]
    Project,
    Namespace,
    Folder,
}

#[derive(Clone)]
pub struct Request {
    pub grouping: Grouping,
    pub expanded: HashSet<String>,
    pub focus: Option<u32>,
    pub direction: Direction,
    pub depth: u32,
    pub path: Option<(u32, u32)>,
    pub filter: Filter,
    pub page: usize,
}

#[derive(Clone, Debug, Default)]
pub struct ViewNode {
    pub key: String,
    pub label: String,
    pub detail: String,
    pub member: Option<u32>,
    pub count: usize,
    pub internal_edges: usize,
    pub cyclic: bool,
    pub position: [f32; 2],
    pub size: [f32; 2],
}

#[derive(Clone, Debug, Default)]
pub struct ViewEdge {
    pub from: usize,
    pub to: usize,
    pub kind: String,
    pub certainty: String,
    pub count: u64,
    pub points: Vec<[f32; 2]>,
}

#[derive(Clone, Debug, Default)]
pub struct GraphView {
    pub nodes: Vec<ViewNode>,
    pub edges: Vec<ViewEdge>,
    pub size: [f32; 2],
    pub omitted_nodes: usize,
    pub omitted_edges: usize,
    pub hidden_nodes: usize,
    pub hidden_edges: usize,
    pub collapsed_edges: usize,
    pub message: String,
    pub pages: usize,
}

struct Selection {
    nodes: Vec<u32>,
    edges: Vec<u32>,
    message: String,
    omitted_nodes: usize,
    omitted_edges: usize,
}

fn key(parts: &[&str]) -> String {
    serde_json::to_string(parts).expect("string arrays serialize")
}

fn relative_file(node: &Node) -> PathBuf {
    let build = Path::new(&node.build);
    let root = if build.extension().is_some() {
        build.parent().unwrap_or(build)
    } else {
        build
    };
    let file = Path::new(&node.file);
    file.strip_prefix(root).unwrap_or(file).to_path_buf()
}

fn groups(node: &Node, grouping: Grouping) -> Vec<(String, String)> {
    let scope = [&*node.build, &*node.project];
    let mut result = vec![(key(&["project", scope[0], scope[1]]), node.project.clone())];
    match grouping {
        Grouping::Namespace => result.push((
            key(&["namespace", scope[0], scope[1], &node.namespace]),
            if node.namespace.is_empty() {
                "(global namespace)".into()
            } else {
                node.namespace.clone()
            },
        )),
        Grouping::Folder => {
            let file = relative_file(node);
            let mut folder = PathBuf::new();
            for part in file.parent().into_iter().flat_map(Path::components) {
                folder.push(part);
                let name = folder.to_string_lossy();
                result.push((
                    key(&["folder", scope[0], scope[1], &name]),
                    name.into_owned(),
                ));
            }
        }
        Grouping::Project => {}
    }
    result.push((
        key(&["class", scope[0], scope[1], &node.container]),
        node.container.clone(),
    ));
    result
}

fn visible_node(node: &Node, index: u32, request: &Request) -> ViewNode {
    if request.focus.is_none() && request.path.is_none() {
        for (key, label) in groups(node, request.grouping) {
            if !request.expanded.contains(&key) {
                return ViewNode {
                    key,
                    label,
                    detail: node.project.clone(),
                    ..Default::default()
                };
            }
        }
    }
    ViewNode {
        key: node.id.clone(),
        label: node.label.clone(),
        detail: node.container.clone(),
        member: Some(index),
        ..Default::default()
    }
}

fn selected_indices(graph: &GraphIndex, request: &Request) -> Selection {
    if let Some((from, to)) = request.path {
        return path_indices(graph, from, to, &request.filter);
    }
    if let Some(focus) = request.focus {
        let slice = graph.neighborhood(&Neighborhood {
            node: focus,
            direction: request.direction,
            depth: request.depth,
            filter: &request.filter,
        });
        return Selection {
            nodes: slice.nodes,
            edges: slice.edges,
            message: format!("Focused neighborhood · depth {}", request.depth),
            omitted_nodes: slice.omitted_nodes,
            omitted_edges: slice.omitted_edges,
        };
    }
    let nodes = graph
        .data
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| request.filter.accepts_node(n))
        .map(|(i, _)| i as u32)
        .collect();
    let edges = graph
        .data
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| request.filter.accepts_edge(e))
        .map(|(i, _)| i as u32)
        .collect();
    Selection {
        nodes,
        edges,
        message: "Whole workspace · expand groups to explore methods".into(),
        omitted_nodes: 0,
        omitted_edges: 0,
    }
}

fn path_indices(graph: &GraphIndex, from: u32, to: u32, filter: &Filter) -> Selection {
    let Some(nodes) = graph.shortest_path(from, to, filter) else {
        return Selection {
            nodes: Vec::new(),
            edges: Vec::new(),
            message: "No directed path in the analyzed graph with these filters".into(),
            omitted_nodes: 0,
            omitted_edges: 0,
        };
    };
    let pairs: HashSet<_> = nodes.windows(2).map(|pair| (pair[0], pair[1])).collect();
    let edges = graph
        .data
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| pairs.contains(&(e.from, e.to)) && filter.accepts_edge(e))
        .map(|(i, _)| i as u32)
        .collect();
    let message = format!(
        "Shortest directed path · {} steps",
        nodes.len().saturating_sub(1)
    );
    Selection {
        nodes,
        edges,
        message,
        omitted_nodes: 0,
        omitted_edges: 0,
    }
}

struct NodePage {
    nodes: Vec<ViewNode>,
    membership: HashMap<u32, String>,
    all_positions: HashMap<String, usize>,
    aggregate_groups: HashSet<usize>,
    positions: HashMap<usize, usize>,
    total_nodes: usize,
    pages: usize,
}

fn node_page(graph: &GraphIndex, request: &Request, selection: &Selection) -> NodePage {
    let cyclic: HashSet<u32> = graph
        .cycles(&request.filter)
        .into_iter()
        .flatten()
        .collect();
    let mut representatives = BTreeMap::new();
    let mut membership = HashMap::new();
    for &index in &selection.nodes {
        let node = visible_node(&graph.data.nodes[index as usize], index, request);
        membership.insert(index, node.key.clone());
        let representative = representatives.entry(node.key.clone()).or_insert(node);
        representative.count += 1;
        representative.cyclic |= cyclic.contains(&index);
    }
    let total_nodes = representatives.len();
    let pages = total_nodes.div_ceil(NODE_LIMIT).max(1);
    let all_positions: HashMap<_, _> = representatives
        .keys()
        .enumerate()
        .map(|(i, key)| (key.clone(), i))
        .collect();
    let aggregate_groups: HashSet<usize> = representatives
        .iter()
        .filter(|(_, node)| node.member.is_none())
        .map(|(key, _)| all_positions[key])
        .collect();
    let nodes: Vec<ViewNode> = representatives
        .into_values()
        .skip(request.page.min(pages - 1) * NODE_LIMIT)
        .take(NODE_LIMIT)
        .collect();
    let positions: HashMap<_, _> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (all_positions[&n.key], i))
        .collect();
    NodePage {
        nodes,
        membership,
        all_positions,
        aggregate_groups,
        positions,
        total_nodes,
        pages,
    }
}

struct PageEdges {
    edges: Vec<ViewEdge>,
    total_edges: usize,
    collapsed_edges: usize,
}

fn page_edges(graph: &GraphIndex, selection: &Selection, page: &mut NodePage) -> PageEdges {
    let mut relations = BTreeMap::new();
    let mut collapsed_edges = 0;
    for &index in &selection.edges {
        let edge = &graph.data.edges[index as usize];
        let Some(from) = page
            .membership
            .get(&edge.from)
            .and_then(|key| page.all_positions.get(key))
        else {
            continue;
        };
        let Some(to) = page
            .membership
            .get(&edge.to)
            .and_then(|key| page.all_positions.get(key))
        else {
            continue;
        };
        if from == to && page.aggregate_groups.contains(from) {
            collapsed_edges += 1;
            if let Some(&visible) = page.positions.get(from) {
                page.nodes[visible].internal_edges += 1;
            }
            continue;
        }
        *relations
            .entry((*from, *to, edge.kind.clone(), edge.certainty.clone()))
            .or_insert(0_u64) += u64::from(edge.count);
    }
    let total_edges = relations.len();
    let edges: Vec<_> = relations
        .into_iter()
        .filter_map(|((from, to, kind, certainty), count)| {
            Some(ViewEdge {
                from: *page.positions.get(&from)?,
                to: *page.positions.get(&to)?,
                kind,
                certainty,
                count,
                ..Default::default()
            })
        })
        .take(EDGE_LIMIT)
        .collect();
    PageEdges {
        edges,
        total_edges,
        collapsed_edges,
    }
}

fn hidden_counts(graph: &GraphIndex, filter: &Filter) -> (usize, usize) {
    let hidden_nodes = graph
        .data
        .nodes
        .iter()
        .filter(|n| !filter.accepts_node(n))
        .count();
    let hidden_edges = graph
        .data
        .edges
        .iter()
        .filter(|e| {
            !filter.accepts_edge(e)
                || !filter.accepts_node(&graph.data.nodes[e.from as usize])
                || !filter.accepts_node(&graph.data.nodes[e.to as usize])
        })
        .count();
    (hidden_nodes, hidden_edges)
}

pub fn project(graph: &GraphIndex, request: &Request) -> GraphView {
    let selection = selected_indices(graph, request);
    let mut page = node_page(graph, request, &selection);
    let relationships = page_edges(graph, &selection, &mut page);
    let (hidden_nodes, hidden_edges) = hidden_counts(graph, &request.filter);
    GraphView {
        omitted_nodes: selection.omitted_nodes + page.total_nodes.saturating_sub(page.nodes.len()),
        omitted_edges: selection.omitted_edges
            + relationships
                .total_edges
                .saturating_sub(relationships.edges.len()),
        nodes: page.nodes,
        edges: relationships.edges,
        pages: page.pages,
        collapsed_edges: relationships.collapsed_edges,
        hidden_nodes,
        hidden_edges,
        message: selection.message,
        ..Default::default()
    }
}

fn dot_input(view: &GraphView) -> String {
    let mut source = String::from(
        "digraph G { graph [rankdir=LR,nodesep=0.35,ranksep=0.8]; node [label=\"\",shape=box,fixedsize=true,width=3.3,height=0.85];\n",
    );
    for index in 0..view.nodes.len() {
        source.push_str(&format!("n{index};\n"));
    }
    for (index, edge) in view.edges.iter().enumerate() {
        source.push_str(&format!(
            "n{} -> n{} [id=\"e{index}\"];\n",
            edge.from, edge.to
        ));
    }
    source.push('}');
    source
}

fn run_dot(dot: &Path, source: &str) -> Result<Vec<u8>> {
    let mut command = Command::new(dot);
    let plugins = dot
        .parent()
        .and_then(Path::parent)
        .map(|path| path.join("lib/graphviz"));
    if let Some(path) = plugins.filter(|path| path.is_dir()) {
        command.env("GVBINDIR", path);
    }
    let mut child = command
        .arg("-Tjson")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Cannot start Graphviz; install or bundle dot")?;
    child
        .stdin
        .take()
        .context("Graphviz input unavailable")?
        .write_all(source.as_bytes())?;
    // Drain pipes while dot works so a large JSON result cannot fill stdout and deadlock.
    let output = child.stdout.take().expect("piped stdout");
    let errors = child.stderr.take().expect("piped stderr");
    let reader = thread::spawn(move || {
        use std::io::Read;
        let mut data = Vec::new();
        let mut stream = output;
        stream.read_to_end(&mut data).map(|_| data)
    });
    let error_reader = thread::spawn(move || {
        use std::io::Read;
        let mut data = String::new();
        let mut stream = errors;
        stream.read_to_string(&mut data).map(|_| data)
    });
    let deadline = Instant::now() + Duration::from_secs(15);
    while child.try_wait()?.is_none() {
        if Instant::now() > deadline {
            child.kill()?;
            child.wait()?;
            bail!("Graphviz layout timed out; narrow the neighborhood or collapse groups")
        }
        thread::sleep(Duration::from_millis(20));
    }
    let data = reader
        .join()
        .map_err(|_| anyhow::anyhow!("Graphviz output worker failed"))??;
    let errors = error_reader
        .join()
        .map_err(|_| anyhow::anyhow!("Graphviz error worker failed"))??;
    if !child.wait()?.success() {
        bail!("Graphviz layout failed: {errors}")
    }
    Ok(data)
}

fn coordinates(value: &str) -> Result<Vec<f32>> {
    value
        .split(',')
        .map(|part| part.parse().context("Invalid Graphviz coordinate"))
        .collect()
}

pub fn layout(mut view: GraphView, dot: &Path) -> Result<GraphView> {
    if view.nodes.is_empty() {
        view.size = [400.0, 300.0];
        return Ok(view);
    }
    let output: Value = serde_json::from_slice(&run_dot(dot, &dot_input(&view))?)?;
    let bounds = coordinates(output["bb"].as_str().context("Graphviz omitted bounds")?)?;
    if bounds.len() != 4 {
        bail!("Invalid Graphviz bounds")
    }
    view.size = [bounds[2] + 32.0, bounds[3] + 32.0];
    for object in output["objects"]
        .as_array()
        .context("Graphviz omitted node geometry")?
    {
        let index: usize = object["name"]
            .as_str()
            .context("Graphviz node has no name")?
            .trim_start_matches('n')
            .parse()?;
        let node = view
            .nodes
            .get_mut(index)
            .context("Graphviz node index out of range")?;
        let position = coordinates(
            object["pos"]
                .as_str()
                .context("Graphviz node has no position")?,
        )?;
        if position.len() != 2 {
            bail!("Invalid node position")
        }
        node.size = [237.6, 61.2];
        node.position = [
            position[0] - node.size[0] / 2.0 + 16.0,
            bounds[3] - position[1] - node.size[1] / 2.0 + 16.0,
        ];
    }
    let output_edges = output["edges"].as_array().cloned().unwrap_or_default();
    for geometry in output_edges {
        let index: usize = geometry["id"]
            .as_str()
            .context("Graphviz relationship has no identity")?
            .trim_start_matches('e')
            .parse()?;
        let edge = view
            .edges
            .get_mut(index)
            .context("Graphviz relationship index out of range")?;
        edge.points = spline_points(&geometry, bounds[3]);
    }
    Ok(view)
}

fn spline_points(edge: &Value, height: f32) -> Vec<[f32; 2]> {
    edge["_draw_"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|op| op["op"] == "b" || op["op"] == "B")
        .flat_map(|op| op["points"].as_array().into_iter().flatten())
        .filter_map(|point| {
            Some([
                point[0].as_f64()? as f32 + 16.0,
                height - point[1].as_f64()? as f32 + 16.0,
            ])
        })
        .collect()
}
