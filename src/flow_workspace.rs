//! Generation-bound flow sessions, background construction and independent navigation.
use crate::{app::State, camera::Camera, source};
use codecallnavigator::{
    flow::{self, FlowChart, FunctionFlowIR},
    graph::GraphIndex,
};
use rune::{Latest, Tx};
use std::{cell::Cell, collections::HashMap, path::Path, rc::Rc, sync::Arc};

pub struct Ready {
    pub chart: FlowChart,
    pub source: source::SourceFile,
}
#[derive(Clone)]
pub struct Session {
    pub symbol: String,
    pub scroll_key: String,
    pub ready: Option<Arc<Ready>>,
    pub camera: Camera,
    pub selected: Option<usize>,
    pub scroll: Rc<Cell<[f32; 2]>>,
    pub message: String,
}
impl Session {
    fn new(symbol: String, epoch: u64, visit: u64) -> Self {
        let scroll_key = format!(
            "flow-source-scroll-{}",
            blake3::hash(format!("{epoch}:{visit}:{symbol}").as_bytes()).to_hex()
        );
        Self {
            symbol,
            scroll_key,
            ready: None,
            camera: Camera::default(),
            selected: None,
            scroll: Rc::new(Cell::new([0.0, 0.0])),
            message: "Preparing function flowchart…".into(),
        }
    }
}
#[derive(Default)]
pub struct Workspace {
    pub current: Option<Session>,
    pub back: Vec<Session>,
    pub job: Latest<Output>,
    pub viewport: [f32; 2],
    pub fit_pending: bool,
    pub window: [f32; 2],
    pub source_width: f32,
    pub drag_width: f32,
    cache: HashMap<(u64, String), Arc<Ready>>,
    serial: u64,
    epoch: u64,
    restore_scroll: Option<[f32; 2]>,
}
pub struct Output {
    serial: u64,
    epoch: u64,
    symbol: String,
    result: Result<Ready, String>,
}
impl Workspace {
    pub fn invalidate(&mut self) {
        self.serial += 1;
        self.job.cancel();
        self.current = None;
        self.back.clear();
        self.cache.clear();
        self.restore_scroll = None;
    }
}

impl State {
    pub fn open_selected_flow(&mut self) {
        if let Some(symbol) = self.nav.selected.clone() {
            self.open_flow(symbol);
        }
    }
    pub fn open_flow(&mut self, symbol: String) {
        self.close_source();
        let Some(graph) = self.graph.clone() else {
            return;
        };
        let workspace = &mut self.flow_workspace;
        if workspace
            .current
            .as_ref()
            .is_some_and(|s| s.symbol == symbol)
        {
            return;
        }
        if let Some(previous) = workspace.current.take() {
            workspace.back.push(previous);
            if workspace.back.len() > 100 {
                workspace.back.remove(0);
            }
        }
        workspace.serial += 1;
        workspace.job.cancel();
        workspace.epoch = self.epoch;
        let mut session = Session::new(symbol.clone(), self.epoch, workspace.serial);
        if let Some(ready) = workspace.cache.get(&(self.epoch, symbol.clone())) {
            session.install(ready.clone(), workspace.viewport);
        }
        workspace.current = Some(session);
        workspace.fit_pending = true;
        workspace.restore_scroll = workspace
            .current
            .as_ref()
            .map(|session| session.scroll.get());
        if workspace.current.as_ref().unwrap().ready.is_none() {
            self.request_flow(graph, symbol);
        }
    }
    fn request_flow(&mut self, graph: Arc<GraphIndex>, symbol: String) {
        let function = match lookup(&graph, &symbol) {
            Ok(function) => function,
            Err(error) => {
                self.flow_workspace.current.as_mut().unwrap().message = error;
                return;
            }
        };
        let dot = self
            .runtime
            .as_ref()
            .map(|r| r.dot.clone())
            .unwrap_or_else(|| "dot".into());
        let epoch = self.epoch;
        let serial = self.flow_workspace.serial;
        self.flow_workspace.job.request(symbol, move |symbol| {
            let result = prepare(&graph, &function, &dot).map_err(|error| format!("{error:#}"));
            Output {
                serial,
                epoch,
                symbol,
                result,
            }
        });
    }
    pub fn close_flow(&mut self) {
        self.flow_workspace.serial += 1;
        self.flow_workspace.job.cancel();
        self.flow_workspace.current = None;
        self.flow_workspace.back.clear();
        self.flow_workspace.restore_scroll = None;
    }
    pub fn flow_back(&mut self) {
        let Some(previous) = self.flow_workspace.back.pop() else {
            self.close_flow();
            return;
        };
        self.flow_workspace.serial += 1;
        self.flow_workspace.job.cancel();
        let symbol = previous.symbol.clone();
        let needs_load = previous.ready.is_none();
        self.flow_workspace.current = Some(previous);
        self.flow_workspace.fit_pending = false;
        self.flow_workspace.restore_scroll = self
            .flow_workspace
            .current
            .as_ref()
            .map(|session| session.scroll.get());
        if needs_load && let Some(graph) = self.graph.clone() {
            self.request_flow(graph, symbol);
        }
    }
    pub fn fit_flow(&mut self) {
        let workspace = &mut self.flow_workspace;
        if let Some(session) = &mut workspace.current
            && let Some(ready) = &session.ready
        {
            session.camera.fit(ready.chart.size, workspace.viewport);
        }
    }
    pub fn select_flow_step(&mut self, index: usize, tx: &mut Tx) {
        let Some(session) = &mut self.flow_workspace.current else {
            return;
        };
        let Some(ready) = &session.ready else {
            return;
        };
        let Some(step) = ready.chart.steps.get(index) else {
            return;
        };
        session.selected = Some(index);
        let target = ready
            .source
            .lines
            .partition_point(|line| line.range.end < step.span.start)
            .min(ready.source.lines.len().saturating_sub(1));
        tx.scroll_center_in_instant(
            session.scroll_key.clone(),
            format!("flow-source-line-{target}"),
        );
    }
}

