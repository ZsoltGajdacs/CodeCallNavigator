//! The desktop consumes the same published artifact as standalone CLI users.
#[cfg(test)]
use codecallnavigator::model::GraphData;
mod analysis;
mod analysis_failure;
mod capabilities;
pub mod flow_comparison;
mod flow_expansion;
mod flow_resources;
mod observation_process;
pub(crate) use observation_process::{CleanupFailure, Notice, ObservationOptions};
mod process;
mod queries;
mod query_process;
#[cfg(test)]
use analysis::analyze;
pub use analysis::{Analysis, analyze_snapshot};
pub(crate) use analysis_failure::AnalysisFailure;
pub(crate) use queries::QueryFailure;
pub use queries::{Descriptor, QueryContext, Snapshot};
#[cfg(test)]
mod query_tests;
#[cfg(test)]
mod tests;

use crate::runtime::Runtime;
use anyhow::{Context, Result, bail, ensure};
use codecallnavigator::{
    discovery::BuildTarget,
    protocol::{AnalysisFailureCode, Event, Request, VERSION},
};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

pub fn discover(runtime: &Runtime, project: &Path) -> Result<Vec<BuildTarget>> {
    match exchange_quiet(
        runtime,
        Request::Discover {
            version: VERSION,
            project: project.to_path_buf(),
        },
    )? {
        Event::Discovered { targets } => Ok(targets),
        _ => bail!("Engine returned an analysis result for discovery"),
    }
}

pub fn clear_cache(runtime: &Runtime, project: &Path) -> Result<u32> {
    match exchange_quiet(
        runtime,
        Request::ClearCache {
            version: VERSION,
            project: project.to_path_buf(),
        },
    )? {
        Event::CacheCleared { removed_files } => Ok(removed_files),
        _ => bail!("Engine returned an unexpected result for cache clearing"),
    }
}

fn exchange_quiet(runtime: &Runtime, request: Request) -> Result<Event> {
    exchange(runtime, &request, &AtomicBool::new(false), &|_| {})
}

fn exchange(
    runtime: &Runtime,
    request: &Request,
    cancel: &AtomicBool,
    progress: &dyn Fn(String),
) -> Result<Event> {
    require_running(cancel)?;
    let mut child = process::Session::start(runtime, request)?;
    let mut conversation = Conversation {
        typed_failure: matches!(
            request,
            Request::Analyze { .. } | Request::PrepareSnapshot { .. }
        ),
        ..Default::default()
    };
    while let Some(event) = child.next(cancel)? {
        if let Some(message) = conversation.receive(event)? {
            progress(message);
        }
        if conversation.terminal.is_some() {
            child.expect_exit();
        }
    }
    let status = child.finish()?;
    require_running(cancel)?;
    ensure!(
        status.success(),
        "Engine failed ({status}): {}",
        child.log_tail()
    );
    conversation
        .terminal
        .context("Engine exited without a terminal result")
}

fn require_running(cancel: &AtomicBool) -> Result<()> {
    ensure!(
        !cancel.load(Ordering::Relaxed),
        AnalysisFailure {
            code: Some(AnalysisFailureCode::Cancelled),
            message: "Analysis cancelled".into(),
            target: None,
        }
    );
    Ok(())
}

#[derive(Default)]
struct Conversation {
    typed_failure: bool,
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
            Event::Failed {
                message,
                code,
                target,
            } => {
                if code.is_none() && self.typed_failure {
                    bail!(
                        "unsupported_capability: configured CLI omitted the typed analysis failure"
                    )
                }
                Err(AnalysisFailure {
                    code,
                    message,
                    target,
                }
                .into())
            }
            terminal => {
                self.terminal = Some(terminal);
                Ok(None)
            }
        }
    }
}
