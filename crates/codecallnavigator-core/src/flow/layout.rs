//! Graphviz geometry for the completion graph; source and semantic binding stay elsewhere.
use super::*;
use crate::graphviz::{coordinates, spline_points};
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::path::Path;

pub fn layout(mut chart: FlowChart, dot: &Path) -> Result<FlowChart> {
    if chart.steps.is_empty() {
        chart.size = [400.0, 300.0];
        return Ok(chart);
    }
    let output: Value = serde_json::from_slice(&crate::graphviz::run(dot, &dot_input(&chart))?)?;
    let bounds = coordinates(
        output["bb"]
            .as_str()
            .context("Graphviz omitted flow bounds")?,
    )?;
    ensure!(bounds.len() == 4, "Invalid Graphviz flow bounds");
    chart.size = [bounds[2] + 32.0, bounds[3] + 32.0];
    read_steps(&mut chart, &output, bounds[3])?;
    read_links(&mut chart, &output, bounds[3])?;
    Ok(chart)
}
fn dot_input(chart: &FlowChart) -> String {
    let mut dot = String::from(
        "digraph Flow { graph [rankdir=TB,nodesep=0.4,ranksep=0.55]; node [label=\"\",shape=box,fixedsize=true,width=4.0,height=0.8];\n",
    );
    for step in &chart.steps {
        dot.push_str(&format!("n{};\n", step.id));
    }
    dot.push_str(&format!("{{rank=min;n{};}}\n", chart.entry));
    for (index, link) in chart.links.iter().enumerate() {
        let constraint = if link.kind == "back" { "false" } else { "true" };
        dot.push_str(&format!(
            "n{} -> n{} [id=\"e{index}\",constraint={constraint}];\n",
            link.from, link.to
        ));
    }
    dot.push('}');
    dot
}
fn read_steps(chart: &mut FlowChart, output: &Value, height: f32) -> Result<()> {
    let objects = output["objects"]
        .as_array()
        .context("Graphviz omitted flow steps")?;
    for object in objects {
        let Some(index) = identity(&object["name"], 'n')? else {
            continue;
        };
        let step = chart
            .steps
            .get_mut(index)
            .context("Graphviz flow step index out of range")?;
        let position = coordinates(
            object["pos"]
                .as_str()
                .context("Graphviz flow step has no position")?,
        )?;
        ensure!(
            position.len() == 2,
            "Invalid Graphviz flow step coordinates"
        );
        step.size = [288.0, 57.6];
        step.position = [
            position[0] - step.size[0] / 2.0 + 16.0,
            height - position[1] - step.size[1] / 2.0 + 16.0,
        ];
    }
    Ok(())
}
fn read_links(chart: &mut FlowChart, output: &Value, height: f32) -> Result<()> {
    for geometry in output["edges"].as_array().into_iter().flatten() {
        let Some(index) = identity(&geometry["id"], 'e')? else {
            continue;
        };
        let link = chart
            .links
            .get_mut(index)
            .context("Graphviz flow link index out of range")?;
        link.points = spline_points(geometry, height);
    }
    Ok(())
}
fn identity(value: &Value, prefix: char) -> Result<Option<usize>> {
    value
        .as_str()
        .and_then(|value| value.strip_prefix(prefix))
        .map(|value| value.parse().context("Invalid Graphviz flow identity"))
        .transpose()
}
