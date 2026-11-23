use super::{Preview, PreviewTarget, Workspace};
use crate::{
    engine_client::flow_comparison::Error,
    flow_comparison::{
        SideId,
        requests::{Clock, StringError},
        work::Tracked,
    },
};
use codecallnavigator::{
    artifact::comparison_changes::{ClassSource, ClassSourceTarget},
    query::ComparisonSide,
};
use rune::{Latest, Waker};

#[derive(Clone, Eq, PartialEq)]
struct Key {
    root: String,
    node: String,
    side: SideId,
}

type Output = (Key, Result<ClassSource, Error>);

#[derive(Default)]
pub(super) struct ClassPreview {
    clock: Clock,
    job: Latest<Tracked<Output>>,
    pending: Option<Key>,
    result: Option<(Key, Result<ClassSource, String>)>,
}

impl ClassPreview {
    pub(super) fn set_waker(&self, waker: Waker) {
        self.job.set_waker(waker);
    }

    pub(super) fn pending(&self) -> bool {
        self.job.is_pending()
    }

    pub(super) fn cancel_request(&mut self) {
        self.clock.cancel();
        self.job.cancel();
        self.pending = None;
    }

    pub(super) fn clear(&mut self) {
        self.cancel_request();
        self.result = None;
    }
}

impl Workspace {
    pub fn show_class_preview(&mut self, side: SideId, id: String, anchor: String) {
        if self.worktree_refresh.pending() {
            return;
        }
        let Some(root) = self.class_selection() else {
            return;
        };
        let Some(bound) = self.binding.clone() else {
            return;
        };
        let key = Key {
            root: root.to_owned(),
            node: id.clone(),
            side,
        };
        self.previews.active = Some(Preview {
            anchor,
            target: PreviewTarget::Class(side, id),
        });
        let class = &mut self.previews.class;
        if class.pending.as_ref() == Some(&key)
            || class
                .result
                .as_ref()
                .is_some_and(|(cached, _)| *cached == key)
        {
            return;
        }
        let cancel = class.clock.restart();
        class.pending = Some(key.clone());
        let work = self.workers.track((bound, key, cancel));
        class.job.request(work, |work| {
            work.map(|(bound, key, cancel)| {
                let side = match key.side {
                    SideId::Before => ComparisonSide::Before,
                    SideId::After => ComparisonSide::After,
                };
                let result = bound.class_source(
                    ClassSourceTarget {
                        root: &key.root,
                        node: &key.node,
                        side,
                    },
                    &cancel,
                );
                (key, result)
            })
        });
    }

    pub fn class_preview(&self, side: SideId, node: &str) -> Option<Result<&ClassSource, &str>> {
        let root = self.class_selection()?;
        let (key, result) = self.previews.class.result.as_ref()?;
        (key.root == root && key.node == node && key.side == side)
            .then(|| result.as_ref().map_err(String::as_str))
    }

    pub(super) fn publish_class_preview(&mut self) -> bool {
        let Some(job) = self.previews.class.job.take() else {
            return false;
        };
        job.map(|(key, result)| {
            if self.previews.class.pending.as_ref() != Some(&key) {
                return;
            }
            self.previews.class.pending = None;
            if self.class_selection() != Some(key.root.as_str()) {
                return;
            }
            let result = match result {
                Err(error) if super::mismatch(&error) => {
                    self.failed(StringError::Query(error));
                    return;
                }
                other => other.map_err(|error| error.to_string()),
            };
            self.previews.class.result = Some((key, result));
        });
        true
    }
}
