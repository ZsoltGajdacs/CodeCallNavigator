//! Read-only verification of observed inputs; this does not certify a build environment.
mod csharp_policy;
mod live;
#[cfg(test)]
mod observation_tests;
use super::{Artifact, GRAPH_PATH, Roots, paths::Paths, provenance};
use crate::fingerprint::observe_content;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path, sync::atomic::AtomicBool};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputStatus {
    Match,
    Stale,
    Unverifiable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reason {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    // Validate mappings even when the supplied artifact is malformed.
    Paths::new(roots)?;
    let digest = blake3::hash(bytes).to_hex().to_string();
    match Artifact::from_bytes(bytes) {
        Ok(artifact) => verify_artifact(&artifact, &digest, roots),
        Err(_) => {
            let mut report = report(&digest);
            report.contract_valid = false;
            report.input_status = InputStatus::Unverifiable;
            report.reason("invalid_contract", None);
            Ok(report)
        }
    }
}

fn report(digest: &str) -> Report {
    Report {
        version: 1,
        contract_valid: true,
        input_status: InputStatus::Match,
        digest: digest.into(),
        reasons: Vec::new(),
    }
}

/// Recheck observed inputs for an already validated immutable artifact.
/// `digest` must identify the exact bytes used to load that artifact. Query
/// sessions reuse parsing while checking current input contents on every request.
pub fn verify_artifact(artifact: &Artifact, digest: &str, roots: &Roots) -> Result<Report> {
    Ok(inspect(artifact, digest, roots, None)?.report)
}

pub struct Observation {
    pub report: Report,
    pub input_revision: Option<String>,
    pub complete: bool,
}

/// Observe recorded inputs and current descendants; never certify unrecorded inputs.
pub fn observe_artifact(
    artifact: &Artifact,
    digest: &str,
    roots: &Roots,
    cancel: &AtomicBool,
) -> Result<Observation> {
    inspect(artifact, digest, roots, Some(cancel))
}

struct Mappings<'a> {
    roots: &'a Roots,
    paths: Paths,
}

fn inspect(
    artifact: &Artifact,
    digest: &str,
    roots: &Roots,
    cancel: Option<&AtomicBool>,
) -> Result<Observation> {
    let analysis = provenance::read(&artifact.graph)?;
    Inspection {
        artifact,
        digest,
        roots,
    }
    .manifest(analysis.as_ref(), cancel, &[])
}

/// Scheduling-only observation using explicit compiler-output ownership. This
/// never replaces the ordinary report or full live input revision.
pub fn observe_refresh_trigger(
    artifact: &Artifact,
    digest: &str,
    roots: &Roots,
    cancel: &AtomicBool,
) -> Result<Observation> {
    let analysis = provenance::read(&artifact.graph)?.context("Missing refresh provenance")?;
    let filtered = super::refresh_inputs::filtered(&analysis)?;
    let outputs = super::refresh_inputs::owned_outputs(&analysis, roots)?;
    Inspection {
        artifact,
        digest,
        roots,
    }
    .manifest(Some(&filtered), Some(cancel), &outputs)
}

struct Inspection<'a> {
    artifact: &'a Artifact,
    digest: &'a str,
    roots: &'a Roots,
}

