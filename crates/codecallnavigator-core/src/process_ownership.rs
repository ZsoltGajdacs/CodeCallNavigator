//! Owned compiler-process cleanup shared by adapters and refresh supervision.
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeMap,
    io::Read,
    os::unix::process::CommandExt,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

/// Native identity lookup was indeterminate; this never authorizes a signal.
#[derive(Debug)]
struct IdentityUnavailable {
    pid: u32,
    detail: String,
}

impl std::fmt::Display for IdentityUnavailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Cannot read birth identity for owned process {}: {}",
            self.pid, self.detail
        )
    }
}
impl std::error::Error for IdentityUnavailable {}

#[derive(Clone, Debug)]
struct Identity {
    pid: u32,
    started: (u64, u64),
}

impl Identity {
    fn capture(pid: u32) -> Result<Option<Self>> {
        Ok(process_observation(pid)?.map(|observed| observed.identity))
    }

    fn alive(&self) -> Result<bool> {
        Ok(process_start(self.pid)? == Some(self.started))
    }

    fn present_in(&self, processes: &[Process]) -> Result<bool> {
        if !processes.iter().any(|row| row.pid == self.pid) {
            return Ok(false);
        }
        self.alive()
    }

    fn signal(&self, signal: i32) -> Result<()> {
        if self.alive()? {
            signal_pid(self.pid as i32, signal)?;
        }
        Ok(())
    }
}

struct Process {
    pid: u32,
    parent: u32,
    group: u32,
}

struct Observation {
    identity: Identity,
    parent: u32,
    group: u32,
}

enum Relationship {
    Parent(Identity),
    SpawnedGroup,
}

enum GroupObservation {
    Unknown,
    Members(std::collections::BTreeSet<u32>),
    Retired,
}

struct SpawnedGroup {
    pid: u32,
    leader: Option<Identity>,
    observation: GroupObservation,
}

impl SpawnedGroup {
    fn observe(&mut self, processes: &[Process]) -> Result<Vec<u32>> {
        if matches!(self.observation, GroupObservation::Retired) {
            return Ok(Vec::new());
        }
        let members = processes
            .iter()
            .filter(|row| row.group == self.pid)
            .map(|row| row.pid)
            .collect::<std::collections::BTreeSet<_>>();
        if members.is_empty() || self.leader_replaced()? {
            self.observation = GroupObservation::Retired;
            return Ok(Vec::new());
        }
        ensure!(
            self.leader.is_some(),
            "Spawned process group {} has members but its original leader birth identity was unavailable",
            self.pid
        );
        let result = members.iter().copied().collect();
        self.observation = GroupObservation::Members(members);
        Ok(result)
    }

    fn leader_replaced(&self) -> Result<bool> {
        let Some(original) = &self.leader else {
            return Ok(false);
        };
        Ok(process_start(self.pid)?.is_some_and(|started| started != original.started))
    }

    fn signal<'a>(
        &'a self,
        identities: &BTreeMap<u32, Identity>,
        signal: i32,
    ) -> Result<Option<&'a std::collections::BTreeSet<u32>>> {
        let GroupObservation::Members(members) = &self.observation else {
            return Ok(None);
        };
        for pid in members {
            if self.current_member(*pid, identities)? {
                signal_pid(-(self.pid as i32), signal)?;
                return Ok(Some(members));
            }
        }
        Ok(None)
    }

    fn current_member(&self, pid: u32, identities: &BTreeMap<u32, Identity>) -> Result<bool> {
        let Some(identity) = identities.get(&pid) else {
            return Ok(false);
        };
        let Some(current) = process_observation(pid)? else {
            return Ok(false);
        };
        if current.identity.started != identity.started {
            return Ok(false);
        }
        Ok(current.group == self.pid)
    }

    fn owns_current(&self, current: &Observation) -> Result<bool> {
        if current.group != self.pid {
            return Ok(false);
        }
        if !matches!(self.observation, GroupObservation::Members(_)) {
            return Ok(false);
        }
        Ok(!self.leader_replaced()?)
    }
}

/// Retains identities across reparenting; numeric PID reuse never authorizes a kill.
pub struct Descendants {
    owned: BTreeMap<u32, Identity>,
    unidentified: std::collections::BTreeSet<u32>,
    group: Option<SpawnedGroup>,
}

impl Descendants {
    pub fn new(child: &Child) -> Result<Self> {
        let owner =
            Identity::capture(child.id())?.context("Cannot identify owned analysis process")?;
        Ok(Self {
            owned: [(owner.pid, owner)].into_iter().collect(),
            unidentified: Default::default(),
            group: None,
        })
    }

