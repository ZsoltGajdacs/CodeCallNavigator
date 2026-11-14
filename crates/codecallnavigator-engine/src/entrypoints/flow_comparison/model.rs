//! Borrowed wire values retain endpoint provenance until the complete envelope is bounded.
use codecallnavigator_core::{
    artifact::{
        flow::{Reason, Symbol, Unavailable},
        revision::InspectedEndpoint,
    },
    flow::{FlowChart, FunctionFlowIR},
};
use serde::Serialize;
use std::borrow::Cow;

#[derive(Clone, Serialize)]
#[serde(untagged)]
pub(super) enum FailureReason {
    Fact(Reason),
    Artifact(&'static str),
}
#[derive(Clone, Serialize)]
pub(super) struct Failure<'a> {
    pub reason: FailureReason,
    pub message: Cow<'a, str>,
}
impl From<Unavailable> for Failure<'_> {
    fn from(error: Unavailable) -> Self {
        Self {
            reason: FailureReason::Fact(error.reason),
            message: Cow::Owned(error.message),
        }
    }
}
#[derive(Serialize)]
pub(super) struct Header<'a> {
    pub version: u32,
    pub base: &'a InspectedEndpoint,
    pub head: &'a InspectedEndpoint,
    pub comparison_id: &'a str,
}
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Presence {
    Present,
    Unknown,
}
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(super) enum EndpointStatus<'a> {
    Available,
    Unavailable(Failure<'a>),
}
#[derive(Serialize)]
pub(super) struct EndpointStatuses<'a> {
    pub before: EndpointStatus<'a>,
    pub after: EndpointStatus<'a>,
}
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(super) enum Recorded<'a> {
    Recorded,
    Unavailable(Failure<'a>),
}
#[derive(Serialize)]
pub(super) struct Present<'a> {
    pub node: Option<Symbol<'a>>,
    pub flow: Recorded<'a>,
}
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(super) enum CatalogSide<'a> {
    Present(Box<Present<'a>>),
    Missing,
    Unknown(Failure<'a>),
}
impl CatalogSide<'_> {
    pub fn membership(&self) -> Option<bool> {
        match self {
            Self::Present(_) => Some(true),
            Self::Missing => Some(false),
            Self::Unknown(_) => None,
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Membership {
    Both,
    Added,
    Removed,
    Unknown,
}
#[derive(Serialize)]
pub(super) struct Record<'a> {
    pub symbol: &'a str,
    pub membership: Membership,
    pub before: CatalogSide<'a>,
    pub after: CatalogSide<'a>,
}
#[derive(Serialize)]
pub(super) struct Catalog<'a> {
    #[serde(flatten)]
    pub header: Header<'a>,
    pub kind: &'static str,
    pub complete: bool,
    pub endpoints: EndpointStatuses<'a>,
    pub text: &'a str,
    pub records: Vec<Record<'a>>,
    pub offset: usize,
    pub limit: usize,
    pub returned: usize,
    pub total: usize,
    pub more: bool,
    pub next_offset: Option<usize>,
}
#[derive(Serialize)]
pub(super) struct Source<'a> {
    pub origin: &'static str,
    pub file: &'a str,
    pub symbol: &'a str,
    pub line: u32,
    pub content: &'a str,
}
#[derive(Serialize)]
pub(super) struct Ready<'a> {
    pub membership: Presence,
    pub node: Symbol<'a>,
    pub function: FunctionFlowIR,
    pub chart: FlowChart,
    pub source: Source<'a>,
    pub targets: Vec<Symbol<'a>>,
}
#[derive(Serialize)]
pub(super) struct UnavailableSide<'a> {
    pub membership: Presence,
    pub node: Option<Symbol<'a>>,
    #[serde(flatten)]
    pub failure: Failure<'a>,
}
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(super) enum DetailSide<'a> {
    Ready(Box<Ready<'a>>),
    Missing,
    Unavailable(Box<UnavailableSide<'a>>),
}
#[derive(Serialize)]
pub(super) struct Correspondence<'a> {
    pub kind: &'static str,
    pub symbol: &'a str,
    pub head_symbol: &'a str,
}
#[derive(Serialize)]
pub(super) struct Detail<'a> {
    #[serde(flatten)]
    pub header: Header<'a>,
    pub kind: &'static str,
    pub correspondence: Correspondence<'a>,
    pub before: DetailSide<'a>,
    pub after: DetailSide<'a>,
}
