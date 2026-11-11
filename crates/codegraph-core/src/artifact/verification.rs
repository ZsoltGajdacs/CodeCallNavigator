//! Read-only verification of observed inputs; this does not certify a build environment.
use super::{Artifact, GRAPH_PATH, Roots, paths::Paths, provenance};
use crate::fingerprint::observe_content;
use anyhow::{Context, Result};
use serde::Serialize;
use std::{collections::BTreeSet, fs, path::Path};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InputStatus {
    Match,
    Stale,
    Unverifiable,
}

#[derive(Debug, Serialize)]
pub struct Reason {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub version: u32,
    pub contract_valid: bool,
    pub input_status: InputStatus,
    pub digest: String,
    pub reasons: Vec<Reason>,
}
impl Report {
    pub fn exit_code(&self) -> i32 {
        if !self.contract_valid {
            2
        } else if self.input_status == InputStatus::Match {
            0
        } else {
            1
        }
    }
    fn reason(&mut self, code: &str, path: Option<String>) {
        self.reasons.push(Reason {
            code: code.into(),
            path,
        });
    }
}

/// Hash actual file bytes (including legacy formatting), and never acquire a write lock.
pub fn verify(project: &Path, roots: &Roots) -> Result<Report> {
    let bytes = fs::read(project.join(GRAPH_PATH)).context("Cannot read graph artifact")?;
    verify_bytes(&bytes, roots)
}

pub fn verify_bytes(bytes: &[u8], roots: &Roots) -> Result<Report> {
    let paths = Paths::new(roots)?;
    let mut report = Report {
        version: 1,
        contract_valid: true,
        input_status: InputStatus::Match,
        digest: blake3::hash(bytes).to_hex().to_string(),
        reasons: Vec::new(),
    };
    let artifact = match Artifact::from_bytes(bytes) {
        Ok(artifact) => artifact,
        Err(_) => {
            report.contract_valid = false;
            report.input_status = InputStatus::Unverifiable;
            report.reason("invalid_contract", None);
            return Ok(report);
        }
    };
    let Some(analysis) = provenance::read(&artifact.graph)? else {
        report.input_status = InputStatus::Unverifiable;
        report.reason("missing_analysis", None);
        return Ok(report);
    };
    let stale = verify_manifest(&analysis, roots, &paths, &mut report);
    report.input_status = if stale {
        InputStatus::Stale
    } else if !report.reasons.is_empty() {
        InputStatus::Unverifiable
    } else {
        InputStatus::Match
    };
    Ok(report)
}

fn verify_manifest(
    analysis: &provenance::Analysis,
    roots: &Roots,
    paths: &Paths,
    report: &mut Report,
) -> bool {
    for limitation in &analysis.limitations {
        report.reason(&limitation.code, None);
    }
    if analysis.inputs.is_empty() {
        report.reason("no_observed_inputs", None);
    }
    let available = available_roots(analysis, roots, report);
    let mut stale = false;
    for input in analysis.inputs.iter().filter(|input| {
        input
            .path
            .split_once('/')
            .is_some_and(|(root, _)| available.contains(root))
    }) {
        stale |= verify_input(input, paths, report);
    }
    stale
}

fn available_roots(
    analysis: &provenance::Analysis,
    roots: &Roots,
    report: &mut Report,
) -> BTreeSet<String> {
    let mut available = BTreeSet::new();
    for name in &analysis.roots {
        if roots
            .iter()
            .any(|(mapped, path)| mapped == name && path.is_dir())
        {
            available.insert(format!("@{name}"));
        } else {
            report.reason("unavailable_root", Some(format!("@{name}/")));
        }
    }
    available
}

