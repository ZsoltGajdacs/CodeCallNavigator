//! Shared bounded Graphviz process execution; layout policy belongs to each caller.
mod process;
use anyhow::{Context, Result};
pub use process::{run, run_cancellable};
use serde_json::Value;

pub fn coordinates(value: &str) -> Result<Vec<f32>> {
    value
        .split(',')
        .map(|part| part.parse().context("Invalid Graphviz coordinate"))
        .collect()
}

pub fn spline_points(edge: &Value, height: f32) -> Vec<[f32; 2]> {
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
