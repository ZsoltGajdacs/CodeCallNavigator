//! Evidence records drawn observations, never successful input intent alone.
use super::{config::Configuration, protocol::Checkpoint};
use crate::{
    app::State,
    flow_acceptance::report::{Frames, RuntimePaths},
};
use serde::Serialize;
use std::{collections::BTreeMap, path::PathBuf, time::Instant};
#[derive(Clone, Serialize)]
pub(in crate::flow_acceptance) struct Report {
    pub version: u32,
    pub status: String,
    pub error: Option<String>,
    pub project: PathBuf,
    pub expected_viewport: [u32; 2],
    pub viewport: [f32; 2],
    pub process_id: u32,
    pub window_id: Option<u32>,
    pub runtime: Option<RuntimePaths>,
    pub frames: Frames,
    pub input_model: &'static str,
    pub timing_definition: &'static str,
    pub checks: BTreeMap<String, bool>,
    pub observations: Vec<serde_json::Value>,
    pub ready_drawn_ms: BTreeMap<String, f64>,
    pub captures: Vec<Checkpoint>,
}
impl Report {
    pub fn new(config: &Configuration) -> Self {
        Self {
            version: 2,
            status: "running".into(),
            error: None,
            project: config.project.clone(),
            expected_viewport: config.expected_viewport,
            viewport: [0.0; 2],
            process_id: std::process::id(),
            window_id: None,
            runtime: None,
            frames: Frames::default(),
            input_model: "Synthetic pointer and focus/key/paste input through production Runtime handlers in an actual native window; OS delivery is not measured",
            timing_definition: "Queued selection press to first native Drawn frame with the exact requested pair; gesture settling does not delay the readiness sample",
            checks: BTreeMap::new(),
            observations: Vec::new(),
            ready_drawn_ms: BTreeMap::new(),
            captures: Vec::new(),
        }
    }
    pub fn observe(&mut self, runtime: &rune::Runtime<State>) {
        self.viewport = [runtime.viewport.0, runtime.viewport.1];
        self.runtime = runtime.state.runtime.as_ref().map(|r| RuntimePaths {
            engine: r.engine.clone(),
            resources: r.resources.clone(),
            dot: r.dot.clone(),
        });
    }
    pub fn check(&mut self, name: &str) {
        self.checks.insert(name.into(), true);
    }
    pub fn ready(&mut self, name: &str, requested: Option<Instant>, drawn_at: Instant) {
        if let Some(at) = requested {
            self.ready_drawn_ms
                .entry(name.into())
                .or_insert(drawn_at.saturating_duration_since(at).as_secs_f64() * 1000.0);
        }
    }
}
