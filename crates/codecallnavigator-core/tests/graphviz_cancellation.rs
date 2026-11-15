//! Graphviz lifecycle needs real process evidence. The executable below is only
//! a blocked-process fault probe; it never fabricates a successful layout result.
use codecallnavigator_core::graphviz;
use std::{
    os::unix::fs::PermissionsExt,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

fn blocked_probe(directory: &Path) -> std::path::PathBuf {
    let executable = directory.join("blocked-dot");
    // The temporary directory comes from tempfile, without shell metacharacters.
    std::fs::write(
        &executable,
        format!(
            "#!/bin/sh\necho $$ > '{}'\nexec /bin/sleep 30\n",
            directory.join("started.pid").display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    executable
}

#[test]
fn pre_cancelled_graphviz_never_starts_a_child() {
    let directory = tempfile::tempdir().unwrap();
    let executable = blocked_probe(directory.path());
    let result = graphviz::run_cancellable(&executable, "digraph{}", &AtomicBool::new(true));
    assert!(format!("{:#}", result.unwrap_err()).contains("cancelled"));
    assert!(!directory.path().join("started.pid").exists());
}

#[test]
fn cancelling_a_blocked_graphviz_reaps_the_owned_process() {
    let directory = tempfile::tempdir().unwrap();
    let executable = blocked_probe(directory.path());
    let marker = directory.path().join("started.pid");
    let cancel = Arc::new(AtomicBool::new(false));
    let signal = cancel.clone();
    let observed = marker.clone();
    let worker = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !observed.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        signal.store(true, Ordering::Release);
        observed.exists()
    });
    let started = Instant::now();
    let result = graphviz::run_cancellable(&executable, &"x".repeat(1024 * 1024), &cancel);
    assert!(
        worker.join().unwrap(),
        "Probe must really start before cancellation"
    );
    let pid = std::fs::read_to_string(marker).unwrap();
    assert!(
        !std::process::Command::new("/bin/kill")
            .args(["-0", pid.trim()])
            .status()
            .unwrap()
            .success(),
        "Cancelled Graphviz child survived"
    );
    assert!(
        started.elapsed() < Duration::from_secs(8),
        "Cancellation waited for the layout timeout"
    );
    assert!(format!("{:#}", result.unwrap_err()).contains("cancelled"));
}
