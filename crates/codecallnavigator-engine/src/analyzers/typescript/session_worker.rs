//! Versioned request/reply boundary for the persistent Node compiler worker.
use super::{Metadata, Output, adapter};
use crate::{
    analyzers::{AnalysisJob, find_tool, process::worker::Worker},
    discovery::BuildTarget,
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
    time::Duration,
};

const VERSION: u32 = 1;

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Operation {
    Update,
    Analyze,
}

#[derive(Serialize)]
struct Request {
    version: u32,
    sequence: u64,
    command: Operation,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Status {
    Success,
    Error,
    Restart,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Diagnostic {
    pub code: String,
    pub message: String,
    pub path: Option<PathBuf>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Reply {
    version: u32,
    sequence: u64,
    pub status: Status,
    pub metadata: Option<Metadata>,
    pub graph: Option<Output>,
    pub error: Option<Diagnostic>,
    pub affected: Option<u64>,
}

impl Reply {
    fn read(path: &Path, sequence: u64) -> Result<Self> {
        let reply: Self = serde_json::from_slice(&fs::read(path)?)
            .context("Invalid TypeScript worker response; rebuild matching analyzer resources")?;
        ensure!(
            reply.version == VERSION && reply.sequence == sequence,
            "TypeScript worker returned a different version or response sequence"
        );
        match reply.status {
            Status::Success => ensure!(
                reply.metadata.is_some() && reply.error.is_none(),
                "TypeScript worker success omitted metadata or included an error"
            ),
            Status::Error | Status::Restart => ensure!(
                reply.error.is_some() && reply.graph.is_none(),
                "Invalid TypeScript worker failure response"
            ),
        }
        Ok(reply)
    }

    pub fn into_metadata(self) -> Result<Metadata> {
        self.metadata
            .context("TypeScript worker omitted preparation metadata")
    }

    pub fn into_graph(self) -> Result<Output> {
        self.graph
            .context("TypeScript worker omitted analysis output")
    }
}

pub(super) struct CompilerWorker {
    worker: Worker,
    response: PathBuf,
    sequence: u64,
}

impl CompilerWorker {
    pub fn start(build: &BuildTarget, context: &AnalysisJob<'_>) -> Result<(Self, Metadata)> {
        let node = find_tool("node").context("TypeScript analysis requires installed Node 24+")?;
        let response = context.path.join("typescript-response.json");
        clear(&response)?;
        let worker = Worker::start(
            Command::new(node)
                .arg(adapter(context)?)
                .arg("worker")
                .arg(&build.path)
                .arg(&response),
            context.path,
            "typescript-worker",
            context.cancel,
        )?;
        let mut worker = Self {
            worker,
            response,
            sequence: 0,
        };
        let result = worker
            .receive(context)
            .and_then(|reply| match reply.status {
                Status::Success => {
                    ensure!(
                        reply.graph.is_none(),
                        "Invalid TypeScript worker startup response"
                    );
                    reply.into_metadata()
                }
                Status::Error | Status::Restart => Err(super::native_error(reply, build)),
            });
        match result {
            Ok(metadata) => Ok((worker, metadata)),
            Err(error) => worker.stop(Err(error)),
        }
    }

    pub fn request(&mut self, command: Operation, context: &AnalysisJob<'_>) -> Result<Reply> {
        clear(&self.response)?;
        self.sequence = self
            .sequence
            .checked_add(1)
            .context("TypeScript worker sequence overflow")?;
        let request = Request {
            version: VERSION,
            sequence: self.sequence,
            command,
        };
        self.worker.request(&serde_json::to_string(&request)?)?;
        let reply = self.receive(context)?;
        if matches!(reply.status, Status::Success) {
            let analyzed = matches!(command, Operation::Analyze);
            ensure!(
                reply.graph.is_some() == analyzed,
                "TypeScript worker returned the wrong output"
            );
        }
        Ok(reply)
    }

    fn receive(&mut self, context: &AnalysisJob<'_>) -> Result<Reply> {
        self.worker
            .wait_for_output(&self.response, context.cancel)?;
        let reply = Reply::read(&self.response, self.sequence)?;
        if let Some(affected) = reply.affected {
            crate::metrics::count("typescript.affected_diagnostics", affected);
        }
        Ok(reply)
    }

    pub fn stop<T>(&mut self, result: Result<T>) -> Result<T> {
        self.worker.complete(result)
    }

    pub fn close(&mut self) -> Result<()> {
        self.worker
            .finish(&AtomicBool::new(false), Duration::from_secs(5))
    }
}

fn clear(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => bail!(
            "Cannot clear TypeScript worker response {}: {error}",
            path.display()
        ),
    }
}
