//! Shared endpoint loading, exact binding, reference policy and saved-input freshness.
use super::super::query::{self, freshness};
use codecallnavigator_core::{
    artifact::{Artifact, difference, revision, verification},
    query::{Error, Request, Result},
};
use serde::Serialize;
use serde_json::Value;

pub(crate) struct Pair {
    pub before: revision::Snapshot,
    pub after: revision::Snapshot,
    pub comparison_id: String,
}

pub(crate) struct InspectedPair {
    pub before: revision::Inspection,
    pub after: revision::Inspection,
    pub comparison_id: String,
}

/// Request references and their optional follow-up binding belong to one comparison.
pub(crate) struct EndpointRequest<'a> {
    pub base: &'a str,
    pub head: &'a str,
    pub base_snapshot: Option<&'a str>,
    pub head_snapshot: Option<&'a str>,
    pub expected: Option<&'a str>,
}
enum ArtifactPolicy {
    RequireValid,
    RetainUnavailable,
}
impl ArtifactPolicy {
    fn check(&self, inspection: &revision::Inspection) -> Result<()> {
        match (self, &inspection.artifact) {
            (Self::RequireValid, revision::ArtifactResult::Unavailable(error)) => {
                Err(Error::new(error.code, error.message.clone()))
            }
            _ => Ok(()),
        }
    }
}

impl EndpointRequest<'_> {
    /// Keep strict base-before-head failure order for existing Compare callers.
    pub(crate) fn load(&self, request: &Request) -> Result<Pair> {
        let pair = self.read(request, ArtifactPolicy::RequireValid)?;
        Ok(Pair {
            before: pair.before.into_snapshot().map_err(revision_error)?,
            after: pair.after.into_snapshot().map_err(revision_error)?,
            comparison_id: pair.comparison_id,
        })
    }

    pub(crate) fn inspect(&self, request: &Request) -> Result<InspectedPair> {
        self.read(request, ArtifactPolicy::RetainUnavailable)
    }

    fn read(&self, request: &Request, policy: ArtifactPolicy) -> Result<InspectedPair> {
        let project = project(request)?;
        let before = revision::inspect_cached(&project, self.base, self.base_snapshot)
            .map_err(revision_error)?;
        policy.check(&before)?;
        let after = revision::inspect_cached(&project, self.head, self.head_snapshot)
            .map_err(revision_error)?;
        policy.check(&after)?;
        query::check_generation(request, &after.endpoint.digest)?;
        let comparison_id = identity(&before.endpoint, &after.endpoint)?;
        self.check_binding(&comparison_id)?;
        Ok(InspectedPair {
            before,
            after,
            comparison_id,
        })
    }

    pub(crate) fn freshness(
        &self,
        request: &Request,
        artifact: &Artifact,
        digest: &str,
    ) -> Result<Option<Value>> {
        if self.head != "WORKTREE" {
            return Ok(None);
        }
        let roots = query::roots(request)?;
        Ok(Some(freshness::summary(
            verification::verify_artifact(artifact, digest, &roots)
                .map_err(|error| Error::new("verification_failed", format!("{error:#}")))?,
        )?))
    }

    pub(crate) fn validate_base(&self) -> Result<()> {
        if self.base == "WORKTREE" {
            return Err(Error::new(
                "invalid_request",
                "Comparison base must resolve to a commit",
            ));
        }
        Ok(())
    }
    pub(crate) fn validate_references(&self) -> Result<()> {
        for reference in [self.base, self.head] {
            if reference.trim().is_empty() || reference.len() > 16384 {
                return Err(Error::new(
                    "invalid_request",
                    "Comparison references must contain 1..16384 bytes",
                ));
            }
        }
        if [self.base_snapshot, self.head_snapshot]
            .into_iter()
            .flatten()
            .any(|snapshot| !valid_digest(snapshot))
        {
            return Err(Error::new(
                "invalid_request",
                "Snapshot IDs must be 64-character hexadecimal digests",
            ));
        }
        Ok(())
    }
    pub(crate) fn validate_expected(&self) -> Result<()> {
        if self.expected.is_some_and(|id| !valid_digest(id)) {
            return Err(Error::new(
                "invalid_request",
                "expected_comparison must be a 64-character hexadecimal digest",
            ));
        }
        Ok(())
    }
    pub(crate) fn check_binding(&self, comparison_id: &str) -> Result<()> {
        if self
            .expected
            .is_some_and(|expected| expected != comparison_id)
        {
            return Err(Error::new(
                "comparison_mismatch",
                "Comparison endpoints changed; restart pagination",
            ));
        }
        Ok(())
    }
}

pub(crate) fn project(request: &Request) -> Result<std::path::PathBuf> {
    request
        .project
        .canonicalize()
        .map_err(|error| Error::new("project_unavailable", error.to_string()))
}

/// Preserve the legacy version, tuple order and endpoint field serialization exactly.
/// Inspected endpoints use the same known-schema serialization as validated endpoints.
pub(crate) fn identity(before: &impl Serialize, after: &impl Serialize) -> Result<String> {
    let binding =
        serde_json::to_vec(&(difference::VERSION, before, after)).map_err(super::encoding_error)?;
    Ok(blake3::hash(&binding).to_hex().to_string())
}
fn valid_digest(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|byte| byte.is_ascii_hexdigit())
}
pub(crate) fn revision_error(error: anyhow::Error) -> Error {
    Error::new(revision::error_code(&error), format!("{error:#}"))
}
