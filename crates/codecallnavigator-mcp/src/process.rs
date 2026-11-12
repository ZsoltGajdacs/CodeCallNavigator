//! Each operation owns a CLI process. Dropping its future cancels the supervisor.
mod descendants;
use anyhow::{Context, Result, bail, ensure};
use std::{path::Path, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, Command},
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

const MAX_INPUT: usize = 4 * 1024 * 1024;
const MAX_OUTPUT: usize = 32 * 1024 * 1024;
const MAX_STDERR: usize = 64 * 1024;
const CLEANUP: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub struct Output {
    pub stdout: Vec<u8>,
    pub stderr: String,
    pub success: bool,
    pub cleanup_diagnostic: Option<String>,
}
struct CancelOnDrop(CancellationToken);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

pub struct Invocation {
    pub arguments: Vec<String>,
    pub input: Option<Vec<u8>>,
    pub mode: StdinMode,
    pub deadline: Duration,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StdinMode {
    Close,
    Analysis,
}
struct Signals {
    cancel: CancellationToken,
    shutdown: CancellationToken,
}
struct OutputPipes {
    stdout: tokio::process::ChildStdout,
    stderr: tokio::process::ChildStderr,
}

#[derive(Clone, Default)]
pub struct Processes {
    shutdown: CancellationToken,
    tasks: TaskTracker,
}
impl Processes {
    pub async fn shutdown(&self) {
        self.shutdown.cancel();
        self.tasks.close();
        self.tasks.wait().await;
    }
    pub async fn run(
        &self,
        engine: &Path,
        invocation: Invocation,
        cancel: &CancellationToken,
    ) -> Result<Output> {
        ensure!(
            !cancel.is_cancelled() && !self.shutdown.is_cancelled(),
            "CLI operation cancelled"
        );
        ensure!(
            invocation
                .input
                .as_ref()
                .is_none_or(|b| b.len() <= MAX_INPUT),
            "CLI request exceeds 4 MiB"
        );
        let token = cancel.child_token();
        let guard = CancelOnDrop(token.clone());
        let mut command = Command::new(engine);
        command
            .args(&invocation.arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(unix)]
        command.process_group(0);
        let child = command.spawn().context("Cannot start configured CLI")?;
        let task = self.tasks.spawn(supervise(
            child,
            invocation,
            Signals {
                cancel: token,
                shutdown: self.shutdown.clone(),
            },
        ));
        let result = task.await.context("CLI supervisor failed")?;
        drop(guard);
        result
    }
}
struct OwnedGroup(Option<u32>);
impl Drop for OwnedGroup {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(pid) = self.0 {
            // Every CLI is spawned into its own process group; never target our MCP group.
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
        }
    }
}

async fn supervise(mut child: Child, invocation: Invocation, signals: Signals) -> Result<Output> {
    let keep_stdin = invocation.mode == StdinMode::Analysis;
    let deadline = invocation.deadline;
    let initial_pid = child.id();
    let group = OwnedGroup(initial_pid);
    let mut stdin = child.stdin.take();
    let stdout = child.stdout.take().context("CLI stdout unavailable")?;
    let stderr = child.stderr.take().context("CLI stderr unavailable")?;
    let io = exchange(
        &mut child,
        &mut stdin,
        OutputPipes { stdout, stderr },
        invocation,
    );
    let mut result = tokio::select! {
        result = io => result,
        _ = signals.shutdown.cancelled() => Err(anyhow::anyhow!("MCP disconnected")),
        _ = signals.cancel.cancelled() => Err(anyhow::anyhow!("CLI operation cancelled")),
        _ = tokio::time::sleep(deadline) => Err(anyhow::anyhow!("CLI operation timed out")),
    };
    let unsuccessful_analysis = keep_stdin && result.as_ref().is_ok_and(|output| !output.success);
    if result.is_err() || unsuccessful_analysis {
        let diagnostic = cleanup(&mut child, &mut stdin, keep_stdin, initial_pid)
            .await
            .err();
        if let Some(diagnostic) = diagnostic {
            result = match result {
                Err(original) => Err(anyhow::anyhow!(
                    "{original:#}; descendant cleanup could not be confirmed: {diagnostic:#}"
                )),
                Ok(mut output) => {
                    output.cleanup_diagnostic = Some(format!("{diagnostic:#}"));
                    Ok(output)
                }
            };
        }
    }
    drop(group);
    result
}

