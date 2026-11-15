//! Evidence collected from actual native Drawn frames, distinct from queued input.
use super::config::Configuration;
use crate::{
    app::State,
    flow_acceptance::report::{Frames, RuntimePaths},
};
use rune::Runtime;
use serde::Serialize;
use std::{collections::BTreeMap, path::PathBuf};
#[derive(Clone, Serialize)]
pub(in crate::flow_acceptance) struct Report {
    pub version: u32,
    pub status: String,
    pub error: Option<String>,
    pub project: PathBuf,
    pub root_symbol: String,
    pub generation: String,
    pub expected_viewport: [u32; 2],
    pub initial_viewport: Option<[f32; 2]>,
    pub viewport: [f32; 2],
    pub process_id: u32,
    pub window_id: Option<u32>,
    pub window_key: Option<String>,
    pub runtime: Option<RuntimePaths>,
    pub frames: Frames,
    pub checks: BTreeMap<String, bool>,
    pub captures: Vec<String>,
    pub input_model: &'static str,
    pub readiness_definition: &'static str,
    pub ready_drawn: Vec<ReadySample>,
}
#[derive(Clone, Serialize)]
pub(in crate::flow_acceptance) struct ReadySample {
    pub operation: &'static str,
    pub milliseconds: f64,
    pub steps: usize,
    pub links: usize,
    pub query_milliseconds: Option<f64>,
    pub layout_milliseconds: Option<f64>,
}
impl Report {
    pub fn new(config: &Configuration) -> Self {
        Self {
            version: 3,
            status: "running".into(),
            error: None,
            project: config.project.clone(),
            root_symbol: config.root_symbol.clone(),
            generation: config.generation.clone(),
            expected_viewport: config.expected_viewport,
            initial_viewport: None,
            viewport: [0.0; 2],
            process_id: std::process::id(),
            window_id: None,
            window_key: None,
            runtime: None,
            frames: Frames::default(),
            checks: BTreeMap::new(),
            captures: vec![],
            input_model: "Synthetic pointer input through production Runtime handlers in an actual native window; OS delivery is not measured",
            readiness_definition: "Synthetic pointer-down to first Drawn frame containing the replacement chart; gesture settling is excluded. No latency pass threshold.",
            ready_drawn: vec![],
        }
    }
    pub fn check(&mut self, name: &str) {
        self.checks.insert(name.into(), true);
    }
    pub fn observe(&mut self, runtime: &Runtime<State>) {
        self.viewport = [runtime.viewport.0, runtime.viewport.1];
        self.initial_viewport.get_or_insert(self.viewport);
        self.runtime = runtime.state.runtime.as_ref().map(|runtime| RuntimePaths {
            engine: runtime.engine.clone(),
            resources: runtime.resources.clone(),
            dot: runtime.dot.clone(),
        });
    }
}