fn lookup(graph: &GraphIndex, symbol: &str) -> Result<FunctionFlowIR, String> {
    flow::lookup(&graph.data, symbol).cloned()
}
fn prepare(graph: &GraphIndex, function: &FunctionFlowIR, dot: &Path) -> anyhow::Result<Ready> {
    let facts = graph
        .data
        .architecture
        .as_ref()
        .and_then(|i| {
            i.files.iter().find(|file| {
                file.file == function.file
                    && file.build == function.build
                    && file.project == function.project
            })
        })
        .ok_or_else(|| {
            anyhow::anyhow!("Refresh the graph to cache this function's analyzed source")
        })?;
    anyhow::ensure!(
        function.span.start <= function.span.end
            && function.span.end <= facts.content.len()
            && facts.content.is_char_boundary(function.span.start)
            && facts.content.is_char_boundary(function.span.end),
        "The flow source range does not match its cached snapshot"
    );
    anyhow::ensure!(
        facts.content.len() as u64 <= source::MAX_SOURCE_BYTES,
        "The analyzed source exceeds the 16 MiB flow source limit"
    );
    let chart = flow::layout(flow::build(function)?, dot)?;
    Ok(Ready {
        chart,
        source: source::from_snapshot(facts.content.clone(), Path::new(&function.file)),
    })
}

pub fn poll(state: &mut State, tx: &mut Tx) -> bool {
    if state.flow_workspace.current.is_some() && state.flow_workspace.epoch != state.epoch {
        state.flow_workspace.invalidate();
        return true;
    }
    let loaded = take_output(state);
    let scrolled = restore_scroll(&mut state.flow_workspace, tx);
    loaded | scrolled
}

fn take_output(state: &mut State) -> bool {
    let workspace = &mut state.flow_workspace;
    let Some(output) = workspace.job.take() else {
        return false;
    };
    let matches = (output.serial, output.epoch) == (workspace.serial, state.epoch)
        && workspace
            .current
            .as_ref()
            .is_some_and(|session| session.symbol == output.symbol);
    if !matches {
        return false;
    }
    let session = workspace.current.as_mut().unwrap();
    match output.result {
        Ok(ready) => {
            let ready = Arc::new(ready);
            session.install(ready.clone(), workspace.viewport);
            workspace.fit_pending = true;
            if workspace.cache.len() >= 32 {
                workspace.cache.clear();
            }
            workspace.cache.insert((state.epoch, output.symbol), ready);
        }
        Err(error) => session.message = error,
    }
    workspace.restore_scroll = workspace
        .current
        .as_ref()
        .map(|session| session.scroll.get());
    true
}

fn restore_scroll(workspace: &mut Workspace, tx: &mut Tx) -> bool {
    let Some([x, y]) = workspace.restore_scroll else {
        return false;
    };
    let Some(session) = workspace
        .current
        .as_ref()
        .filter(|session| session.ready.is_some())
    else {
        return false;
    };
    tx.scroll_to_instant(session.scroll_key.clone(), x, y);
    workspace.restore_scroll = None;
    true
}

impl State {
    pub fn flow_workspace_revision(&self) -> u64 {
        self.flow_workspace.serial
    }
}

impl Session {
    fn install(&mut self, ready: Arc<Ready>, viewport: [f32; 2]) {
        self.camera.fit(ready.chart.size, viewport);
        let target = ready
            .source
            .lines
            .partition_point(|line| line.range.end < ready.chart.span.start);
        self.scroll.set([0.0, target as f32 * 20.0]);
        self.ready = Some(ready);
        self.message.clear();
    }
}