fn verify_input(input: &provenance::Input, paths: &Paths, report: &mut Report) -> bool {
    let local = match paths.local(&input.path) {
        Ok(path) => path,
        Err(_) => {
            report.reason("unavailable_root", Some(input.path.clone()));
            return false;
        }
    };
    match observe_content(Path::new(&local), &input.excluded_children) {
        Ok(current) if current.kind == input.kind && current.hash == input.hash => false,
        Ok(_) => {
            report.reason("input_changed", Some(input.path.clone()));
            true
        }
        Err(_) => {
            report.reason("unreadable_input", Some(input.path.clone()));
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{artifact, fingerprint, model::GraphData};
    fn artifact_bytes(root: &Path) -> Vec<u8> {
        let fingerprints = fingerprint::capture_excluding(
            [root.into(), root.join("input.txt"), root.join("absent.txt")],
            &[],
        )
        .unwrap();
        let roots = vec![("repo".into(), root.into())];
        let mut artifact = artifact::canonical(&GraphData::default(), &roots).unwrap();
        artifact.graph["analysis"] =
            serde_json::to_value(provenance::build(&fingerprints, &[], "{}", &roots).unwrap())
                .unwrap();
        artifact.bytes().unwrap()
    }
    #[test]
    fn content_changes_directory_additions_missing_transitions_and_relocation() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("input.txt"), "one").unwrap();
        let bytes = artifact_bytes(root.path());
        let roots = vec![("repo".into(), root.path().into())];
        assert_eq!(
            verify_bytes(&bytes, &roots).unwrap().input_status,
            InputStatus::Match
        );
        fs::write(root.path().join("input.txt"), "two").unwrap();
        assert_eq!(
            verify_bytes(&bytes, &roots).unwrap().input_status,
            InputStatus::Stale
        );
        fs::write(root.path().join("input.txt"), "one").unwrap();
        fs::write(root.path().join("added.txt"), "").unwrap();
        assert_eq!(
            verify_bytes(&bytes, &roots).unwrap().input_status,
            InputStatus::Stale
        );
        fs::remove_file(root.path().join("added.txt")).unwrap();
        fs::write(root.path().join("absent.txt"), "").unwrap();
        assert_eq!(
            verify_bytes(&bytes, &roots).unwrap().input_status,
            InputStatus::Stale
        );
        let relocated = tempfile::tempdir().unwrap();
        fs::write(relocated.path().join("input.txt"), "one").unwrap();
        assert_eq!(
            verify_bytes(&bytes, &[("repo".into(), relocated.path().into())])
                .unwrap()
                .input_status,
            InputStatus::Match
        );
    }
    #[test]
    fn nonexistent_mapped_root_is_unverifiable_and_available_stale_inputs_take_precedence() {
        let repo = tempfile::tempdir().unwrap();
        let shared = tempfile::tempdir().unwrap();
        fs::write(repo.path().join("input.txt"), "one").unwrap();
        fs::write(shared.path().join("input.txt"), "one").unwrap();
        let roots = [
            ("repo".into(), repo.path().into()),
            ("shared".into(), shared.path().into()),
        ];
        let fingerprints = fingerprint::capture([
            repo.path().join("input.txt"),
            shared.path().join("input.txt"),
        ])
        .unwrap();
        let mut artifact = artifact::canonical(&GraphData::default(), &roots).unwrap();
        artifact.graph["analysis"] =
            serde_json::to_value(provenance::build(&fingerprints, &[], "{}", &roots).unwrap())
                .unwrap();
        let bytes = artifact.bytes().unwrap();
        let missing = repo.path().join("unavailable");
        let mappings = [
            ("repo".into(), repo.path().into()),
            ("shared".into(), missing),
        ];
        let report = verify_bytes(&bytes, &mappings).unwrap();
        assert_eq!(report.input_status, InputStatus::Unverifiable);
        assert!(
            report
                .reasons
                .iter()
                .any(|reason| reason.code == "unavailable_root")
        );
        assert!(
            !report
                .reasons
                .iter()
                .any(|reason| reason.code == "input_changed")
        );
        fs::write(repo.path().join("input.txt"), "two").unwrap();
        let report = verify_bytes(&bytes, &mappings).unwrap();
        assert_eq!(report.input_status, InputStatus::Stale);
        assert!(
            report
                .reasons
                .iter()
                .any(|reason| reason.code == "unavailable_root")
        );
        assert!(
            report
                .reasons
                .iter()
                .any(|reason| reason.code == "input_changed")
        );
    }
    #[test]
    fn legacy_and_unmapped_roots_are_unverifiable_and_invalid_contract_is_two() {
        let roots = [("repo".into(), "/tmp".into())];
        let legacy = artifact::canonical(&GraphData::default(), &roots)
            .unwrap()
            .bytes()
            .unwrap();
        assert_eq!(verify_bytes(&legacy, &roots).unwrap().exit_code(), 1);
        assert_eq!(verify_bytes(b"{}", &roots).unwrap().exit_code(), 2);
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("input.txt"), "one").unwrap();
        let bytes = artifact_bytes(root.path());
        assert_eq!(
            verify_bytes(&bytes, &[("other".into(), root.path().into())])
                .unwrap()
                .input_status,
            InputStatus::Unverifiable
        );
    }
}
