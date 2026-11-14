use codecallnavigator_core::artifact::{self, Artifact};
#[path = "snapshot.rs"]
mod snapshot;
use serde_json::{Value, json};
use snapshot::Snapshot;
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

pub fn git(root: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}
pub fn symbol(saved: &Artifact, name: &str) -> String {
    saved.graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["signature"] == format!("App::{name}()"))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}
pub fn save(root: &Path, saved: &Artifact) {
    fs::create_dir_all(root.join(".codecallnavigator")).unwrap();
    fs::write(root.join(artifact::GRAPH_PATH), saved.bytes().unwrap()).unwrap();
}
pub struct Repository {
    pub directory: tempfile::TempDir,
    pub before: Artifact,
    pub after: Artifact,
}
impl Repository {
    pub fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        git(root, &["init", "-q"]);
        git(root, &["config", "user.name", "Flow comparison tests"]);
        git(root, &["config", "user.email", "test@example.invalid"]);
        let before = Snapshot {
            root,
            marker: "before",
            extras: &["removed"],
        }
        .artifact();
        save(root, &before);
        git(root, &["add", artifact::GRAPH_PATH]);
        git(root, &["commit", "-qm", "before"]);
        git(root, &["tag", "base"]);
        let after = Snapshot {
            root,
            marker: "after",
            extras: &["added"],
        }
        .artifact();
        save(root, &after);
        git(root, &["add", artifact::GRAPH_PATH]);
        git(root, &["commit", "-qm", "after"]);
        git(root, &["tag", "head"]);
        Self {
            directory,
            before,
            after,
        }
    }
    pub fn root(&self) -> &Path {
        self.directory.path()
    }
}
pub fn engine() -> Command {
    Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
}
pub fn query(root: &Path, operation: Value) -> Value {
    let mut command = engine();
    command.arg("query");
    exchange(command, root, operation)
}
pub fn exchange(mut command: Command, root: &Path, operation: Value) -> Value {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            json!({"version":1,"project":root,"query":operation})
                .to_string()
                .as_bytes(),
        )
        .unwrap();
    let output = child.wait_with_output().unwrap();
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&output.stderr)))
}
#[derive(Clone, Copy)]
pub struct Revisions {
    pub base: &'static str,
    pub head: &'static str,
}
pub const SAVED: Revisions = Revisions {
    base: "base",
    head: "head",
};
pub const WORKTREE: Revisions = Revisions {
    base: "base",
    head: "WORKTREE",
};
impl Revisions {
    pub fn catalog(&self) -> Value {
        json!({"kind":"flow_comparison","base":self.base,"head":self.head,"selection":{"kind":"catalog"}})
    }
    pub fn detail(&self, symbol: &str) -> Value {
        json!({"kind":"flow_comparison","base":self.base,"head":self.head,"selection":{"kind":"detail","symbol":symbol}})
    }
}