    /// Call immediately after this caller spawns with `Command::process_group(0)`,
    /// before reaping the child. That spawn fact owns exactly PGID=child.id(). A
    /// fast exited child need not remain queryable; unknown identity errors fail.
    pub fn for_group(child: &Child) -> Result<Self> {
        let leader = Identity::capture(child.id())?;
        let owned = leader
            .iter()
            .cloned()
            .map(|identity| (identity.pid, identity))
            .collect();
        Ok(Self {
            owned,
            unidentified: Default::default(),
            group: Some(SpawnedGroup {
                pid: child.id(),
                leader,
                observation: GroupObservation::Unknown,
            }),
        })
    }

    pub fn refresh(&mut self) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(2);
        let retry_until = Instant::now() + Duration::from_millis(100);
        loop {
            let result = self.refresh_bounded(deadline.saturating_duration_since(Instant::now()));
            let retryable = result
                .as_ref()
                .is_err_and(|error| error.is::<IdentityUnavailable>());
            if !retryable || Instant::now() >= retry_until {
                return result;
            }
            // Re-enumerate immediately: a coherent new table can prove a short-
            // lived candidate absent without guessing from a denied PID query.
        }
    }

    fn refresh_bounded(&mut self, timeout: Duration) -> Result<()> {
        if let Some(group) = &mut self.group
            && !matches!(group.observation, GroupObservation::Retired)
        {
            group.observation = GroupObservation::Unknown;
        }
        let processes = process_inventory(timeout)?;
        let mut pending = self.refresh_known(&processes)?;
        if let Some(group) = &mut self.group {
            let members = group.observe(&processes)?;
            for pid in members {
                self.capture_descendant(pid, &Relationship::SpawnedGroup)?;
                pending.push(pid);
            }
        }
        self.discover_children(&processes, pending)?;
        self.confirm_unidentified(&processes)
    }

    fn refresh_known(&mut self, processes: &[Process]) -> Result<Vec<u32>> {
        let mut parents = Vec::new();
        for identity in self.owned.values().cloned().collect::<Vec<_>>() {
            if identity.present_in(processes)? {
                parents.push(identity.pid);
            } else {
                // A dead or reused numeric PID cannot authorize ancestry.
                self.owned.remove(&identity.pid);
            }
        }
        Ok(parents)
    }

    fn capture_descendant(&mut self, pid: u32, relationship: &Relationship) -> Result<()> {
        match self.capture_related(pid, relationship) {
            Ok(Some(identity)) => {
                self.owned.insert(pid, identity);
                self.unidentified.remove(&pid);
            }
            Ok(None) => {
                self.owned.remove(&pid);
                self.unidentified.remove(&pid);
            }
            Err(error) => {
                self.unidentified.insert(pid);
                return Err(error);
            }
        }
        Ok(())
    }

    fn capture_related(&self, pid: u32, relationship: &Relationship) -> Result<Option<Identity>> {
        let Some(current) = process_observation(pid)? else {
            return Ok(None);
        };
        if self
            .owned
            .get(&pid)
            .is_some_and(|previous| previous.started == current.identity.started)
        {
            return Ok(Some(current.identity));
        }
        let related = match relationship {
            Relationship::Parent(parent) => current.parent == parent.pid && parent.alive()?,
            Relationship::SpawnedGroup => match &self.group {
                Some(group) => group.owns_current(&current)?,
                None => false,
            },
        };
        ensure!(
            related,
            "Descendant {pid} changed relationship before ownership could be confirmed"
        );
        Ok(Some(current.identity))
    }

    fn discover_children(&mut self, processes: &[Process], mut pending: Vec<u32>) -> Result<()> {
        let mut visited = std::collections::BTreeSet::new();
        while let Some(parent) = pending.pop() {
            if !visited.insert(parent) {
                continue;
            }
            pending.extend(self.discover_children_of(processes, parent)?);
        }
        Ok(())
    }

    fn discover_children_of(&mut self, processes: &[Process], parent: u32) -> Result<Vec<u32>> {
        let children = processes
            .iter()
            .filter(|row| row.parent == parent)
            .map(|row| row.pid)
            .collect::<Vec<_>>();
        let Some(parent) = self.owned.get(&parent).cloned() else {
            return Ok(Vec::new());
        };
        for pid in &children {
            if !self.owned.contains_key(pid) {
                self.capture_descendant(*pid, &Relationship::Parent(parent.clone()))?;
            }
        }
        Ok(children)
    }

    fn confirm_unidentified(&mut self, processes: &[Process]) -> Result<()> {
        for pid in self.unidentified.iter().copied().collect::<Vec<_>>() {
            if !processes.iter().any(|row| row.pid == pid) {
                self.unidentified.remove(&pid);
            }
        }
        if let Some(pid) = self.unidentified.iter().next() {
            return Err(IdentityUnavailable {
                pid: *pid,
                detail: "candidate remains present without a proven identity".into(),
            }
            .into());
        }
        Ok(())
    }

    pub fn stop(&mut self, child: &mut Child) -> Result<()> {
        let observed = self.refresh();
        let term = self.signal(libc::SIGTERM);
        let _ = self.wait_stopped(child, Duration::from_secs(1));
        let killed = self.signal(libc::SIGKILL);
        // The exact Child remains safe to kill even when identity observation is
        // unavailable. Its direct exit alone never proves descendant cleanup.
        let direct = if child.try_wait()?.is_none() {
            child.kill().map_err(Into::into)
        } else {
            Ok(())
        };
        ensure!(
            self.wait_stopped(child, Duration::from_secs(2))?,
            "Owned process cleanup could not be confirmed within its deadline"
        );
        // The final strict observation proved the reaped leader, known members,
        // unidentified candidates and owned group empty. Only transient native
        // identity queries can be superseded by that positive cleanup proof.
        resolved_after_cleanup(observed)
            .context("Owned process stopped, but descendant inventory could not be verified")?;
        resolved_after_cleanup(term)?;
        resolved_after_cleanup(killed)?;
        direct
    }

    fn wait_stopped(&mut self, child: &mut Child, timeout: Duration) -> Result<bool> {
        let deadline = Instant::now() + timeout;
        let mut last_error = None;
        while Instant::now() < deadline {
            let reaped = child.try_wait()?.is_some();
            match self.refresh_bounded(deadline.saturating_duration_since(Instant::now())) {
                Ok(()) => {
                    if self.cleanup_complete(reaped) {
                        return Ok(true);
                    }
                    last_error = None;
                }
                Err(error) => last_error = Some(error),
            }
            std::thread::sleep(
                Duration::from_millis(20).min(deadline.saturating_duration_since(Instant::now())),
            );
        }
        if let Some(error) = last_error {
            return Err(error);
        }
        Ok(false)
    }

    fn cleanup_complete(&self, reaped: bool) -> bool {
        if !reaped {
            return false;
        }
        if !self.owned.is_empty() {
            return false;
        }
        if !self.unidentified.is_empty() {
            return false;
        }
        self.group
            .as_ref()
            .is_none_or(|group| matches!(group.observation, GroupObservation::Retired))
    }

    fn signal(&self, signal: i32) -> Result<()> {
        let mut failure = None;
        let covered = match self
            .group
            .as_ref()
            .map(|group| group.signal(&self.owned, signal))
        {
            Some(Ok(members)) => members,
            Some(Err(error)) => {
                failure = Some(error);
                None
            }
            None => None,
        };
        for identity in self.owned.values() {
            if covered.is_some_and(|members| members.contains(&identity.pid)) {
                continue;
            }
            if let Err(error) = identity.signal(signal) {
                retain_signal_failure(&mut failure, error);
            }
        }
        failure.map_or(Ok(()), Err)
    }
}