async fn exchange(
    child: &mut Child,
    stdin: &mut Option<ChildStdin>,
    pipes: OutputPipes,
    invocation: Invocation,
) -> Result<Output> {
    let (stdout, stderr, status) = tokio::try_join!(
        protocol_io(stdin, pipes.stdout, invocation),
        drain_stderr(pipes.stderr),
        async { child.wait().await.map_err(anyhow::Error::from) }
    )?;
    Ok(Output {
        stdout,
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        success: status.success(),
        cleanup_diagnostic: None,
    })
}

async fn protocol_io(
    stdin: &mut Option<ChildStdin>,
    stdout: tokio::process::ChildStdout,
    invocation: Invocation,
) -> Result<Vec<u8>> {
    let keep_stdin = invocation.mode == StdinMode::Analysis;
    let mut stdout = BufReader::new(stdout);
    let prefix = if keep_stdin {
        analysis_hello(&mut stdout).await?
    } else {
        Vec::new()
    };
    let write = async {
        if let Some(input) = invocation.input {
            let pipe = stdin.as_mut().context("CLI stdin unavailable")?;
            pipe.write_all(&input).await?;
            pipe.flush().await?;
        }
        if !keep_stdin {
            stdin.take();
        }
        anyhow::Ok(())
    };
    let ((), mut stdout) = tokio::try_join!(write, bounded_stdout(stdout))?;
    let mut combined = prefix;
    combined.append(&mut stdout);
    ensure!(combined.len() <= MAX_OUTPUT, "CLI response exceeds 32 MiB");
    Ok(combined)
}

async fn analysis_hello(reader: &mut (impl AsyncRead + Unpin)) -> Result<Vec<u8>> {
    let mut line = Vec::new();
    loop {
        ensure!(line.len() < 4096, "CLI hello exceeds 4 KiB");
        let byte = reader
            .read_u8()
            .await
            .context("CLI omitted protocol hello")?;
        line.push(byte);
        if byte == b'\n' {
            break;
        }
    }
    let hello: serde_json::Value =
        serde_json::from_slice(&line).context("Invalid CLI protocol hello")?;
    ensure!(
        hello == serde_json::json!({"event":"hello","version":1}),
        "Incompatible CLI analysis protocol hello"
    );
    Ok(line)
}

async fn bounded_stdout(mut reader: impl AsyncRead + Unpin) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    (&mut reader)
        .take(MAX_OUTPUT as u64 + 1)
        .read_to_end(&mut bytes)
        .await?;
    ensure!(bytes.len() <= MAX_OUTPUT, "CLI response exceeds 32 MiB");
    Ok(bytes)
}
async fn drain_stderr(mut reader: impl AsyncRead + Unpin) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        let count = reader.read(&mut buffer).await?;
        if count == 0 {
            return Ok(bytes);
        }
        let remaining = MAX_STDERR.saturating_sub(bytes.len());
        bytes.extend_from_slice(&buffer[..count.min(remaining)]);
    }
}
async fn analysis_descendants(
    current: Option<u32>,
    initial: Option<u32>,
    cooperative: bool,
) -> Result<Option<descendants::Descendants>> {
    if !cooperative {
        return Ok(None);
    }
    match current {
        Some(pid) => descendants::Descendants::capture(pid).await.map(Some),
        None => Err(anyhow::anyhow!(
            "Analysis CLI PID {initial:?} exited before live descendant inventory; independently grouped descendants cannot be confirmed stopped"
        )),
    }
}

async fn cleanup(
    child: &mut Child,
    stdin: &mut Option<ChildStdin>,
    cooperative: bool,
    initial_pid: Option<u32>,
) -> Result<()> {
    let captured = analysis_descendants(child.id(), initial_pid, cooperative).await;
    let (mut descendants, mut failure) = match captured {
        Ok(owned) => (owned, None),
        Err(error) => (None, Some(error)),
    };
    // Inventory precedes EOF so independently grouped analyzer descendants remain owned.
    stdin.take();
    let forced = !cooperative || tokio::time::timeout(CLEANUP, child.wait()).await.is_err();
    if let Some(descendants) = &mut descendants {
        if let Err(error) = descendants.update().await {
            failure = Some(error);
        }
        descendants.terminate();
    }
    if forced {
        let _ = child.start_kill();
        let _ = tokio::time::timeout(CLEANUP, child.wait()).await;
    }
    if let Some(error) = failure {
        return Err(error);
    }
    Ok(())
}

