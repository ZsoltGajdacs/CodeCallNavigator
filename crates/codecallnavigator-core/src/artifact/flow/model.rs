use crate::flow::{FlowChart, FunctionFlowIR};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Owner<'a> {
    pub build: &'a str,
    pub project: &'a str,
    pub file: &'a str,
    pub language: &'a str,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Symbol<'a> {
    pub symbol: &'a str,
    pub label: &'a str,
    pub signature: &'a str,
    pub line: u32,
    pub body_unavailable: bool,
    pub owner: Owner<'a>,
}
#[derive(Debug)]
pub struct Ready<'a> {
    pub node: Symbol<'a>,
    pub function: FunctionFlowIR,
    pub chart: FlowChart,
    pub content: &'a str,
    pub targets: Vec<Symbol<'a>>,
}
/// Metadata and owned source are recorded; chart/source-range limits are checked by select.
#[derive(Debug)]
pub enum Recording<'a> {
    Recorded(Symbol<'a>),
    Missing,
    Unavailable(Unavailable),
}
#[derive(Debug)]
pub enum Selection<'a> {
    Missing,
    Unavailable(Unavailable),
    Ready(Box<Ready<'a>>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    InvalidArtifact,
    DuplicateNode,
    UnsupportedLanguage,
    BodyUnavailable,
    MissingInventory,
    InventoryVersion,
    MissingFunction,
    DuplicateFunction,
    FunctionOwnership,
    MissingSource,
    DuplicateSource,
    InvalidSource,
    InvalidFlow,
    InvalidTarget,
}
#[derive(Clone, Debug, Serialize)]
pub struct Unavailable {
    pub reason: Reason,
    pub message: String,
}

impl Unavailable {
    pub(super) fn new(reason: Reason, message: impl Into<String>) -> Self {
        Self {
            reason,
            message: message.into(),
        }
    }
}
impl std::fmt::Display for Unavailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}
impl std::error::Error for Unavailable {}
