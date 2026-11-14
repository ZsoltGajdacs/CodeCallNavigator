//! Cancellation kills the query-owned Git group before a blocked blob read can publish anything.
use super::support::*;
use codecallnavigator_core::artifact;
use serde_json::json;
use std::{
    fs,
    io::Write,
    os::unix::{fs::PermissionsExt, process::CommandExt},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[test]
fn cancellation_during_git_read_preserves_artifacts_and_reaps_the_git_group() {
    let repo = Repository::new();
    let root = repo.root();
    let driver = tempfile::tempdir().unwrap();
    let marker = driver.path().join("git-pid");
    let wrapper = driver.path().join("git");
    fs::write(&wrapper,format!("#!/bin/sh\ncase \"$*\" in *'cat-file blob'*) printf '%s' \"$$\" > '{}'; sleep 30;; esac\nexec /usr/bin/git \"$@\"\n",marker.display())).unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    let saved = fs::read(root.join(artifact::GRAPH_PATH)).unwrap();
    let head = git(root, &["rev-parse", "HEAD"]);
    let mut command = engine();
    command
        .arg("query")
        .env("PATH", format!("{}:/usr/bin:/bin", driver.path().display()))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            json!({"version":1,"project":root,"query":SAVED.catalog()})
                .to_string()
                .as_bytes(),
        )
        .unwrap();
    wait_for(|| marker.is_file());
    let git_pid: i32 = fs::read_to_string(&marker).unwrap().parse().unwrap();
    assert_eq!(
        unsafe { libc::kill(-(child.id() as i32), libc::SIGTERM) },
        0
    );
    assert!(!child.wait().unwrap().success());
    wait_for(|| !running(git_pid));
    let after = json!({"graph":fs::read(root.join(artifact::GRAPH_PATH)).unwrap(),"head":git(root,&["rev-parse","HEAD"]),"fetch":root.join(".git/FETCH_HEAD").exists(),"lock":root.join(".codecallnavigator/analysis.lock").exists()});
    assert_eq!(
        after,
        json!({"graph":saved,"head":head,"fetch":false,"lock":false})
    );
}
fn wait_for(complete: impl Fn() -> bool) {
    let start = Instant::now();
    while !complete() {
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "Cancellation fixture timed out"
        );
        thread::sleep(Duration::from_millis(10));
    }
}
fn running(pid: i32) -> bool {
    let output = Command::new("/bin/ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .output()
        .unwrap();
    let state = String::from_utf8_lossy(&output.stdout);
    let state = state.trim();
    !state.is_empty() && !state.starts_with('Z')
}
