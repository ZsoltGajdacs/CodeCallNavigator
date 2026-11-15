//! Spool finite layout input/output to owned files so pipe backpressure cannot
//! prevent cancellation. Every failed wait stops and reaps the exact child.
use crate::process_ownership::{Descendants, reap_eventually};
use anyhow::{Context, Result, ensure};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    os::unix::process::CommandExt,
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

const MAX_OUTPUT: u64 = 32 * 1024 * 1024;
const MAX_ERRORS: u64 = 64 * 1024;

pub fn run(dot: &Path, source: &str) -> Result<Vec<u8>> {
    run_cancellable(dot, source, &AtomicBool::new(false))
}

/// Cancellation includes owned process cleanup; no partial geometry is returned.
pub fn run_cancellable(dot: &Path, source: &str, cancel: &AtomicBool) -> Result<Vec<u8>> {
    check_cancel(cancel)?;
    let mut input = tempfile::tempfile().context("Cannot spool Graphviz input")?;
    input.write_all(source.as_bytes())?;
    input.seek(SeekFrom::Start(0))?;
    let output = tempfile::tempfile()?;
    let errors = tempfile::tempfile()?;
    let mut command = command(dot);
    command
        .stdin(Stdio::from(input))
        .stdout(Stdio::from(output.try_clone()?))
        .stderr(Stdio::from(errors.try_clone()?));
    check_cancel(cancel)?;
    let mut child = command
        .spawn()
        .context("Cannot start Graphviz; install or bundle dot")?;
    let mut owned = match Descendants::new(&child) {
        Ok(owned) => owned,
        Err(error) => {
            let _ = child.kill();
            reap_eventually(child);
            return Err(error.context("Cannot establish Graphviz process ownership"));
        }
    };
    let status = match wait(&mut child, cancel, &output, &errors) {
        Ok(status) => status,
        Err(error) => {
            if let Err(cleanup) = owned.stop(&mut child) {
                reap_eventually(child);
                return Err(error.context(format!("Graphviz cleanup failed: {cleanup:#}")));
            }
            return Err(error);
        }
    };
    check_cancel(cancel)?;
    let data = read(output, MAX_OUTPUT)?;
    let errors = read(errors, MAX_ERRORS)?;
    ensure!(
        status.success(),
        "Graphviz layout failed: {}",
        String::from_utf8_lossy(&errors)
    );
    Ok(data)
}

fn command(dot: &Path) -> Command {
    let mut command = Command::new(dot);
    let plugins = dot
        .parent()
        .and_then(Path::parent)
        .map(|path| path.join("lib/graphviz"));
    if let Some(path) = plugins.filter(|path| path.is_dir()) {
        command.env("GVBINDIR", path);
    }
    command.arg("-Tjson").process_group(0);
    command
}

fn wait(
    child: &mut Child,
    cancel: &AtomicBool,
    output: &File,
    errors: &File,
) -> Result<ExitStatus> {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        check_cancel(cancel)?;
        ensure!(
            output.metadata()?.len() <= MAX_OUTPUT,
            "Graphviz output exceeds 32 MiB"
        );
        ensure!(
            errors.metadata()?.len() <= MAX_ERRORS,
            "Graphviz diagnostics exceed 64 KiB"
        );
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        ensure!(
            Instant::now() < deadline,
            "Graphviz layout timed out; narrow the neighborhood or collapse groups"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn read(mut file: File, limit: u64) -> Result<Vec<u8>> {
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= limit,
        "Graphviz output exceeds its byte limit"
    );
    Ok(bytes)
}

fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    ensure!(!cancel.load(Ordering::Acquire), "Graphviz layout cancelled");
    Ok(())
}
