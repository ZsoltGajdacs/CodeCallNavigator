//! Graphviz process execution and geometry decoding. Ordering is supplied by projection.
use super::GraphView;
use anyhow::{Context, Result, bail};
use codegraph::graphviz::{coordinates, spline_points};
use serde_json::Value;
use std::{collections::BTreeMap, path::Path};

fn dot_input(view: &GraphView) -> String {
    let mut source = String::from(
        "digraph G { graph [rankdir=TB,nodesep=0.35,ranksep=0.8]; node [label=\"\",shape=box,fixedsize=true,width=3.3,height=0.85];\n",
    );
    if view.layer_bands.is_empty() {
        traversal_constraints(view, &mut source);
    } else {
        super::layer_geometry::constraints(view, &mut source);
    }
    for (index, edge) in view.edges.iter().enumerate() {
        source.push_str(&format!(
            "n{} -> n{} [id=\"e{index}\",constraint=false];\n",
            edge.from, edge.to
        ));
    }
    source.push('}');
    source
}

fn traversal_constraints(view: &GraphView, source: &mut String) {
    let mut ranks: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (index, order) in view.ordering.iter().enumerate() {
        source.push_str(&format!("n{index};\n"));
        ranks.entry(order.rank).or_default().push(index);
        if let Some(parent) = order.parent {
            source.push_str(&format!(
                "n{parent} -> n{index} [style=invis,weight=100];\n"
            ));
        }
    }
    // Explicit same-rank order keeps each parent's children adjacent even when
    // shared methods and recursive cross-links would otherwise rearrange them.
    for members in ranks.values() {
        append_rank(source, members);
    }
    // Preserve vertical order for pages whose owning parents are on another page.
    let representatives: Vec<_> = ranks.values().map(|members| members[0]).collect();
    for pair in representatives.windows(2) {
        source.push_str(&format!(
            "n{} -> n{} [style=invis,weight=0];\n",
            pair[0], pair[1]
        ));
    }
}

fn append_rank(source: &mut String, members: &[usize]) {
    source.push_str("{rank=same; ");
    for index in members {
        source.push_str(&format!("n{index}; "));
    }
    for pair in members.windows(2) {
        source.push_str(&format!(
            "n{} -> n{} [style=invis,weight=1000]; ",
            pair[0], pair[1]
        ));
    }
    source.push_str("}\n");
}

pub fn layout(mut view: GraphView, dot: &Path) -> Result<GraphView> {
    if view.nodes.is_empty() {
        view.size = [400.0, 300.0];
        return Ok(view);
    }
    let output: Value = serde_json::from_slice(&codegraph::graphviz::run(dot, &dot_input(&view))?)?;
    let bounds = coordinates(output["bb"].as_str().context("Graphviz omitted bounds")?)?;
    if bounds.len() != 4 {
        bail!("Invalid Graphviz bounds")
    }
    view.size = [bounds[2] + 32.0, bounds[3] + 32.0];
    for object in output["objects"]
        .as_array()
        .context("Graphviz omitted node geometry")?
    {
        let Some(name) = object["name"]
            .as_str()
            .and_then(|name| name.strip_prefix('n'))
        else {
            continue;
        };
        let index: usize = name.parse()?;
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
        let Some(identity) = geometry["id"].as_str() else {
            continue;
        };
        let index: usize = identity.trim_start_matches('e').parse()?;
        let edge = view
            .edges
            .get_mut(index)
            .context("Graphviz relationship index out of range")?;
        edge.points = spline_points(&geometry, bounds[3]);
    }
    super::layers::measure(&mut view);
    Ok(view)
}