fn retain_signal_failure(failure: &mut Option<anyhow::Error>, error: anyhow::Error) {
    // A real signal failure takes precedence over transient identity sampling.
    if failure
        .as_ref()
        .is_none_or(|previous| previous.is::<IdentityUnavailable>())
    {
        *failure = Some(error);
    }
}

fn resolved_after_cleanup(result: Result<()>) -> Result<()> {
    match result {
        Err(error) if error.is::<IdentityUnavailable>() => Ok(()),
        other => other,
    }
}

fn signal_pid(pid: i32, signal: i32) -> Result<()> {
    if unsafe { libc::kill(pid, signal) } == 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        return Ok(());
    }
    Err(error).with_context(|| format!("Cannot signal owned process {pid} with signal {signal}"))
}

fn process_inventory(timeout: Duration) -> Result<Vec<Process>> {
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
    use std::io::{Seek, SeekFrom};
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
        });
    }
    ensure!(
        seen.contains(&std::process::id()),
        "Owned descendant inventory omitted its observer"
    );
    Ok(processes)
}

/// Transfer a direct child after bounded cleanup reports uncertainty. No PID
/// lookup is used: this exact Child remains owned until the kernel can reap it.
pub fn reap_eventually(mut child: Child) {
    std::thread::spawn(move || {
        let _ = child.wait();
    });
}

fn wait_inventory(child: &mut Child) -> Result<std::process::ExitStatus> {
    wait_inventory_bounded(child, Duration::from_secs(2))
}

