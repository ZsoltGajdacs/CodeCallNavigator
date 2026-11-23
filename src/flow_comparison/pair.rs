//! Build independent presentation sessions from each endpoint's cached source and chart.
use super::{Selection, call_changes::CallChanges};
use crate::{
    engine_client::flow_comparison as wire,
    flow_workspace::{Ready, Session},
    source,
};
use codecallnavigator::flow;
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SideId {
    Before,
    After,
}
#[derive(Clone)]
pub struct ReadySide {
    pub session: Session,
    pub node: Arc<wire::Symbol>,
    pub targets: Arc<Vec<wire::Symbol>>,
}
#[derive(Clone)]
pub enum Side {
    Ready(ReadySide),
    Missing,
    Unavailable(Arc<wire::UnavailableSide>),
}
#[derive(Clone)]
pub struct Pair {
    pub selection: Selection,
    pub correspondence: Arc<wire::Correspondence>,
    pub before: Side,
    pub after: Side,
    pub active_source: SideId,
    pub(super) fit_pending: [bool; 2],
    pub(super) call_changes: CallChanges,
}
pub(super) struct Prepared {
    correspondence: wire::Correspondence,
    before: PreparedSide,
    after: PreparedSide,
    call_changes: CallChanges,
}
enum PreparedSide {
    Ready {
        node: wire::Symbol,
        targets: Vec<wire::Symbol>,
        ready: Arc<Ready>,
    },
    Missing,
    Unavailable(wire::UnavailableSide),
}
impl Prepared {
    pub fn load(detail: wire::Detail, dot: &Path, cancel: &AtomicBool) -> Result<Self, String> {
        let before = prepare(detail.before, dot, cancel)?;
        let after = prepare(detail.after, dot, cancel)?;
        let call_changes = match (&detail.correspondence.kind, &before, &after) {
            (
                wire::CorrespondenceKind::Exact,
                PreparedSide::Ready { ready: before, .. },
                PreparedSide::Ready { ready: after, .. },
            ) => CallChanges::between(before, after),
            _ => CallChanges::default(),
        };
        Ok(Self {
            correspondence: detail.correspondence,
            before,
            after,
            call_changes,
        })
    }
    pub fn install(self, selection: Selection, namespace: &str, viewports: [[f32; 2]; 2]) -> Pair {
        Pair {
            selection,
            correspondence: Arc::new(self.correspondence),
            active_source: SideId::Before,
            fit_pending: [true, true],
            call_changes: self.call_changes,
            before: install(self.before, &format!("{namespace}:before"), viewports[0]),
            after: install(self.after, &format!("{namespace}:after"), viewports[1]),
        }
    }
}
fn prepare(
    side: wire::DetailSide,
    dot: &Path,
    cancel: &AtomicBool,
) -> Result<PreparedSide, String> {
    if cancel.load(Ordering::Acquire) {
        return Err("Comparison cancelled".into());
    }
    Ok(match side {
        wire::DetailSide::Missing => PreparedSide::Missing,
        wire::DetailSide::Unavailable(side) => PreparedSide::Unavailable(*side),
        wire::DetailSide::Ready(side) => {
            let wire::Ready {
                node,
                targets,
                chart,
                source: cached,
                ..
            } = *side;
            let chart = flow::layout(chart, dot).map_err(|error| format!("{error:#}"))?;
            let ready = Arc::new(Ready {
                chart,
                source: source::from_snapshot(cached.content, Path::new(&cached.file)),
            });
            PreparedSide::Ready {
                node,
                targets,
                ready,
            }
        }
    })
}
fn install(side: PreparedSide, namespace: &str, viewport: [f32; 2]) -> Side {
    match side {
        PreparedSide::Missing => Side::Missing,
        PreparedSide::Unavailable(side) => Side::Unavailable(Arc::new(side)),
        PreparedSide::Ready {
            node,
            targets,
            ready,
        } => {
            let mut session = Session::namespaced(node.symbol.clone(), namespace);
            session.install(ready, viewport);
            Side::Ready(ReadySide {
                session,
                node: Arc::new(node),
                targets: Arc::new(targets),
            })
        }
    }
}
impl Pair {
    pub fn side(&self, side: SideId) -> &Side {
        match side {
            SideId::Before => &self.before,
            SideId::After => &self.after,
        }
    }
    pub fn side_mut(&mut self, side: SideId) -> &mut Side {
        match side {
            SideId::Before => &mut self.before,
            SideId::After => &mut self.after,
        }
    }
}

impl SideId {
    pub fn index(self) -> usize {
        match self {
            Self::Before => 0,
            Self::After => 1,
        }
    }
}