pub fn require_success(output: &Output) -> Result<()> {
    if !output.success {
        bail!(
            "CLI exited unsuccessfully: {} {}",
            output.stderr.trim(),
            output.cleanup_diagnostic.as_deref().unwrap_or("")
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn unexpected_analysis_exit_reports_unverified_orphan_cleanup() {
        let processes = Processes::default();
        let output = processes
            .run(
                Path::new("/bin/sh"),
                Invocation {
                    arguments: vec![
                        "-c".into(),
                        "printf '{\"event\":\"hello\",\"version\":1}\\n'; read request; exit 7"
                            .into(),
                    ],
                    input: Some(b"{}\n".to_vec()),
                    mode: StdinMode::Analysis,
                    deadline: Duration::from_secs(2),
                },
                &CancellationToken::new(),
            )
            .await
            .unwrap();
        assert!(!output.success);
        assert!(
            output
                .cleanup_diagnostic
                .unwrap()
                .contains("independently grouped descendants cannot be confirmed stopped")
        );
        processes.shutdown().await;
    }
    #[tokio::test]
    async fn query_error_exit_does_not_require_analysis_process_inventory() {
        let processes = Processes::default();
        let output = processes
            .run(
                Path::new("/bin/sh"),
                Invocation {
                    arguments: vec!["-c".into(), "printf '{}'; exit 2".into()],
                    input: None,
                    mode: StdinMode::Close,
                    deadline: Duration::from_secs(2),
                },
                &CancellationToken::new(),
            )
            .await
            .unwrap();
        assert!(!output.success);
        assert!(output.cleanup_diagnostic.is_none());
        processes.shutdown().await;
    }
    #[tokio::test]
    async fn stderr_is_drained_before_the_analysis_hello_arrives() {
        let processes = Processes::default();
        let script = "import sys,json; sys.stderr.write('x'*1048576); sys.stderr.flush(); print(json.dumps({'event':'hello','version':1}),flush=True); json.loads(sys.stdin.readline()); print(json.dumps({'event':'complete','digest':'a'*64,'roots':[],'fingerprints':[],'metrics':{},'engine_versions':'test'}),flush=True)";
        let output = processes
            .run(
                Path::new("/usr/bin/python3"),
                Invocation {
                    arguments: vec!["-c".into(), script.into()],
                    input: Some(b"{}\n".to_vec()),
                    mode: StdinMode::Analysis,
                    deadline: Duration::from_secs(2),
                },
                &CancellationToken::new(),
            )
            .await
            .unwrap();
        assert!(output.success);
        assert_eq!(output.stderr.len(), MAX_STDERR);
        let events: Vec<serde_json::Value> = std::str::from_utf8(&output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1]["event"], "complete");
        processes.shutdown().await;
    }
    #[tokio::test]
    async fn cancellation_interrupts_a_blocked_stdin_write() {
        let processes = Processes::default();
        let cancel = CancellationToken::new();
        let running = processes.clone();
        let token = cancel.clone();
        let task = tokio::spawn(async move {
            running
                .run(
                    Path::new("/bin/sh"),
                    Invocation {
                        arguments: vec!["-c".into(), "sleep 30".into()],
                        input: Some(vec![b'x'; 1024 * 1024]),
                        mode: StdinMode::Close,
                        deadline: Duration::from_secs(30),
                    },
                    &token,
                )
                .await
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        cancel.cancel();
        let result = tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap();
        assert!(result.unwrap_err().to_string().contains("cancelled"));
        processes.shutdown().await;
    }
    #[tokio::test]
    async fn dropping_request_future_still_drains_its_owned_supervisor() {
        let processes = Processes::default();
        let running = processes.clone();
        let task = tokio::spawn(async move {
            running
                .run(
                    Path::new("/bin/sh"),
                    Invocation {
                        arguments: vec!["-c".into(), "sleep 30 & wait".into()],
                        input: None,
                        mode: StdinMode::Close,
                        deadline: Duration::from_secs(30),
                    },
                    &CancellationToken::new(),
                )
                .await
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        task.abort();
        let _ = task.await;
        tokio::time::timeout(Duration::from_secs(2), processes.shutdown())
            .await
            .unwrap();
        assert!(processes.tasks.is_empty());
    }
    #[tokio::test]
    async fn analysis_hello_is_checked_before_any_request_is_written() {
        let processes = Processes::default();
        let result = processes
            .run(
                Path::new("/bin/sh"),
                Invocation {
                    arguments: vec![
                        "-c".into(),
                        "printf '{\"event\":\"hello\",\"version\":2}\\n'; read request; exit 7"
                            .into(),
                    ],
                    input: Some(b"analysis must not be sent\n".to_vec()),
                    mode: StdinMode::Analysis,
                    deadline: Duration::from_secs(2),
                },
                &CancellationToken::new(),
            )
            .await;
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Incompatible CLI analysis protocol hello")
        );
        processes.shutdown().await;
    }
}
