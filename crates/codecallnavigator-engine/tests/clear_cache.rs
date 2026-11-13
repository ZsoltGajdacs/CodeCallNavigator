//! The executable owns cache deletion for both CLI and desktop clients.
use codecallnavigator_core::{
    artifact::AnalysisLock,
    boundary::DecisionCache,
    protocol::{Event, Request, VERSION},
};
use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};

fn populated_project() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join(".codecallnavigator")).unwrap();
    DecisionCache::open(project.path()).commit();
    assert!(
        project
            .path()
            .join(".codecallnavigator/inference.sqlite3")
            .is_file()
    );
    fs::write(
        project.path().join(".codecallnavigator/graph.json"),
        "last successful graph",
    )
    .unwrap();
    project
}

#[test]
fn cli_clears_only_inference_cache_and_is_idempotent() {
    let project = populated_project();
    for removed in [1, 0] {
        let output = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
            .args(["clear-cache", "--project"])
            .arg(project.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains(&format!("{removed} files removed"))
        );
        assert!(
            !project
                .path()
                .join(".codecallnavigator/inference.sqlite3")
                .exists()
        );
        assert_eq!(
            fs::read_to_string(project.path().join(".codecallnavigator/graph.json")).unwrap(),
            "last successful graph"
        );
    }
}

fn assert_cli_failure(project: &std::path::Path, expected: &str) {
    let output = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
        .args(["clear-cache", "--project"])
        .arg(project)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains(expected), "{error}");
}

#[test]
fn cli_rejects_invalid_projects_and_held_lock_without_modifying_cache_or_graph() {
    let project = populated_project();
    let directory = project.path().join(".codecallnavigator");
    let cache = fs::read(directory.join("inference.sqlite3")).unwrap();
    let graph = fs::read(directory.join("graph.json")).unwrap();
    assert_cli_failure(&project.path().join("missing"), "Cannot open cache project");
    assert_cli_failure(
        &directory.join("graph.json"),
        "Cache project must be a directory",
    );
    let _lock = AnalysisLock::acquire(project.path()).unwrap();
    assert_cli_failure(project.path(), "Another analysis");
    assert_eq!(
        fs::read(directory.join("inference.sqlite3")).unwrap(),
        cache
    );
    assert_eq!(fs::read(directory.join("graph.json")).unwrap(), graph);
}

#[cfg(unix)]
#[test]
fn cli_rejects_directory_and_symlink_cache_paths_before_deletion() {
    use std::os::unix::fs::symlink;
    let project = populated_project();
    let directory = project.path().join(".codecallnavigator");
    let cache = fs::read(directory.join("inference.sqlite3")).unwrap();
    let graph = fs::read(directory.join("graph.json")).unwrap();
    let outside = project.path().join("preserved.txt");
    fs::write(&outside, "external contents").unwrap();
    let sidecar = directory.join("inference.sqlite3-journal");
    fs::create_dir(&sidecar).unwrap();
    assert_cli_failure(project.path(), "not a regular file");
    assert!(sidecar.is_dir());
    fs::remove_dir(&sidecar).unwrap();
    symlink(&outside, &sidecar).unwrap();
    assert_cli_failure(project.path(), "not a regular file");
    assert!(
        fs::symlink_metadata(&sidecar)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        fs::read(directory.join("inference.sqlite3")).unwrap(),
        cache
    );
    assert_eq!(fs::read(directory.join("graph.json")).unwrap(), graph);
    assert_eq!(fs::read_to_string(&outside).unwrap(), "external contents");

    fs::remove_file(&sidecar).unwrap();
    let relocated = project.path().join("preserved-codecallnavigator");
    fs::rename(&directory, &relocated).unwrap();
    symlink(&relocated, &directory).unwrap();
    assert_cli_failure(project.path(), "Cache directory must be a real directory");
    assert_eq!(
        fs::read(relocated.join("inference.sqlite3")).unwrap(),
        cache
    );
    assert_eq!(fs::read(relocated.join("graph.json")).unwrap(), graph);
}

#[test]
fn protocol_reports_completion_or_lock_failure() {
    let project = populated_project();
    for locked in [true, false] {
        let lock = locked.then(|| AnalysisLock::acquire(project.path()).unwrap());
        let mut child = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
            .arg("serve-once")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let request = Request::ClearCache {
            version: VERSION,
            project: project.path().to_owned(),
        };
        writeln!(
            child.stdin.as_mut().unwrap(),
            "{}",
            serde_json::to_string(&request).unwrap()
        )
        .unwrap();
        let output = child.wait_with_output().unwrap();
        let events: Vec<Event> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert!(matches!(
            events.first(),
            Some(Event::Hello { version: VERSION })
        ));
        assert_eq!(output.status.success(), !locked);
        if locked {
            assert!(
                matches!(events.last(), Some(Event::Failed { message, .. }) if message.contains("Another analysis"))
            );
            assert!(
                project
                    .path()
                    .join(".codecallnavigator/inference.sqlite3")
                    .exists()
            );
        } else {
            assert!(matches!(
                events.last(),
                Some(Event::CacheCleared { removed_files: 1 })
            ));
            assert!(
                !project
                    .path()
                    .join(".codecallnavigator/inference.sqlite3")
                    .exists()
            );
        }
        drop(lock);
    }
}
