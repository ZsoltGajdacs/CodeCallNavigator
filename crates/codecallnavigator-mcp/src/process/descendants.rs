//! OS process inventory is used only for cleanup; no project files are inspected.
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeSet, process::Stdio, time::Duration};
use tokio::process::Command;

struct Process {
    pid: u32,
    parent: u32,
    group: u32,
}
pub struct Descendants {
    root: u32,
    identities: BTreeSet<u32>,
    groups: BTreeSet<u32>,
}
impl Descendants {
    pub async fn capture(root: u32) -> Result<Self> {
        let mut owned = Self {
            root,
            identities: BTreeSet::from([root]),
            groups: BTreeSet::new(),
        };
        owned.update().await?;
        Ok(owned)
    }
    pub async fn update(&mut self) -> Result<()> {
        let processes = inventory().await?;
        loop {
            let previous = self.identities.len();
            for process in &processes {
                if self.identities.contains(&process.parent) {
                    self.identities.insert(process.pid);
                }
            }
            if self.identities.len() == previous {
                break;
            }
        }
        // Only groups led by an owned PID are safe to target as a group.
        self.groups.extend(
            processes
                .iter()
                .filter(|process| {
                    self.identities.contains(&process.pid)
                        && self.identities.contains(&process.group)
                })
                .map(|process| process.group),
        );
        Ok(())
    }
    pub fn terminate(&self) {
        #[cfg(unix)]
        unsafe {
            for group in &self.groups {
                libc::kill(-(*group as i32), libc::SIGKILL);
            }
            for pid in self
                .identities
                .iter()
                .rev()
                .filter(|pid| **pid != self.root)
            {
                libc::kill(*pid as i32, libc::SIGKILL);
            }
        }
    }
}
async fn inventory() -> Result<Vec<Process>> {
    let mut command = Command::new("/bin/ps");
    command
        .args(["-A", "-o", "pid=,ppid=,pgid="])
        .stdin(Stdio::null())
        .kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_millis(500), command.output())
        .await
        .context("Process cleanup inventory timed out")??;
    ensure!(output.status.success(), "Process cleanup inventory failed");
    let text = std::str::from_utf8(&output.stdout).context("Invalid process inventory encoding")?;
    text.lines()
        .map(|line| {
            let fields = line
                .split_whitespace()
                .map(str::parse::<u32>)
                .collect::<std::result::Result<Vec<_>, _>>()?;
            ensure!(fields.len() == 3, "Invalid process inventory record");
            Ok(Process {
                pid: fields[0],
                parent: fields[1],
                group: fields[2],
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, BufReader};
    #[tokio::test]
    async fn cleanup_captures_a_descendant_that_started_its_own_process_group() {
        let script = "import subprocess,sys,time; subprocess.Popen([sys.executable,'-c','import os,time; os.setsid(); print(os.getpid(), flush=True); time.sleep(30)']); time.sleep(30)";
        let mut leader = Command::new("/usr/bin/python3")
            .args(["-c", script])
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .process_group(0)
            .spawn()
            .unwrap();
        let root = leader.id().unwrap();
        let mut line = String::new();
        tokio::time::timeout(
            Duration::from_secs(5),
            BufReader::new(leader.stdout.take().unwrap()).read_line(&mut line),
        )
        .await
        .unwrap()
        .unwrap();
        let descendant: u32 = line.trim().parse().unwrap();
        let mut owned = Descendants::capture(root).await.unwrap();
        assert!(
            owned.groups.contains(&descendant),
            "Detached child group was not captured"
        );
        owned.update().await.unwrap();
        owned.terminate();
        leader.start_kill().unwrap();
        leader.wait().await.unwrap();
        // SIGKILL reached the separate group; zombies may await their OS reaper.
        let output = Command::new("/bin/ps")
            .args(["-o", "stat=", "-p", &descendant.to_string()])
            .output()
            .await
            .unwrap();
        assert!(output.stdout.is_empty() || output.stdout.starts_with(b"Z"));
    }
}
