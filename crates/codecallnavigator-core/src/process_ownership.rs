//! Owned compiler-process cleanup shared by adapters and refresh supervision.
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(target_os = "macos")]
use macos as platform;
use platform::{process_inventory, process_observation};

use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeMap,
    process::Child,
    time::{Duration, Instant},
};

/// Native process information was indeterminate; this never authorizes a signal.
#[derive(Debug)]
enum SamplingUnavailable {
    Identity {
        pid: u32,
        detail: String,
    },
    #[cfg(target_os = "macos")]
    Inventory(std::io::Error),
}

impl std::fmt::Display for SamplingUnavailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Identity { pid, detail } => {
                write!(
                    formatter,
                    "Cannot read birth identity for owned process {pid}: {detail}"
                )
            }
            #[cfg(target_os = "macos")]
            Self::Inventory(error) => write!(formatter, "Cannot read process inventory: {error}"),
        }
    }
}
impl std::error::Error for SamplingUnavailable {}

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
        let Some(row) = processes.iter().find(|row| row.pid == self.pid) else {
            return Ok(false);
        };
        row.started
            .map_or_else(|| self.alive(), |started| Ok(started == self.started))
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
    // The Linux inventory does not carry start times. Read /proc when needed.
    started: Option<(u64, u64)>,
}

impl Process {
    fn observation(&self) -> Result<Option<Observation>> {
        if let Some(started) = self.started {
            return Ok(Some(Observation {
                identity: Identity {
                    pid: self.pid,
                    started,
                },
                parent: self.parent,
                group: self.group,
            }));
        }
        process_observation(self.pid)
    }
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
    fn invalidate_members(&mut self) {
        if matches!(self.observation, GroupObservation::Members(_)) {
            self.observation = GroupObservation::Unknown;
        }
    }

    fn observe<'a>(&mut self, processes: &'a [Process]) -> Result<Vec<&'a Process>> {
        if matches!(self.observation, GroupObservation::Retired) {
            return Ok(Vec::new());
        }
        let members = processes
            .iter()
            .filter(|row| row.group == self.pid)
            .collect::<Vec<_>>();
        if members.is_empty() || self.leader_replaced()? {
            self.observation = GroupObservation::Retired;
            return Ok(Vec::new());
        }
        ensure!(
            self.leader.is_some(),
            "Spawned process group {} has members but its original leader birth identity was unavailable",
            self.pid
        );
        self.observation = GroupObservation::Members(members.iter().map(|row| row.pid).collect());
        Ok(members)
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
            if self.current_member(*pid, identities)? && !self.leader_replaced()? {
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
    tracking_unavailable: bool,
}

impl Descendants {
    pub fn new(child: &Child) -> Result<Self> {
        let owner =
            Identity::capture(child.id())?.context("Cannot identify owned analysis process")?;
        Ok(Self {
            owned: [(owner.pid, owner)].into_iter().collect(),
            unidentified: Default::default(),
            group: None,
            tracking_unavailable: false,
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
            tracking_unavailable: false,
        })
    }

    /// Sample during live work. A temporary macOS denial is reported once per
    /// uninterrupted streak and can be retried without stopping the work.
    /// Call `stop` to require fresh, conclusive cleanup evidence.
    pub fn track(&mut self) -> Result<Option<anyhow::Error>> {
        let result = if cfg!(target_os = "macos") {
            self.refresh_bounded(Duration::from_secs(2))
        } else {
            self.refresh()
        };
        track_sample(&mut self.tracking_unavailable, result)
    }