impl Inspection<'_> {
    fn manifest(
        self,
        analysis: Option<&provenance::Analysis>,
        cancel: Option<&AtomicBool>,
        outputs: &[std::path::PathBuf],
    ) -> Result<Observation> {
        let mappings = Mappings {
            roots: self.roots,
            paths: Paths::new(self.roots)?,
        };
        let mut report = report(self.digest);
        let Some(analysis) = analysis else {
            report.input_status = InputStatus::Unverifiable;
            report.reason("missing_analysis", None);
            return Ok(Observation {
                report,
                input_revision: None,
                complete: false,
            });
        };
        if csharp_policy::legacy(&self.artifact.graph, analysis) {
            report.reason("legacy_csharp_input_policy", None);
        }
        let mut live =
            cancel.map(|cancel| live::Inputs::excluding(&mappings.paths, cancel, outputs));
        let stale = verify_manifest(analysis, &mappings, &mut report, live.as_mut())?;
        let (input_revision, complete) = match live {
            Some(live) => {
                let revision = live.finish()?;
                report.reasons.extend(revision.reasons);
                (
                    Some(revision.revision),
                    revision.complete && !analysis.inputs.is_empty(),
                )
            }
            None => (None, false),
        };
        report.input_status = if stale {
            InputStatus::Stale
        } else if !report.reasons.is_empty() {
            InputStatus::Unverifiable
        } else {
            InputStatus::Match
        };
        Ok(Observation {
            report,
            input_revision,
            complete,
        })
    }
}

fn verify_manifest(
    analysis: &provenance::Analysis,
    mappings: &Mappings<'_>,
    report: &mut Report,
    mut live: Option<&mut live::Inputs<'_>>,
) -> Result<bool> {
    for limitation in &analysis.limitations {
        report.reason(&limitation.code, None);
    }
    if analysis.inputs.is_empty() {
        report.reason("no_observed_inputs", None);
    }
    let available = available_roots(analysis, mappings.roots, report);
    if let Some(live) = &mut live {
        for root in &analysis.roots {
            if !available.contains(&format!("@{root}")) {
                live.unavailable_root(root);
            }
        }
    }
    let mut stale = false;
    for input in analysis.inputs.iter().filter(|input| {
        input
            .path
            .split_once('/')
            .is_some_and(|(root, _)| available.contains(root))
    }) {
        stale |= verify_input(input, &mappings.paths, report, live.as_deref_mut())?;
    }
    Ok(stale)
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

fn verify_input(
    input: &provenance::Input,
    paths: &Paths,
    report: &mut Report,
    live: Option<&mut live::Inputs<'_>>,
) -> Result<bool> {
    if let Some(live) = &live {
        live.check()?;
    }
    let local = match paths.local(&input.path) {
        Ok(path) => path,
        Err(_) => {
            report.reason("unavailable_root", Some(input.path.clone()));
            return Ok(false);
        }
    };
    let current = match &live {
        Some(live) => live.observe(Path::new(&local), &input.excluded_children),
        None => observe_content(Path::new(&local), &input.excluded_children),
    };
    if let Some(live) = live {
        live.record(input, Path::new(&local), current.as_ref())?;
    }
    match current {
        Ok(current) if current.kind == input.kind && current.hash == input.hash => Ok(false),
        Ok(_) => {
            report.reason("input_changed", Some(input.path.clone()));
            Ok(true)
        }
        Err(_) => {
            report.reason("unreadable_input", Some(input.path.clone()));
            Ok(false)
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
    fn legacy_csharp_is_unverifiable_without_rewriting_and_stale_takes_precedence() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("input.txt"), "one").unwrap();
        let roots = vec![("repo".into(), root.path().into())];
        let mut artifact = Artifact::from_bytes(&artifact_bytes(root.path())).unwrap();
        artifact.graph["analysis"]["targets"] = serde_json::json!(["@repo/Empty.CSPROJ"]);
        let bytes = artifact.bytes().unwrap();
        fs::create_dir(root.path().join(".codecallnavigator")).unwrap();
        let path = root.path().join(GRAPH_PATH);
        fs::write(&path, &bytes).unwrap();
        let report = verify(root.path(), &roots).unwrap();
        assert_eq!(report.input_status, InputStatus::Unverifiable);
        assert!(
            report
                .reasons
                .iter()
                .any(|reason| reason.code == "legacy_csharp_input_policy")
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
        fs::write(root.path().join("input.txt"), "two").unwrap();
        assert_eq!(
            verify(root.path(), &roots).unwrap().input_status,
            InputStatus::Stale
        );
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
