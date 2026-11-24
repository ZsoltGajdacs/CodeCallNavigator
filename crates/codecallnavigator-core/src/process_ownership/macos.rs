use super::{Identity, Observation, Process, SamplingUnavailable};
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeSet,
    io,
    mem::MaybeUninit,
    time::{Duration, Instant},
};

const MAX_PROCESSES: usize = 16_384;
const MAX_TABLE_ATTEMPTS: usize = 4;

#[repr(C)]
struct ProcessRow {
    seconds: u64,
    microseconds: u64,
    pid: i32,
    parent: i32,
    group: i32,
}

unsafe extern "C" {
    fn ccn_process_inventory(output: *mut ProcessRow, capacity: usize, count: *mut usize) -> i32;
    fn ccn_process_observation(pid: i32, output: *mut ProcessRow, count: *mut usize) -> i32;
}

impl TryFrom<ProcessRow> for Observation {
    type Error = anyhow::Error;

    fn try_from(row: ProcessRow) -> Result<Self> {
        ensure!(
            row.seconds != 0 && row.microseconds < 1_000_000,
            "Invalid start time in native process row for PID {}",
            row.pid
        );
        let pid = u32::try_from(row.pid).context("Invalid native process PID")?;
        ensure!(pid != 0, "Invalid zero process PID");
        Ok(Self {
            identity: Identity {
                pid,
                started: (row.seconds, row.microseconds),
            },
            parent: u32::try_from(row.parent).context("Invalid native parent PID")?,
            group: u32::try_from(row.group).context("Invalid native process group")?,
        })
    }
}

impl From<Observation> for Process {
    fn from(observation: Observation) -> Self {
        Self {
            pid: observation.identity.pid,
            parent: observation.parent,
            group: observation.group,
            started: Some(observation.identity.started),
        }
    }
}

pub(super) fn process_inventory(timeout: Duration) -> Result<Vec<Process>> {
    let deadline = Instant::now() + timeout;
    let mut rows = Vec::<ProcessRow>::with_capacity(MAX_PROCESSES);
    let mut remaining = MAX_TABLE_ATTEMPTS;
    loop {
        let status = read_table(&mut rows);
        if status == 0 {
            return normalize_table(rows);
        }
        remaining -= 1;
        if !matches!(status, libc::ENOMEM | libc::EINTR) {
            return Err(inventory_error(status));
        }
        if remaining == 0 || Instant::now() >= deadline {
            return Err(inventory_error(status));
        }
    }
}

fn read_table(rows: &mut Vec<ProcessRow>) -> i32 {
    let mut count = 0;
    // C writes at most the passed capacity and reports only initialized rows.
    let status = unsafe { ccn_process_inventory(rows.as_mut_ptr(), rows.capacity(), &mut count) };
    if status == 0 {
        if count > rows.capacity() {
            return libc::EPROTO;
        }
        unsafe { rows.set_len(count) };
    }
    status
}

fn normalize_table(rows: Vec<ProcessRow>) -> Result<Vec<Process>> {
    let mut seen = BTreeSet::new();
    let mut processes = Vec::with_capacity(rows.len());
    for row in rows.into_iter().filter(|row| row.pid != 0) {
        let process = Process::from(Observation::try_from(row)?);
        ensure!(
            seen.insert(process.pid),
            "Duplicate PID in native process inventory"
        );
        processes.push(process);
    }
    ensure!(
        seen.contains(&std::process::id()),
        "Native process inventory omitted its observer"
    );
    Ok(processes)
}

fn inventory_error(status: i32) -> anyhow::Error {
    let error = io::Error::from_raw_os_error(status);
    if recoverable(status) {
        return SamplingUnavailable::Inventory(error).into();
    }
    anyhow::Error::new(error).context("Cannot read a complete native process inventory")
}

pub(super) fn process_observation(pid: u32) -> Result<Option<Observation>> {
    let native_pid = i32::try_from(pid).context("Invalid PID for native identity lookup")?;
    ensure!(native_pid > 0, "Cannot look up a zero process PID");
    let mut row = MaybeUninit::uninit();
    let mut count = 0;
    let status = unsafe { ccn_process_observation(native_pid, row.as_mut_ptr(), &mut count) };
    match (status, count) {
        (0, 1) => {
            // The successful lookup reported one initialized row.
            let observation = Observation::try_from(unsafe { row.assume_init() })?;
            ensure!(
                observation.identity.pid == pid,
                "Mismatched native identity for owned process {pid}"
            );
            Ok(Some(observation))
        }
        (0 | libc::ESRCH, 0) => verify_absence(native_pid),
        (status, 0) if recoverable(status) => Err(SamplingUnavailable::Identity {
            pid,
            detail: io::Error::from_raw_os_error(status).to_string(),
        }
        .into()),
        _ => anyhow::bail!(
            "Invalid native identity lookup for owned process {pid}: status {status}, rows {count}"
        ),
    }
}

fn recoverable(status: i32) -> bool {
    matches!(
        status,
        libc::EPERM | libc::EACCES | libc::EAGAIN | libc::EINTR | libc::ENOMEM
    )
}

fn verify_absence(pid: i32) -> Result<Option<Observation>> {
    let status = unsafe { libc::kill(pid, 0) };
    if status == -1 && io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
        return Ok(None);
    }
    Err(SamplingUnavailable::Identity {
        pid: pid as u32,
        detail: "process lookup returned no row but absence could not be confirmed".into(),
    }
    .into())
}