    pub fn refresh(&mut self) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(2);
        let retry_until = Instant::now() + Duration::from_millis(100);
        loop {
            let result = self.refresh_bounded(deadline.saturating_duration_since(Instant::now()));
            let retryable = result
                .as_ref()
                .is_err_and(|error| error.is::<SamplingUnavailable>());
            if !retryable || Instant::now() >= retry_until {
                return result;
            }
            // Re-enumerate immediately: a coherent new table can prove a short-
            // lived candidate absent without guessing from a denied PID query.
        }
    }

    fn refresh_bounded(&mut self, timeout: Duration) -> Result<()> {
        self.invalidate_sampled_group_members();
        let result = self.observe_descendants(timeout);
        if result.is_err() {
            self.invalidate_sampled_group_members();
        }
        result
    }

    fn invalidate_sampled_group_members(&mut self) {
        if let Some(group) = &mut self.group {
            group.invalidate_members();
        }
    }

    fn observe_descendants(&mut self, timeout: Duration) -> Result<()> {
        let processes = process_inventory(timeout)?;
        let mut pending = self.refresh_known(&processes)?;
        if let Some(group) = &mut self.group {
            let members = group.observe(&processes)?;
            for row in members {
                self.capture_descendant(row, &Relationship::SpawnedGroup)?;
                pending.push(row.pid);
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

    fn capture_descendant(&mut self, row: &Process, relationship: &Relationship) -> Result<()> {
        let pid = row.pid;
        match self.capture_related(row, relationship) {
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

    fn capture_related(
        &self,
        row: &Process,
        relationship: &Relationship,
    ) -> Result<Option<Identity>> {
        let Some(current) = row.observation()? else {
            return Ok(None);
        };
        let pid = row.pid;
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
            .collect::<Vec<_>>();
        let Some(parent) = self.owned.get(&parent).cloned() else {
            return Ok(Vec::new());
        };
        let relationship = Relationship::Parent(parent);
        for row in &children {
            if !self.owned.contains_key(&row.pid) {
                self.capture_descendant(row, &relationship)?;
            }
        }
        Ok(children.iter().map(|row| row.pid).collect())
    }

    fn confirm_unidentified(&mut self, processes: &[Process]) -> Result<()> {
        for pid in self.unidentified.iter().copied().collect::<Vec<_>>() {
            if !processes.iter().any(|row| row.pid == pid) {
                self.unidentified.remove(&pid);
            }
        }
        if let Some(pid) = self.unidentified.iter().next() {
            return Err(SamplingUnavailable::Identity {
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

fn track_sample(unavailable: &mut bool, result: Result<()>) -> Result<Option<anyhow::Error>> {
    match result {
        Ok(()) => {
            *unavailable = false;
            Ok(None)
        }
        Err(error) if cfg!(target_os = "macos") && error.is::<SamplingUnavailable>() => {
            let previously_unavailable = std::mem::replace(unavailable, true);
            Ok((!previously_unavailable).then_some(error))
        }
        Err(error) => Err(error),
    }
}

fn retain_signal_failure(failure: &mut Option<anyhow::Error>, error: anyhow::Error) {
    // A real signal failure takes precedence over transient identity sampling.
    if failure
        .as_ref()
        .is_none_or(|previous| previous.is::<SamplingUnavailable>())
    {
        *failure = Some(error);
    }
}

fn resolved_after_cleanup(result: Result<()>) -> Result<()> {
    match result {
        Err(error) if error.is::<SamplingUnavailable>() => Ok(()),
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

/// Transfer a direct child after bounded cleanup reports uncertainty. No PID
/// lookup is used: this exact Child remains owned until the kernel can reap it.
pub fn reap_eventually(mut child: Child) {
    std::thread::spawn(move || {
        let _ = child.wait();
    });
}

fn process_start(pid: u32) -> Result<Option<(u64, u64)>> {
    Ok(process_observation(pid)?.map(|observed| observed.identity.started))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        os::unix::process::CommandExt,
        process::{Command, Stdio},
    };
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
    }

    #[test]
    fn native_inventory_observes_live_child_and_checks_its_exit() {
        let mut child = spawn_sleep();
        let pid = child.id();
        let observation = process_observation(pid);
        let row_observation = process_inventory(Duration::from_secs(2)).and_then(|table| {
            table
                .iter()
                .find(|row| row.pid == pid)
                .context("Live child missing from inventory")?
                .observation()
        });
        child.kill().unwrap();
        child.wait().unwrap();

        let observation = observation.unwrap().unwrap();
        let row = row_observation.unwrap().unwrap();
        assert_eq!(observation.parent, std::process::id());
        assert_eq!(row.parent, observation.parent);
        assert_eq!(row.group, observation.group);
        assert_eq!(row.identity.started, observation.identity.started);
        assert!(process_observation(pid).unwrap().is_none());
    }

    #[test]
    fn tracking_reports_unavailable_samples_by_platform() {
        fn unavailable() -> Result<()> {
            Err(SamplingUnavailable::Identity {
                pid: 10,
                detail: "Native process identity withheld".into(),
            }
            .into())
        }

        let mut tracking_unavailable = false;
        if cfg!(target_os = "linux") {
            assert!(track_sample(&mut tracking_unavailable, unavailable()).is_err());
            return;
        }
        assert!(
            track_sample(&mut tracking_unavailable, unavailable())
                .unwrap()
                .is_some()
        );
        assert!(
            track_sample(&mut tracking_unavailable, unavailable())
                .unwrap()
                .is_none()
        );
        assert!(
            track_sample(&mut tracking_unavailable, Ok(()))
                .unwrap()
                .is_none()
        );
        assert!(
            track_sample(&mut tracking_unavailable, unavailable())
                .unwrap()
                .is_some()
        );
        assert!(
            track_sample(
                &mut tracking_unavailable,
                Err(anyhow::anyhow!("Invalid inventory"))
            )
            .is_err()
        );
    }

    #[test]
    fn invalidating_group_members_preserves_confirmed_retirement() {
        let mut group = SpawnedGroup {
            pid: 10,
            leader: None,
            observation: GroupObservation::Members([10, 11].into_iter().collect()),
        };
        group.invalidate_members();
        assert!(matches!(group.observation, GroupObservation::Unknown));

        group.observation = GroupObservation::Retired;
        group.invalidate_members();
        assert!(matches!(group.observation, GroupObservation::Retired));
    }

    #[test]
    fn cleanup_rejects_a_live_unidentified_candidate_without_signaling_it() {
        let mut child = spawn_sleep();
        let mut candidate = spawn_sleep();
        let mut owned = Descendants::new(&child).unwrap();
        let pid = candidate.id();
        owned.unidentified.insert(pid);

        let cleanup = owned.stop(&mut child);
        let candidate_was_alive = candidate.try_wait().unwrap().is_none();
        candidate.kill().unwrap();
        candidate.wait().unwrap();
        child.wait().unwrap();

        let error = cleanup.unwrap_err();
        assert!(error.is::<SamplingUnavailable>(), "{error:#}");
        assert!(
            candidate_was_alive,
            "Cleanup signaled an unidentified process"
        );
    }

    fn spawn_sleep() -> Child {
        Command::new("/bin/sleep").arg("30").spawn().unwrap()
    }
}
