//! The desktop consumes the same published artifact as standalone CLI users.
mod process;
#[cfg(test)]
mod tests;

use crate::runtime::Runtime;
use anyhow::{Context, Result, bail, ensure};
use codecallnavigator::{
    artifact::{Artifact, GRAPH_PATH},
    discovery::BuildTarget,
    model::GraphData,
    protocol::{Event, Request, VERSION},
};
use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

pub fn discover(runtime: &Runtime, project: &Path) -> Result<Vec<BuildTarget>> {
    let request = Request::Discover {
        version: VERSION,
        project: project.to_owned(),
    };
    match exchange(runtime, &request, &AtomicBool::new(false), &|_| {})? {
        Event::Discovered { targets } => Ok(targets),
        _ => bail!("Engine returned an analysis result for discovery"),
    }
}

pub fn clear_cache(runtime: &Runtime, project: &Path) -> Result<u32> {
    let request = Request::ClearCache {
        version: VERSION,
        project: project.to_owned(),
    };
    match exchange(runtime, &request, &AtomicBool::new(false), &|_| {})? {
        Event::CacheCleared { removed_files } => Ok(removed_files),
        _ => bail!("Engine returned an unexpected result for cache clearing"),
    }
}

pub struct Analysis {
    pub project: std::path::PathBuf,
    pub targets: Vec<BuildTarget>,
}

pub fn analyze(
    runtime: &Runtime,
    selection: Analysis,
    cancel: &AtomicBool,
    progress: &dyn Fn(String),
) -> Result<GraphData> {
    let project = selection
        .project
        .canonicalize()
        .context("Cannot open analysis project")?;
    let request = Request::Analyze {
        version: VERSION,
        project: project.clone(),
        roots: Vec::new(),
        infer_linked_roots: true,
        targets: selection.targets,
    };
    let Event::Complete {
        roots,
        digest,
        fingerprints,
        engine_versions,
        metrics,
    } = exchange(runtime, &request, cancel, progress)?
    else {
        bail!("Engine returned discovery results for analysis");
    };
    ensure!(!cancel.load(Ordering::Relaxed), "Analysis cancelled");
    let bytes =
        fs::read(project.join(GRAPH_PATH)).context("Cannot read published project graph")?;
    ensure!(
        blake3::hash(&bytes).to_hex().as_str() == digest,
        "Published graph differs from engine result"
    );
    let mut graph = Artifact::from_bytes(&bytes)?.hydrate(&roots)?;
    graph.fingerprints = fingerprints;
    graph.engine_versions = engine_versions;
    graph.validate()?;
    codecallnavigator::metrics::absorb(&metrics)?;
    Ok(graph)
}

fn exchange(
    runtime: &Runtime,
    request: &Request,
    cancel: &AtomicBool,
    progress: &dyn Fn(String),
) -> Result<Event> {
    ensure!(!cancel.load(Ordering::Relaxed), "Analysis cancelled");
    let mut child = process::Session::start(runtime, request)?;
    let mut conversation = Conversation::default();
    while let Some(event) = child.next(cancel)? {
        if let Some(message) = conversation.receive(event)? {
            progress(message);
        }
        if conversation.terminal.is_some() {
            child.expect_exit();
        }
    }
    let status = child.finish()?;
    ensure!(!cancel.load(Ordering::Relaxed), "Analysis cancelled");
    ensure!(
        status.success(),
        "Engine failed ({status}): {}",
        child.log_tail()
    );
    conversation
        .terminal
        .context("Engine exited without a terminal result")
}

#[derive(Default)]
struct Conversation {
    hello: bool,
    terminal: Option<Event>,
}

impl Conversation {
    fn receive(&mut self, event: Event) -> Result<Option<String>> {
        ensure!(
            self.terminal.is_none(),
            "Engine sent events after its terminal result"
        );
        if !self.hello {
            let Event::Hello { version } = event else {
                bail!("Engine did not begin with protocol hello");
            };
            ensure!(
                version == VERSION,
                "Unsupported engine protocol version: {version}"
            );
            self.hello = true;
            return Ok(None);
        }
        match event {
            Event::Hello { .. } => bail!("Engine sent a duplicate hello"),
            Event::Progress { message } => Ok(Some(message)),
            Event::Failed { message } => bail!("Engine analysis failed: {message}"),
            terminal => {
                self.terminal = Some(terminal);
                Ok(None)
            }
        }
    }
}
