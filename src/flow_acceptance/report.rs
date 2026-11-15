//! Native observations are recorded separately from queued acceptance input.
use super::config::Ordinary as Configuration;
use crate::app::State;
use rune::{FrameEncoded, FrameOutcome, Runtime};
use serde::Serialize;
use std::{path::PathBuf, time::Instant};

#[derive(Clone, Default, Serialize)]
pub(super) struct Checks {
    pub window_setup_acknowledged: bool,
    pub viewport_ready: bool,
    pub root_opened: bool,
    pub source_exact: bool,
    pub source_highlighted: bool,
    pub camera_dragged: bool,
    pub source_scrolled: bool,
    pub source_scroll_preserved_camera: bool,
    pub zero_span_checked: Option<bool>,
    pub target_followed: bool,
    pub back_restored: bool,
    pub graph_restored: bool,
}

#[derive(Clone, Default, Serialize)]
pub(super) struct Timings {
    pub definition: &'static str,
    pub root_click_to_ready_drawn_ms: Option<f64>,
    pub target_click_to_ready_drawn_ms: Option<f64>,
}

#[derive(Clone, Default, Serialize)]
pub(super) struct Frames {
    pub drawn: u64,
    pub skipped: u64,
    pub unchanged: u64,
    pub first_drawn_ms: Option<f64>,
    pub last_drawn_ms: Option<f64>,
    pub last_update_ms: Option<f64>,
    pub last_encode_ms: Option<f64>,
    pub refresh_hz: Option<f64>,
}

#[derive(Clone, Serialize)]
pub(super) struct RuntimePaths {
    pub engine: PathBuf,
    pub resources: PathBuf,
    pub dot: PathBuf,
}

#[derive(Clone, Serialize)]
pub(super) struct Report {
    pub version: u32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub project: PathBuf,
    pub root_symbol: String,
    pub target_symbol: String,
    pub target_signature: Option<String>,
    pub target_certainty: Option<String>,
    pub target_provenance: Option<String>,
    pub generation: Option<String>,
    pub source_hash: Option<String>,
    pub source_bytes: Option<usize>,
    pub runtime: Option<RuntimePaths>,
    pub window_id: Option<u32>,
    pub process_id: u32,
    pub window_key: Option<String>,
    pub expected_viewport: [u32; 2],
    pub initial_viewport: Option<[f32; 2]>,
    pub viewport: [f32; 2],
    pub backing_scale: f32,
    pub input_model: &'static str,
    pub checks: Checks,
    pub timings: Timings,
    pub frames: Frames,
}

impl Report {
    pub fn new(configuration: &Configuration) -> Self {
        Self {
            version: 1,
            status: "running".into(),
            error: None,
            project: configuration.project.clone(),
            root_symbol: configuration.root_symbol.clone(),
            target_symbol: configuration.target_symbol.clone(),
            target_signature: None,
            target_certainty: None,
            target_provenance: None,
            generation: None,
            source_hash: None,
            source_bytes: None,
            runtime: None,
            window_id: None,
            process_id: std::process::id(),
            window_key: None,
            expected_viewport: configuration.expected_viewport,
            initial_viewport: None,
            viewport: [0.0; 2],
            backing_scale: 0.0,
            input_model: "Synthetic pointer input through production Runtime handlers in an actual native window; OS mouse delivery is not measured",
            checks: Checks::default(),
            timings: Timings {
                definition: "Queued synthetic pointer press to first native Drawn frame with the exact requested ready chart, including frames during gesture settling; OS input delivery is not measured",
                ..Timings::default()
            },
            frames: Frames::default(),
        }
    }

    pub fn observe(&mut self, runtime: &Runtime<State>) {
        self.viewport = [runtime.viewport.0, runtime.viewport.1];
        self.initial_viewport.get_or_insert(self.viewport);
        self.backing_scale = runtime.env.scale;
        self.generation = runtime.state.generation.clone();
        self.runtime = runtime.state.runtime.as_ref().map(|context| RuntimePaths {
            engine: context.engine.clone(),
            resources: context.resources.clone(),
            dot: context.dot.clone(),
        });
    }

    pub fn encoded(&mut self, frame: &FrameEncoded, origin: Instant) {
        self.frames.encoded(frame, origin);
    }
}
impl Frames {
    pub fn encoded(&mut self, frame: &FrameEncoded, origin: Instant) {
        match frame.outcome() {
            FrameOutcome::Drawn => {
                let milliseconds =
                    frame.at().saturating_duration_since(origin).as_secs_f64() * 1000.0;
                self.drawn += 1;
                self.first_drawn_ms.get_or_insert(milliseconds);
                self.last_drawn_ms = Some(milliseconds);
                self.last_update_ms = frame.update_ms();
                self.last_encode_ms = frame.encode_ms();
                self.refresh_hz = frame.refresh_hz();
            }
            FrameOutcome::Skipped => self.skipped += 1,
            FrameOutcome::Unchanged => self.unchanged += 1,
            _ => (),
        }
    }
}

#[derive(Serialize)]
pub(super) struct WindowRequest {
    pub version: u32,
    pub window_id: u32,
    pub process_id: u32,
    pub expected_viewport: [u32; 2],
}

#[derive(Serialize)]
pub(super) struct CaptureRequest {
    pub version: u32,
    pub checkpoint: &'static str,
    pub window_id: u32,
    pub symbol: String,
    pub generation: String,
    pub expected_viewport: [u32; 2],
    pub viewport: [f32; 2],
}
