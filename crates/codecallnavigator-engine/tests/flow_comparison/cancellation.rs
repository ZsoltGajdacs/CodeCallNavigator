//! Cancellation kills the query-owned Git group during a stalled revision lookup.
use super::support::*;
use serde_json::json;
use std::{
    fs,
    io::Write,
    os::unix::{fs::PermissionsExt, process::CommandExt},
    process::{Child, Command, Stdio},
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
    // Creating the ready path before printf completes lets the test observe an
    // empty PID. Rename a complete marker so readiness also proves its payload.
    fs::write(&wrapper,format!("#!/bin/sh\ncase \"$*\" in *'rev-parse --verify'*) printf '%s' \"$$\" > '{}.tmp'; mv '{}.tmp' '{}'; sleep 30;; esac\nexec /usr/bin/git \"$@\"\n",marker.display(),marker.display(),marker.display())).unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    let saved = fs::read(working_manifest(root)).unwrap();
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
    let mut query = QueryChild(command.spawn().unwrap());
    let child = &mut query.0;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            json!({"version":2,"project":root,"query":SAVED.catalog()})
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
    let after = json!({"graph":fs::read(working_manifest(root)).unwrap(),"head":git(root,&["rev-parse","HEAD"]),"fetch":root.join(".git/FETCH_HEAD").exists(),"lock":root.join(".codecallnavigator/analysis.lock").exists()});
    assert_eq!(
        after,
        json!({"graph":saved,"head":head,"fetch":false,"lock":false})
    );
}

/// This probe deliberately stalls descendants; assertions must not abandon its
/// session leader or Git wrapper if readiness or cancellation checks fail.
struct QueryChild(Child);

impl Drop for QueryChild {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(None)) {
            // The unreaped child owns this setsid-created process group.
            unsafe {
                libc::kill(-(self.0.id() as i32), libc::SIGKILL);
            }
        }
        let _ = self.0.wait();
    }
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
