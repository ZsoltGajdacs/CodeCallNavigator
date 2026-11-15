//! Published expansion and immutable ordinary backup are separate from pending request intent.
use super::super::{Ready, Session};
use crate::{
    background_work::{Tracked, Workers},
    source,
};
use codecallnavigator::{flow, query::FlowExpansionSelection};
use rune::Latest;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
#[derive(Clone)]
pub(crate) struct Mode {
    pub original: Box<Session>,
    pub published: Option<Arc<Published>>,
    pub selections: Vec<FlowExpansionSelection>,
    pub pending: Option<Vec<FlowExpansionSelection>>,
    pub scrolls: HashMap<String, [f32; 2]>,
    pub mounted: Option<Mount>,
    pub active: Option<flow::SourceId>,
}
#[derive(Clone)]
pub(crate) struct Mount {
    pub key: String,
    pub offset: [f32; 2],
    pub center: bool,
}
pub(crate) struct Published {
    pub badges: HashMap<usize, String>,
    pub result: flow::ExpandedFlow,
    pub sources: Vec<Option<source::SourceFile>>,
    pub keys: Vec<String>,
    pub root_source: flow::SourceId,
    pub query_time: Duration,
    pub layout_time: Duration,
}
pub(crate) struct Prepared {
    pub ready: Ready,
    pub published: Arc<Published>,
}
pub(crate) struct Output {
    pub serial: u64,
    pub epoch: u64,
    pub symbol: String,
    pub generation: String,
    pub selections: Vec<FlowExpansionSelection>,
    pub result: Result<Prepared, String>,
}
#[derive(Default)]
pub(crate) struct Work {
    pub job: Latest<Tracked<Output>>,
    pub workers: Arc<Workers>,
    pub cancel: Arc<AtomicBool>,
}
impl Work {
    pub fn cancel(&mut self) {
        self.cancel.store(true, Ordering::Release);
        self.job.cancel();
    }
    pub fn pending(&self) -> bool {
        self.job.is_pending() || !self.workers.stopped()
    }
    pub fn set_waker(&self, waker: rune::Waker) {
        self.job.set_waker(waker.clone());
        self.workers.set_waker(waker);
    }
}
impl Session {
    pub(crate) fn marker_badge(&self, step: usize) -> Option<&str> {
        self.expansion
            .as_ref()?
            .published
            .as_ref()?
            .badges
            .get(&step)
            .map(String::as_str)
    }
    pub(crate) fn expanded_flow(&self) -> Option<&flow::ExpandedFlow> {
        Some(&self.expansion.as_ref()?.published.as_ref()?.result)
    }
    pub(crate) fn expansion_timings(&self) -> Option<(Duration, Duration)> {
        let published = self.expansion.as_ref()?.published.as_ref()?;
        Some((published.query_time, published.layout_time))
    }
    pub(crate) fn expansion_mode(&self) -> bool {
        self.expansion.is_some()
    }
    pub(crate) fn displayed_source(&self) -> Option<&source::SourceFile> {
        let ready = self.ready.as_ref()?;
        let Some(published) = self
            .expansion
            .as_ref()
            .and_then(|mode| mode.published.as_ref())
        else {
            return Some(&ready.source);
        };
        let source = self.expansion_source()?;
        if source == published.root_source {
            Some(&ready.source)
        } else {
            published.sources.get(source.0)?.as_ref()
        }
    }
    pub(crate) fn displayed_file(&self) -> Option<&str> {
        match self.expanded_flow() {
            Some(result) => Some(&result.sources.get(self.expansion_source()?)?.owner.file),
            None => Some(&self.ready.as_ref()?.chart.file),
        }
    }
    pub(super) fn expansion_source(&self) -> Option<flow::SourceId> {
        let published = self.expansion.as_ref()?.published.as_ref()?;
        self.expansion
            .as_ref()?
            .active
            .or(Some(published.root_source))
    }
    pub(super) fn expansion_key(&self) -> Option<&str> {
        let published = self.expansion.as_ref()?.published.as_ref()?;
        published
            .keys
            .get(self.expansion_source()?.0)
            .map(String::as_str)
    }
}
