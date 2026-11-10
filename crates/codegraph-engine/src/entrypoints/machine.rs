use super::{analysis, cancellation::CANCELLED};
use anyhow::{Context, Result, ensure};
use codegraph_core::{
    discovery,
    protocol::{self, Event, Request},
};
use std::{
    io::{self, BufRead, BufReader, Read, Write},
    sync::{atomic::Ordering, mpsc},
    time::Duration,
};

const MAX_REQUEST_BYTES: u64 = 4 * 1024 * 1024;

fn emit(event: &Event) -> Result<()> {
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, event)?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}

pub fn serve_once() -> Result<i32> {
    emit(&Event::Hello {
        version: protocol::VERSION,
    })?;
    match request() {
        Ok(event) => {
            emit(&event)?;
            Ok(0)
        }
        Err(error) => {
            emit(&Event::Failed {
                message: format!("{error:#}"),
            })?;
            Err(error)
        }
    }
}

fn request() -> Result<Event> {
    let (line, input) = initial_request()?;
    let request: Request = serde_json::from_str(&line).context("Invalid engine request")?;
    ensure!(
        request.version() == protocol::VERSION,
        "Unsupported engine protocol version: {}",
        request.version()
    );
    match request {
        Request::Discover { project, .. } => Ok(Event::Discovered {
            targets: discovery::discover(&project)?,
        }),
        Request::Analyze {
            project,
            roots,
            targets,
            infer_linked_roots,
            ..
        } => {
            monitor(input);
            analysis::analyze_with_linked_roots(
                &project,
                analysis::RootOptions {
                    configured: roots,
                    infer_linked: infer_linked_roots,
                },
                targets,
                &|message| {
                    if emit(&Event::Progress { message }).is_err() {
                        CANCELLED.store(true, Ordering::Relaxed);
                    }
                },
            )
        }
    }
}

fn frame(input: &mut impl BufRead) -> Result<String> {
    let mut line = String::new();
    let count = input.take(MAX_REQUEST_BYTES + 1).read_line(&mut line)?;
    ensure!(count > 0, "No engine request received");
    ensure!(
        count as u64 <= MAX_REQUEST_BYTES,
        "Engine request exceeds 4 MiB"
    );
    Ok(line)
}

fn initial_request() -> Result<(String, BufReader<io::Stdin>)> {
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut input = BufReader::new(io::stdin());
        let result = frame(&mut input).map(|line| (line, input));
        let _ = sender.send(result);
    });
    loop {
        ensure!(
            !CANCELLED.load(Ordering::Relaxed),
            "Engine request cancelled"
        );
        match receiver.recv_timeout(Duration::from_millis(50)) {
            Ok(result) => return result,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(error) => return Err(error.into()),
        }
    }
}

fn monitor(mut input: impl BufRead + Send + 'static) {
    std::thread::spawn(move || {
        loop {
            match frame(&mut input) {
                Err(_) => break,
                Ok(line) => {
                    if line.trim() == "cancel" {
                        break;
                    }
                    let message: Result<serde_json::Value, _> = serde_json::from_str(&line);
                    if message.is_ok_and(|message| {
                        message["command"] == "cancel" && message["version"] == protocol::VERSION
                    }) {
                        break;
                    }
                }
            }
        }
        CANCELLED.store(true, Ordering::Relaxed);
    });
}
