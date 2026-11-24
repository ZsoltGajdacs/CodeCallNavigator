use super::{Identity, Observation, Process, SamplingUnavailable, reap_eventually};
use anyhow::{Context, Result, ensure};
use std::{
    io::{Read, Seek, SeekFrom},
    os::unix::process::CommandExt,
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

pub(super) fn process_inventory(timeout: Duration) -> Result<Vec<Process>> {
    let log = tempfile::tempfile()?;
    let mut child = Command::new("/bin/ps")
        .args(["-axo", "pid=,ppid=,pgid="])
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .context("Cannot enumerate owned descendants")?;
    let result = wait_inventory_bounded(&mut child, timeout);
    if result.is_err() {
        let _ = child.kill();
        if wait_inventory(&mut child).is_err() {
            reap_eventually(child);
        }
    }
    let status = result?;
    ensure!(status.success(), "Owned descendant enumeration failed");
    let mut text = String::new();
    let mut log = log;
    log.seek(SeekFrom::Start(0))?;
    log.take(4 * 1024 * 1024 + 1).read_to_string(&mut text)?;
    ensure!(
        text.len() <= 4 * 1024 * 1024,
        "Owned descendant inventory exceeds 4 MiB"
    );
    parse_processes(&text)
}

fn parse_processes(text: &str) -> Result<Vec<Process>> {
    let mut seen = std::collections::BTreeSet::new();
    let mut processes = Vec::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let values = line
            .split_whitespace()
            .map(str::parse::<u32>)
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("Malformed owned descendant inventory number")?;
        ensure!(
            values.len() == 3 && values.iter().all(|value| *value <= i32::MAX as u32),
            "Malformed owned descendant inventory row"
        );
        ensure!(
            seen.insert(values[0]),
            "Duplicate PID in owned descendant inventory"
        );
        processes.push(Process {
            pid: values[0],
            parent: values[1],
            group: values[2],
            started: None,
        });
    }
    ensure!(
        seen.contains(&std::process::id()),
        "Owned descendant inventory omitted its observer"
    );
    Ok(processes)
}

fn wait_inventory(child: &mut Child) -> Result<ExitStatus> {
    wait_inventory_bounded(child, Duration::from_secs(2))
}

fn wait_inventory_bounded(child: &mut Child, timeout: Duration) -> Result<ExitStatus> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        ensure!(
            Instant::now() < deadline,
            "Owned descendant enumeration timed out"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

pub(super) fn process_observation(pid: u32) -> Result<Option<Observation>> {
    let stat = match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => stat,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(SamplingUnavailable::Identity {
                pid,
                detail: error.to_string(),
            }
            .into());
        }
    };
    let fields = stat
        .rsplit_once(") ")
        .context("Malformed owned process stat record")?
        .1
        .split_whitespace()
        .collect::<Vec<_>>();
    let started = fields
        .get(19)
        .context("Missing owned process birth identity")?
        .parse()
        .context("Malformed owned process birth identity")?;
    let parent = fields
        .get(1)
        .context("Missing owned process parent")?
        .parse()
        .context("Malformed owned process parent")?;
    let group = fields
        .get(2)
        .context("Missing owned process group")?
        .parse()
        .context("Malformed owned process group")?;
    Ok(Some(Observation {
        identity: Identity {
            pid,
            started: (started, 0),
        },
        parent,
        group,
    }))
}
