//! Endpoint-local membership and metadata; unavailable artifacts never imply absence.
use super::model::*;
use codecallnavigator_core::artifact::{
    flow::{self, View},
    revision::{ArtifactResult, Inspection},
};
use std::borrow::Cow;

pub(super) struct Views<'a> {
    pub before: Endpoint<'a>,
    pub after: Endpoint<'a>,
}
impl<'a> Views<'a> {
    pub fn new(before: &'a Inspection, after: &'a Inspection) -> Self {
        Self {
            before: Endpoint::new(before),
            after: Endpoint::new(after),
        }
    }
}

pub(super) enum Endpoint<'a> {
    Available(View<'a>),
    Unavailable(Failure<'a>),
}
impl<'a> Endpoint<'a> {
    pub fn new(inspection: &'a Inspection) -> Self {
        match &inspection.artifact {
            ArtifactResult::Available(artifact) => match View::new(artifact) {
                Ok(view) => Self::Available(view),
                Err(error) => Self::Unavailable(error.into()),
            },
            ArtifactResult::Unavailable(error) => Self::Unavailable(Failure {
                reason: FailureReason::Artifact(error.code),
                message: Cow::Borrowed(&error.message),
            }),
        }
    }
    pub fn status(&self) -> EndpointStatus<'a> {
        match self {
            Self::Available(_) => EndpointStatus::Available,
            Self::Unavailable(error) => EndpointStatus::Unavailable(error.clone()),
        }
    }
    pub fn symbols(&self) -> Box<dyn Iterator<Item = &'a str> + '_> {
        match self {
            Self::Available(view) => Box::new(view.symbols()),
            Self::Unavailable(_) => Box::new(std::iter::empty()),
        }
    }
    pub fn describe(&self, symbol: &str) -> Option<flow::Symbol<'a>> {
        match self {
            Self::Available(view) => view.describe(symbol).ok().flatten(),
            Self::Unavailable(_) => None,
        }
    }
    pub fn catalog(&self, symbol: &str) -> CatalogSide<'a> {
        let view = match self {
            Self::Available(view) => view,
            Self::Unavailable(error) => return CatalogSide::Unknown(error.clone()),
        };
        match view.recording(symbol) {
            flow::Recording::Missing => CatalogSide::Missing,
            flow::Recording::Recorded(node) => CatalogSide::Present(Box::new(Present {
                node: Some(node),
                flow: Recorded::Recorded,
            })),
            flow::Recording::Unavailable(error) => CatalogSide::Present(Box::new(Present {
                node: self.describe(symbol),
                flow: Recorded::Unavailable(error.into()),
            })),
        }
    }
    pub fn detail(&self, symbol: &str) -> DetailSide<'a> {
        match self {
            Self::Unavailable(error) => DetailSide::Unavailable(Box::new(UnavailableSide {
                membership: Presence::Unknown,
                node: None,
                failure: error.clone(),
            })),
            Self::Available(view) => match view.select(symbol) {
                flow::Selection::Missing => DetailSide::Missing,
                flow::Selection::Unavailable(error) => {
                    DetailSide::Unavailable(Box::new(UnavailableSide {
                        membership: Presence::Present,
                        node: self.describe(symbol),
                        failure: error.into(),
                    }))
                }
                flow::Selection::Ready(ready) => DetailSide::Ready(Box::new(Ready {
                    membership: Presence::Present,
                    node: ready.node,
                    source: Source {
                        origin: "analyzed",
                        file: ready.node.owner.file,
                        symbol: ready.node.symbol,
                        line: ready.node.line,
                        content: ready.content,
                    },
                    function: ready.function,
                    chart: ready.chart,
                    targets: ready.targets,
                })),
            },
        }
    }
}