fn wait_inventory_bounded(
    child: &mut Child,
    timeout: Duration,
) -> Result<std::process::ExitStatus> {
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

fn process_start(pid: u32) -> Result<Option<(u64, u64)>> {
    Ok(process_observation(pid)?.map(|observed| observed.identity.started))
}

#[cfg(target_os = "macos")]
fn process_observation(pid: u32) -> Result<Option<Observation>> {
    let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::uninit();
    let size = std::mem::size_of::<libc::proc_bsdinfo>();
    // Clear errno so a short/unknown response cannot inherit an old ESRCH.
    let read = unsafe {
        *libc::__error() = 0;
        libc::proc_pidinfo(
            pid as i32,
            libc::PROC_PIDTBSDINFO,
            0,
            info.as_mut_ptr().cast(),
            size as i32,
        )
    };
    if read != size as i32 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ESRCH) || positively_absent(pid) {
            return Ok(None);
        }
        return Err(IdentityUnavailable {
            pid,
            detail: format!("returned {read} bytes: {error}"),
        }
        .into());
    }
    let info = unsafe { info.assume_init() };
    ensure!(
        info.pbi_pid == pid,
        "Mismatched native identity for owned process {pid}"
    );
    Ok(Some(Observation {
        identity: Identity {
            pid,
            started: (info.pbi_start_tvsec, info.pbi_start_tvusec),
        },
        parent: info.pbi_ppid,
        group: info.pbi_pgid,
    }))
}

#[cfg(target_os = "macos")]
fn positively_absent(pid: u32) -> bool {
    if unsafe { libc::kill(pid as i32, 0) } == 0 {
        return false;
    }
    std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
}

#[cfg(target_os = "linux")]
fn process_observation(pid: u32) -> Result<Option<Observation>> {
    let stat = match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => stat,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(IdentityUnavailable {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reaped_leader_does_not_abandon_an_independently_grouped_descendant() {
        assert_reaped_worker_stopped(true);
        assert_reaped_worker_stopped(false);
    }

    fn assert_reaped_worker_stopped(independent: bool) {
        let directory = tempfile::tempdir().unwrap();
        let marker = directory.path().join("worker.pid");
        let program = r#"import subprocess,sys,time
subprocess.Popen([sys.executable,'-c','import signal,time,os,pathlib,sys; signal.signal(signal.SIGTERM,signal.SIG_IGN); marker=pathlib.Path(sys.argv[1]); temporary=marker.with_name(marker.name+".tmp"); temporary.write_text(str(os.getpid())); temporary.replace(marker); time.sleep(30)',sys.argv[1]],start_new_session=sys.argv[2]=='independent')
time.sleep(30)
"#;
        let mut child = Command::new("python3")
            .args(["-c", program])
            .arg(&marker)
            .arg(if independent {
                "independent"
            } else {
                "inherited"
            })
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .unwrap();
        let mut owned = if independent {
            Descendants::new(&child)
        } else {
            Descendants::for_group(&child)
        }
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while !marker.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        let pid: u32 = std::fs::read_to_string(marker).unwrap().parse().unwrap();
        if independent {
            owned.refresh().unwrap();
        }
        // Same-group coverage deliberately does not discover the child until
        // after the leader was reaped: PPID ancestry alone cannot find it then.
        unsafe {
            libc::kill(child.id() as i32, libc::SIGTERM);
        }
        child.wait().unwrap();
        owned.stop(&mut child).unwrap();
        assert!(
            process_start(pid).unwrap().is_none(),
            "Owned worker survived its reaped leader"
        );
    }

    #[test]
    fn process_identity_mismatch_never_authorizes_a_signal() {
        let mut child = Command::new("/bin/sleep").arg("30").spawn().unwrap();
        let mut identity = Identity::capture(child.id()).unwrap().unwrap();
        identity.started = (0, 0);
        identity.signal(libc::SIGTERM).unwrap();
        let alive = child.try_wait().unwrap().is_none();
        let _ = child.kill();
        let _ = child.wait();
        assert!(alive, "An obsolete PID identity signalled a newer process");
        let observer = std::process::id();
        assert!(parse_processes(&format!("0 0 0\n{observer} 1 1\n")).is_ok());
        for invalid in [
            String::new(),
            "1 2".into(),
            "1 2 3 4".into(),
            "bad 2 3".into(),
            format!("{observer} 1 1\n{observer} 1 1\n"),
        ] {
            assert!(
                parse_processes(&invalid).is_err(),
                "Accepted incomplete process inventory: {invalid:?}"
            );
        }
    }
}
