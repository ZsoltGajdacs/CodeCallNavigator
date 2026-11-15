//! A fixed sequence of owned-fixture operations; acknowledgments cannot satisfy another step.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Operation {
    MoveAfterRef,
    ArmGitGate,
    WaitGitGate,
    ReleaseGitGate,
    Capture,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Checkpoint {
    AfterRefMoved,
    GateArmed,
    GateHeld,
    GateReleased,
    MissingSource,
    ManualPair,
    RestoredPair,
}
#[derive(Clone, Debug, Serialize)]
pub(crate) struct Request {
    pub version: u32,
    pub sequence: u32,
    pub operation: Operation,
    pub checkpoint: Checkpoint,
    pub project: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparison_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Acknowledgment {
    version: u32,
    sequence: u32,
    operation: Operation,
    checkpoint: Checkpoint,
    success: bool,
    error: Option<String>,
}
impl Request {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version == 2 && (1..=16).contains(&self.sequence),
            "Invalid comparison request version or sequence"
        );
        ensure!(
            self.project.is_absolute(),
            "External operation requires an absolute owned project"
        );
        let checkpoint = match self.operation {
            Operation::MoveAfterRef => self.checkpoint == Checkpoint::AfterRefMoved,
            Operation::ArmGitGate => self.checkpoint == Checkpoint::GateArmed,
            Operation::WaitGitGate => self.checkpoint == Checkpoint::GateHeld,
            Operation::ReleaseGitGate => self.checkpoint == Checkpoint::GateReleased,
            Operation::Capture => matches!(
                self.checkpoint,
                Checkpoint::MissingSource | Checkpoint::ManualPair | Checkpoint::RestoredPair
            ),
        };
        ensure!(
            checkpoint,
            "External operation does not match its fixed checkpoint"
        );
        if self.operation == Operation::Capture {
            ensure!(
                self.window_id.is_some_and(|id| id > 0),
                "Capture requires its owned native window"
            );
            ensure!(
                self.comparison_id
                    .as_ref()
                    .is_some_and(|id| id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit())),
                "Capture requires its comparison binding"
            );
        } else {
            ensure!(
                self.window_id.is_none() && self.comparison_id.is_none(),
                "Only capture may carry window/binding metadata"
            );
        }
        ensure!(
            serde_json::to_vec(self)?.len() <= 8192,
            "Comparison request exceeds 8 KiB"
        );
        Ok(())
    }
    pub fn filename(&self, suffix: &str) -> String {
        format!("external-{:02}-{suffix}.json", self.sequence)
    }
    pub fn acknowledge(&self, bytes: &[u8]) -> Result<()> {
        ensure!(
            bytes.len() <= 8192,
            "Comparison acknowledgment exceeds 8 KiB"
        );
        let ack: Acknowledgment =
            serde_json::from_slice(bytes).context("Invalid comparison acknowledgment")?;
        ensure!(
            ack.version == 2 && ack.sequence == self.sequence,
            "Stale comparison acknowledgment version/sequence"
        );
        ensure!(
            ack.operation == self.operation && ack.checkpoint == self.checkpoint,
            "Comparison acknowledgment belongs to another operation/checkpoint"
        );
        ensure!(
            ack.success,
            "Comparison operation failed: {}",
            ack.error
                .as_deref()
                .unwrap_or("unspecified external failure")
        );
        ensure!(
            ack.error.is_none(),
            "Successful comparison acknowledgment carries an error"
        );
        Ok(())
    }
}
pub(crate) fn wait(
    directory: &Path,
    request: &Request,
    timeout: Duration,
    stopped: &AtomicBool,
) -> Result<()> {
    let deadline = Instant::now() + timeout;
    loop {
        ensure!(
            !stopped.load(Ordering::Acquire),
            "Comparison acceptance stopped before acknowledgment"
        );
        let path = directory.join(request.filename("ack"));
        match File::open(path) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take(8193).read_to_end(&mut bytes)?;
                return request.acknowledge(&bytes);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.into()),
        }
        ensure!(
            Instant::now() < deadline,
            "Comparison acknowledgment timed out"
        );
        thread::sleep(Duration::from_millis(25));
    }
}
#[cfg(test)]
pub(crate) fn sample() -> Request {
    Request {
        version: 2,
        sequence: 1,
        operation: Operation::ArmGitGate,
        checkpoint: Checkpoint::GateArmed,
        project: "/tmp/owned-project".into(),
        window_id: None,
        comparison_id: None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn acknowledgments_reject_stale_sequence_operation_checkpoint_unknown_fields_and_failure() {
        let request = sample();
        request.validate().unwrap();
        let valid = serde_json::json!({"version":2,"sequence":1,"operation":"arm_git_gate","checkpoint":"gate-armed","success":true});
        request
            .acknowledge(&serde_json::to_vec(&valid).unwrap())
            .unwrap();
        for (field, value) in [
            ("sequence", 2.into()),
            ("version", 1.into()),
            ("operation", "release_git_gate".into()),
            ("checkpoint", "gate-held".into()),
            ("success", false.into()),
            ("unknown", true.into()),
            ("error", "unexpected".into()),
        ] {
            let mut invalid = valid.clone();
            invalid[field] = value;
            assert!(
                request
                    .acknowledge(&serde_json::to_vec(&invalid).unwrap())
                    .is_err(),
                "Accepted {field}"
            );
        }
        assert!(request.acknowledge(&vec![b' '; 8193]).is_err());
        let mut mismatch = request;
        mismatch.checkpoint = Checkpoint::RestoredPair;
        assert!(mismatch.validate().is_err());
    }
    #[test]
    fn external_wait_never_turns_missing_timeout_or_cancel_into_success() {
        let dir = tempfile::tempdir().unwrap();
        let request = sample();
        let stop = AtomicBool::new(false);
        assert!(wait(dir.path(), &request, Duration::ZERO, &stop).is_err());
        stop.store(true, Ordering::Release);
        assert!(wait(dir.path(), &request, Duration::from_secs(1), &stop).is_err());
    }
}
