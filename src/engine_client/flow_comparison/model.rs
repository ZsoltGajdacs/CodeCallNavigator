//! Owned public-wire DTOs; compiler target certainty remains in the function/chart calls.
use codecallnavigator::artifact::local_snapshots::SnapshotRecord;
use codecallnavigator::flow::{FlowChart, FunctionFlowIR};
use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointKind {
    Commit,
    Worktree,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Endpoint {
    pub kind: EndpointKind,
    pub requested_ref: String,
    pub commit: Option<String>,
    pub path: String,
    pub schema: Option<String>,
    pub digest: String,
    pub snapshot: Option<Box<SnapshotRecord>>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Header {
    pub version: u32,
    pub base: Endpoint,
    pub head: Endpoint,
    pub comparison_id: String,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Failure {
    pub reason: String,
    pub message: String,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum EndpointStatus {
    Available,
    Unavailable(Failure),
}
#[derive(Clone, Debug, Deserialize)]
pub struct EndpointStatuses {
    pub before: EndpointStatus,
    pub after: EndpointStatus,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Owner {
    pub build: String,
    pub project: String,
    pub file: String,
    pub language: String,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Symbol {
    pub symbol: String,
    pub label: String,
    pub signature: String,
    pub line: u32,
    pub body_unavailable: bool,
    pub owner: Owner,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Recorded {
    Recorded,
    Unavailable(Failure),
}
#[derive(Clone, Debug, Deserialize)]
pub struct Present {
    pub node: Option<Symbol>,
    pub flow: Recorded,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CatalogSide {
    Present(Box<Present>),
    Missing,
    Unknown(Failure),
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Membership {
    Both,
    Added,
    Removed,
    Unknown,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Record {
    pub symbol: String,
    pub membership: Membership,
    pub before: CatalogSide,
    pub after: CatalogSide,
}
#[derive(Debug, Deserialize)]
pub struct Catalog {
    #[serde(flatten)]
    pub header: Header,
    pub kind: String,
    pub complete: bool,
    pub endpoints: EndpointStatuses,
    pub text: String,
    pub records: Vec<Record>,
    pub offset: usize,
    pub limit: usize,
    pub returned: usize,
    pub total: usize,
    pub more: bool,
    pub next_offset: Option<usize>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Presence {
    Present,
    Unknown,
}
#[derive(Debug, Deserialize)]
pub struct Source {
    pub origin: String,
    pub file: String,
    pub symbol: String,
    pub line: u32,
    pub content: String,
}
#[derive(Debug, Deserialize)]
pub struct Ready {
    pub membership: Presence,
    pub node: Symbol,
    pub function: FunctionFlowIR,
    pub chart: FlowChart,
    pub source: Source,
    pub targets: Vec<Symbol>,
}
#[derive(Debug, Deserialize)]
pub struct UnavailableSide {
    pub membership: Presence,
    pub node: Option<Symbol>,
    #[serde(flatten)]
    pub failure: Failure,
}
#[derive(Debug, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DetailSide {
    Ready(Box<Ready>),
    Missing,
    Unavailable(Box<UnavailableSide>),
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrespondenceKind {
    Exact,
    Manual,
}
#[derive(Debug, Deserialize)]
pub struct Correspondence {
    pub kind: CorrespondenceKind,
    pub symbol: String,
    pub head_symbol: String,
}
#[derive(Debug, Deserialize)]
pub struct Detail {
    #[serde(flatten)]
    pub header: Header,
    pub kind: String,
    pub correspondence: Correspondence,
    pub before: DetailSide,
    pub after: DetailSide,
}

use anyhow::ensure;
impl Header {
    pub(super) fn validate(&self, base: &str, head: &str) -> anyhow::Result<()> {
        let header = self;
        ensure!(
            header.version == 1,
            "Unsupported CLI flow comparison version"
        );
        ensure!(
            header.base.requested_ref == base && header.head.requested_ref == head,
            "CLI returned different comparison references"
        );
        ensure!(
            digest(&header.comparison_id),
            "Invalid CLI comparison binding"
        );
        ensure!(
            header.base.kind == EndpointKind::Commit,
            "Comparison base is not a committed endpoint"
        );
        let head_kind = if head == "WORKTREE" {
            EndpointKind::Worktree
        } else {
            EndpointKind::Commit
        };
        ensure!(
            header.head.kind == head_kind,
            "Comparison head kind differs from requested reference"
        );
        endpoint(&header.base)?;
        endpoint(&header.head)
    }
}
fn digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
fn endpoint(endpoint: &Endpoint) -> anyhow::Result<()> {
    ensure!(digest(&endpoint.digest), "Invalid CLI artifact digest");
    ensure!(!endpoint.path.is_empty(), "CLI omitted artifact path");
    match endpoint.kind {
        EndpointKind::Commit => ensure!(
            endpoint
                .commit
                .as_ref()
                .is_some_and(|commit| !commit.is_empty()
                    && commit.bytes().all(|byte| byte.is_ascii_hexdigit())),
            "CLI omitted resolved commit"
        ),
        EndpointKind::Worktree => ensure!(
            endpoint.commit.is_none(),
            "WORKTREE endpoint has a resolved commit"
        ),
    }
    if let Some(snapshot) = &endpoint.snapshot {
        ensure!(
            endpoint.commit.as_deref() == Some(&snapshot.identity.commit)
                && endpoint.digest == snapshot.digest
                && digest(&snapshot.id),
            "CLI returned inconsistent cached snapshot provenance"
        );
    }
    Ok(())
}
