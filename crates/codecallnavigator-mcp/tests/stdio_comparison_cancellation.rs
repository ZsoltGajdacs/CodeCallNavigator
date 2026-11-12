//! Pending Git reads inherit the real CLI's cancellable process group.
mod support;
use serde_json::json;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use support::{Client, command, engine};

fn repository(root: &Path) -> Vec<u8> {
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.email", "test@example.invalid"],
        vec!["config", "user.name", "Comparison cancellation"],
    ] {
        assert!(
            Command::new("/usr/bin/git")
                .arg("-C")
                .arg(root)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    fs::create_dir(root.join(".codecallnavigator")).unwrap();
    let bytes = serde_json::to_vec(&json!({"schema":"codecallnavigator.graph.v1","graph":{"nodes":[],"edges":[],"roots":[],"diagnostics":[]}})).unwrap();
    fs::write(root.join(".codecallnavigator/graph.json"), &bytes).unwrap();
    for args in [
        vec!["add", ".codecallnavigator/graph.json"],
        vec!["commit", "-qm", "baseline"],
    ] {
        assert!(
            Command::new("/usr/bin/git")
                .arg("-C")
                .arg(root)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    bytes
}

struct PendingGit(Vec<String>);
impl PendingGit {
    fn observe(marker: &Path) -> Self {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Ok(text) = fs::read_to_string(marker) {
                let pids: Vec<_> = text.split_whitespace().map(str::to_owned).collect();
                if pids.len() == 2 {
                    return Self(pids);
                }
            }
            assert!(
                Instant::now() < deadline,
                "CLI did not reach pending Git blob read"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn assert_stopped(&self) {
        let deadline = Instant::now() + Duration::from_secs(8);
        while self.0.iter().any(|pid| alive(pid)) {
            assert!(
                Instant::now() < deadline,
                "Comparison cancellation left CLI/Git PIDs {:?}",
                self.0
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for PendingGit {
    fn drop(&mut self) {
        for pid in &self.0 {
            let _ = Command::new("/bin/kill")
                .args(["-TERM", pid])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
}
fn alive(pid: &str) -> bool {
    Command::new("/bin/kill")
        .args(["-0", pid])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success()
}

fn pending_comparison(disconnect: bool) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let before = repository(&root);
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    let wrapper = bin.join("git");
    fs::write(&wrapper, "#!/bin/sh\nfor argument in \"$@\"; do\n if [ \"$argument\" = cat-file ]; then\n  printf '%s %s' \"$$\" \"$PPID\" > \"$CCN_COMPARISON_MARKER\"\n  exec /bin/sleep 30\n fi\ndone\nexec /usr/bin/git \"$@\"\n").unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    let marker = root.join("pending.pid");
    let mut configured = command(&engine(), &root);
    configured.env(
        "PATH",
        format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
    );
    configured.env("CCN_COMPARISON_MARKER", &marker);
    let mut client = Client::launch(configured);
    let request = client.start(
        "tools/call",
        json!({"name":"compare_graphs","arguments":{"base":"HEAD"}}),
    );
    let pending = PendingGit::observe(&marker);
    if disconnect {
        client.disconnect();
        client.assert_shutdown();
    } else {
        client.notify(
            "notifications/cancelled",
            json!({"requestId":request,"reason":"cancel pending snapshot read"}),
        );
    }
    pending.assert_stopped();
    assert_eq!(
        before,
        fs::read(root.join(".codecallnavigator/graph.json")).unwrap()
    );
    assert!(!root.join(".git/FETCH_HEAD").exists());
    assert_eq!(
        fs::read_dir(root.join(".codecallnavigator"))
            .unwrap()
            .count(),
        1
    );
    if !disconnect {
        client.assert_no_response(request);
        client.disconnect();
        client.assert_shutdown();
    }
}

#[test]
#[ignore = "requires built real CLI via CODECALLNAVIGATOR_TEST_ENGINE and process inspection"]
fn cancelling_comparison_reaps_the_pending_git_read() {
    pending_comparison(false);
}

#[test]
#[ignore = "requires built real CLI via CODECALLNAVIGATOR_TEST_ENGINE and process inspection"]
fn disconnecting_comparison_reaps_the_pending_git_read() {
    pending_comparison(true);
}
