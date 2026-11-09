pub(super) mod worker;
use anyhow::{Context, Result, bail, ensure};
use std::os::unix::process::CommandExt;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

pub(super) fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    ensure!(!cancel.load(Ordering::Relaxed), "Analysis cancelled");
    Ok(())
}

pub(super) fn run_process(
    command: &mut Command,
    job: &Path,
    name: &str,
    cancel: &AtomicBool,
) -> Result<PathBuf> {
    check_cancel(cancel)?;
    let _timing = crate::metrics::stage("subprocess");
    crate::metrics::count("subprocesses", 1);
    let stdout = job.join(format!("{name}.stdout"));
    let stderr = job.join(format!("{name}.stderr"));
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(fs::File::create(&stdout)?))
        .stderr(Stdio::from(fs::File::create(&stderr)?))
        .process_group(0);
    let mut child = command
        .spawn()
        .with_context(|| format!("Cannot start {}", command.get_program().to_string_lossy()))?;
    let started = Instant::now();
    loop {
        if cancel.load(Ordering::Relaxed) || started.elapsed() > Duration::from_secs(1800) {
            stop_process_group(&mut child);
            bail!(
                "Analysis cancelled or exceeded 30 minutes; logs: {}",
                job.display()
            );
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                ensure!(
                    status.success(),
                    "{name} failed ({status}). {}",
                    log_tail(&stderr)
                );
                break;
            }
            Ok(None) => thread::sleep(Duration::from_millis(100)),
            Err(error) => {
                stop_process_group(&mut child);
                return Err(error.into());
            }
        }
    }
    Ok(stdout)
}

fn stop_process_group(child: &mut std::process::Child) {
    let group = -(child.id() as i32);
    // Each child was created in its own process group above; never signal our UI's group.
    unsafe {
        libc::kill(group, libc::SIGTERM);
    }
    for _ in 0..20 {
        if child.try_wait().ok().flatten().is_some() {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    unsafe {
        libc::kill(group, libc::SIGKILL);
    }
    let _ = child.wait();
}

fn log_tail(path: &Path) -> String {
    let text = fs::read_to_string(path).unwrap_or_default();
    text.chars()
        .rev()
        .take(4000)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    #[test]
    fn analyzer_children_receive_eof_instead_of_the_controlling_terminal() {
        if let Some(directory) = std::env::var_os("CODEGRAPH_TEST_PROCESS_TTY") {
            assert_noninteractive_child(Path::new(&directory));
            return;
        }
        let directory = tempfile::tempdir().unwrap();
        let status = Command::new("python3")
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/process_tty.py"))
            .arg(std::env::current_exe().unwrap())
            .arg("analyzers::process::tests::analyzer_children_receive_eof_instead_of_the_controlling_terminal")
            .arg(directory.path())
            .status()
            .unwrap();
        assert!(
            status.success(),
            "Analyzer child inherited the controlling terminal"
        );
    }

    fn assert_noninteractive_child(directory: &Path) {
        assert_eq!(
            unsafe { libc::isatty(0) },
            1,
            "Helper must have a controlling terminal"
        );
        let marker = directory.join("stdin-child.pid");
        let output = run_process(
            Command::new("python3")
                .args(["-c", "import os,sys; from pathlib import Path; Path(sys.argv[1]).write_text(str(os.getpid())); data=os.read(0,1); assert data==b''; assert not os.isatty(0); print('stdin EOF')"])
                .arg(marker),
            directory,
            "stdin-child",
            &AtomicBool::new(false),
        ).unwrap();
        assert_eq!(fs::read_to_string(output).unwrap().trim(), "stdin EOF");
    }

    #[test]
    fn already_cancelled_process_never_starts() {
        let directory = tempfile::tempdir().unwrap();
        let marker = directory.path().join("started");
        let result = run_process(
            Command::new("/usr/bin/touch").arg(&marker),
            directory.path(),
            "cancelled",
            &AtomicBool::new(true),
        );
        assert!(result.is_err());
        assert!(!marker.exists());
    }

    #[test]
    fn cancellation_stops_descendants_in_the_owned_process_group() {
        let directory = tempfile::tempdir().unwrap();
        let started = directory.path().join("started");
        let late_write = directory.path().join("should-not-be-created");
        let cancel = Arc::new(AtomicBool::new(false));
        let trigger = cancel.clone();
        let marker = started.clone();
        let cancellation = thread::spawn(move || {
            for _ in 0..100 {
                if marker.exists() {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
            trigger.store(true, Ordering::Relaxed);
        });
        let result = run_process(
            Command::new("/bin/sh")
                .args([
                    "-c",
                    "touch \"$1\"; (sleep 1; touch \"$2\") & wait",
                    "codegraph-cancellation-test",
                ])
                .arg(&started)
                .arg(&late_write),
            directory.path(),
            "cancel-group",
            &cancel,
        );
        cancellation.join().unwrap();
        assert!(result.is_err());
        thread::sleep(Duration::from_millis(1200));
        assert!(
            !late_write.exists(),
            "A cancelled descendant continued executing"
        );
    }
}
