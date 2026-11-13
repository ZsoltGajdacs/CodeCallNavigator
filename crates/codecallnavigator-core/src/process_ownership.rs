//! Owned compiler-process cleanup shared by adapters and refresh supervision.
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeMap,
    io::Read,
    os::unix::process::CommandExt,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
struct Identity {
    pid: u32,
    started: (u64, u64),
}

impl Identity {
    fn capture(pid: u32) -> Option<Self> {
        Some(Self {
            pid,
            started: process_start(pid)?,
        })
    }
    fn alive(&self) -> bool {
        process_start(self.pid) == Some(self.started)
    }
    fn signal(&self, signal: i32) {
        if self.alive() {
            unsafe {
                libc::kill(-(self.pid as i32), signal);
                libc::kill(self.pid as i32, signal);
            }
        }
    }
}

/// Retains identities across reparenting; numeric PID reuse never authorizes a kill.
pub struct Descendants {
    owned: BTreeMap<u32, Identity>,
}

impl Descendants {
    pub fn new(child: &Child) -> Result<Self> {
        let owner =
            Identity::capture(child.id()).context("Cannot identify owned analysis process")?;
        Ok(Self {
            owned: [(owner.pid, owner)].into_iter().collect(),
        })
    }

    pub fn refresh(&mut self) -> Result<()> {
        let parents = process_parents()?;
        self.owned.retain(|_, identity| identity.alive());
        let mut pending = self.owned.keys().copied().collect::<Vec<_>>();
        while let Some(parent) = pending.pop() {
            for &(pid, owner) in &parents {
                if owner != parent || self.owned.contains_key(&pid) {
                    continue;
                }
                let Some(identity) = Identity::capture(pid) else {
                    continue;
                };
                self.owned.insert(pid, identity);
                pending.push(pid);
            }
        }
        Ok(())
    }

    pub fn stop(&mut self, child: &mut Child) -> Result<()> {
        let observed = self.refresh();
        self.signal(libc::SIGTERM);
        let _ = self.wait_stopped(child, Duration::from_secs(1));
        self.signal(libc::SIGKILL);
        ensure!(
            self.wait_stopped(child, Duration::from_secs(2))?,
            "Owned process cleanup could not be confirmed within its deadline"
        );
        observed.context("Owned process stopped, but descendant inventory could not be verified")
    }

    fn wait_stopped(&self, child: &mut Child, timeout: Duration) -> Result<bool> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            let reaped = child.try_wait()?.is_some();
            if reaped && !self.owned.values().any(Identity::alive) {
                return Ok(true);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        Ok(false)
    }

    fn signal(&self, signal: i32) {
        for identity in self.owned.values() {
            identity.signal(signal);
        }
    }
}

fn process_parents() -> Result<Vec<(u32, u32)>> {
    let log = tempfile::tempfile()?;
    let mut child = Command::new("/bin/ps")
        .args(["-axo", "pid=,ppid="])
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .context("Cannot enumerate owned descendants")?;
    let result = wait_inventory(&mut child);
    if result.is_err() {
        let _ = child.kill();
        if wait_inventory(&mut child).is_err() {
            reap_eventually(child);
        }
    }
    let status = result?;
    ensure!(status.success(), "Owned descendant enumeration failed");
    let mut text = String::new();
    use std::io::{Seek, SeekFrom};
    let mut log = log;
    log.seek(SeekFrom::Start(0))?;
    log.take(4 * 1024 * 1024 + 1).read_to_string(&mut text)?;
    ensure!(
        text.len() <= 4 * 1024 * 1024,
        "Owned descendant inventory exceeds 4 MiB"
    );
    Ok(text
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            Some((fields.next()?.parse().ok()?, fields.next()?.parse().ok()?))
        })
        .collect())
}

/// Transfer a direct child after bounded cleanup reports uncertainty. No PID
/// lookup is used: this exact Child remains owned until the kernel can reap it.
pub fn reap_eventually(mut child: Child) {
    std::thread::spawn(move || {
        let _ = child.wait();
    });
}

fn wait_inventory(child: &mut Child) -> Result<std::process::ExitStatus> {
    let deadline = Instant::now() + Duration::from_secs(2);
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

#[cfg(target_os = "macos")]
fn process_start(pid: u32) -> Option<(u64, u64)> {
    let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::uninit();
    let size = std::mem::size_of::<libc::proc_bsdinfo>();
    // proc_pidinfo fills exactly this documented BSD-info structure for this PID.
    let read = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            libc::PROC_PIDTBSDINFO,
            0,
            info.as_mut_ptr().cast(),
            size as i32,
        )
    };
    if read != size as i32 {
        return None;
    }
    let info = unsafe { info.assume_init() };
    Some((info.pbi_start_tvsec, info.pbi_start_tvusec))
}

#[cfg(target_os = "linux")]
fn process_start(pid: u32) -> Option<(u64, u64)> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let fields = stat
        .rsplit_once(") ")?
        .1
        .split_whitespace()
        .collect::<Vec<_>>();
    Some((fields.get(19)?.parse().ok()?, 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reaped_leader_does_not_abandon_an_independently_grouped_descendant() {
        let directory = tempfile::tempdir().unwrap();
        let marker = directory.path().join("worker.pid");
        let program = r#"import subprocess,sys,time
subprocess.Popen([sys.executable,'-c','import signal,time,os,pathlib,sys; signal.signal(signal.SIGTERM,signal.SIG_IGN); pathlib.Path(sys.argv[1]).write_text(str(os.getpid())); time.sleep(30)',sys.argv[1]],start_new_session=True)
time.sleep(30)
"#;
        let mut child = Command::new("python3")
            .args(["-c", program])
            .arg(&marker)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .unwrap();
        let mut owned = Descendants::new(&child).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while !marker.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        let pid: u32 = std::fs::read_to_string(marker).unwrap().parse().unwrap();
        owned.refresh().unwrap();
        unsafe {
            libc::kill(child.id() as i32, libc::SIGTERM);
        }
        child.wait().unwrap();
        owned.stop(&mut child).unwrap();
        assert!(
            process_start(pid).is_none(),
            "Independently grouped worker survived its reaped leader"
        );
    }

    #[test]
    fn process_identity_mismatch_never_authorizes_a_signal() {
        let mut child = Command::new("/bin/sleep").arg("30").spawn().unwrap();
        let mut identity = Identity::capture(child.id()).unwrap();
        identity.started = (0, 0);
        identity.signal(libc::SIGTERM);
        let alive = child.try_wait().unwrap().is_none();
        let _ = child.kill();
        let _ = child.wait();
        assert!(alive, "An obsolete PID identity signalled a newer process");
    }
}
