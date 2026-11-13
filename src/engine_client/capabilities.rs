//! Detect an incompatible configured CLI before sending desktop query requests.
use super::query_process::{active, stop_child};
use crate::runtime::Runtime;
use anyhow::{Context, Result, ensure};
use std::{
    io::Read,
    os::unix::process::CommandExt,
    process::{Child, Command, Stdio},
    sync::{atomic::AtomicBool, mpsc},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub(super) struct Capabilities {
    pub flow_languages: Vec<String>,
    pub observation: bool,
    pub automatic_refresh: bool,
}

pub(super) fn check(
    runtime: &Runtime,
    cancel: &AtomicBool,
    started: Instant,
) -> Result<Capabilities> {
    let mut child = Command::new(&runtime.engine)
        .arg("capabilities")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .context("Cannot inspect CLI capabilities")?;
    let result = (|| {
        let bytes = read_output(&mut child, cancel, started)?;
        let languages = validate(&bytes)?;
        require_exit(&mut child, cancel, started)?;
        Ok(languages)
    })();
    stop_child(&mut child);
    result
}

fn read_output(child: &mut Child, cancel: &AtomicBool, started: Instant) -> Result<Vec<u8>> {
    let output = child
        .stdout
        .take()
        .context("CLI capability output is unavailable")?;
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = output
            .take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = sender.send(result);
    });
    loop {
        active(cancel, started)?;
        match receiver.recv_timeout(Duration::from_millis(20)) {
            Ok(result) => return Ok(result?),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(error) => return Err(error.into()),
        }
    }
}

fn validate(bytes: &[u8]) -> Result<Capabilities> {
    ensure!(
        bytes.len() <= 1024 * 1024,
        "CLI capability response exceeds 1 MiB"
    );
    let capabilities: serde_json::Value = serde_json::from_slice(bytes)
        .context("Configured CLI does not expose compatible graph queries")?;
    ensure!(
        capabilities["query_version"] == codecallnavigator::query::VERSION,
        "Unsupported CLI query version"
    );
    let operations = capabilities["operations"]
        .as_array()
        .context("CLI omitted query capabilities")?;
    for operation in [
        "snapshot",
        "render_selection",
        "inspect",
        "architecture",
        "live_source",
        "explorer",
        "flow",
        "search",
        "status",
    ] {
        ensure!(
            operations.iter().any(|name| name == operation),
            "Configured CLI lacks required desktop operation {operation}"
        );
    }
    let flow_languages = serde_json::from_value(capabilities["flow_languages"].clone())
        .context("CLI omitted supported flow languages")?;
    let observation = supports_observation(&capabilities, operations);
    let automatic_refresh = observation && supports_automatic(&capabilities["automatic_refresh"]);
    Ok(Capabilities {
        flow_languages,
        observation,
        automatic_refresh,
    })
}

fn supports_observation(
    capabilities: &serde_json::Value,
    operations: &[serde_json::Value],
) -> bool {
    let freshness = &capabilities["freshness"];
    operations.iter().any(|name| name == "freshness")
        && freshness["version"] == codecallnavigator::freshness::VERSION
        && freshness["streaming"] == true
        && freshness["mode"] == "verified_polling"
        && freshness["limited"] == true
        && freshness["reconcile_milliseconds"]
            == codecallnavigator::freshness::RECONCILE_MILLISECONDS
        && freshness["heartbeat_milliseconds"]
            == codecallnavigator::freshness::HEARTBEAT_MILLISECONDS
}

fn supports_automatic(automatic: &serde_json::Value) -> bool {
    automatic["version"] == 1
        && automatic["activity_version"] == 1
        && automatic["observe_flag"] == "--auto-refresh"
        && automatic["debounce_milliseconds"] == 300
        && automatic["cancel_command"] == "cancel_refresh"
}

fn require_exit(child: &mut Child, cancel: &AtomicBool, started: Instant) -> Result<()> {
    let completed = Instant::now();
    loop {
        active(cancel, started)?;
        if let Some(status) = child.try_wait()? {
            ensure!(status.success(), "CLI capability command failed ({status})");
            return Ok(());
        }
        ensure!(
            completed.elapsed() < Duration::from_secs(5),
            "CLI capability command did not exit promptly"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn automatic_refresh_requires_its_complete_optional_contract() {
        let mut value = codecallnavigator::query::capabilities();
        value["automatic_refresh"] = serde_json::json!({
            "version":1,"activity_version":1,"observe_flag":"--auto-refresh",
            "debounce_milliseconds":300,"cancel_command":"cancel_refresh"
        });
        assert!(
            validate(&serde_json::to_vec(&value).unwrap())
                .unwrap()
                .automatic_refresh
        );
        value["automatic_refresh"]["activity_version"] = 2.into();
        assert!(
            !validate(&serde_json::to_vec(&value).unwrap())
                .unwrap()
                .automatic_refresh
        );
        value.as_object_mut().unwrap().remove("automatic_refresh");
        assert!(
            !validate(&serde_json::to_vec(&value).unwrap())
                .unwrap()
                .automatic_refresh
        );
    }

    #[test]
    fn optional_observation_support_does_not_disable_legacy_graph_queries() {
        let mut capabilities = codecallnavigator::query::capabilities();
        assert!(
            validate(&serde_json::to_vec(&capabilities).unwrap())
                .unwrap()
                .observation
        );
        capabilities.as_object_mut().unwrap().remove("freshness");
        let legacy = validate(&serde_json::to_vec(&capabilities).unwrap()).unwrap();
        assert!(!legacy.observation);
        assert!(
            legacy
                .flow_languages
                .iter()
                .any(|language| language == "typescript")
        );
    }
}
