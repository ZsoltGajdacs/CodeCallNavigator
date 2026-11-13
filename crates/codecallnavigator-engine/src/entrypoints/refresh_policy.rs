//! Pure revision-bound debounce and failure suppression; no compiler or I/O policy.
use codecallnavigator_core::refresh::DEBOUNCE_MILLISECONDS;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Key {
    pub generation: String,
    pub selection: String,
    pub revision: String,
}
impl Key {
    pub fn supersedes(&self, active: &Self) -> bool {
        // A child may have atomically published before its terminal event is
        // consumed. A generation change alone must not cancel that handoff.
        self.selection != active.selection || self.revision != active.revision
    }
}
#[derive(Clone)]
pub(super) struct Pending {
    pub key: Key,
    pub changed: Instant,
    pub observed_revision: Option<String>,
}
impl Pending {
    // Exclusive time after configured debounce, including scan/scheduling and
    // lock contention. Child analysis time is reported separately.
    pub fn queue_milliseconds(&self, now: Instant) -> f64 {
        now.duration_since(self.changed)
            .saturating_sub(Duration::from_millis(DEBOUNCE_MILLISECONDS))
            .as_secs_f64()
            * 1000.0
    }
}
#[derive(Default)]
pub(super) struct Policy {
    current: Option<Key>,
    handled: Option<Key>,
    pub pending: Option<Pending>,
}

impl Policy {
    pub fn observe(&mut self, key: Key, refresh: bool, now: Instant) -> bool {
        if self.current.as_ref() == Some(&key) {
            if !refresh {
                self.pending = None;
            }
            return false;
        }
        self.current = Some(key.clone());
        self.pending = if refresh && self.handled.as_ref() != Some(&key) {
            Some(Pending {
                observed_revision: None,
                key,
                changed: now,
            })
        } else {
            None
        };
        true
    }
    pub fn due(&self, now: Instant) -> Option<Pending> {
        self.pending
            .as_ref()
            .filter(|pending| {
                now.duration_since(pending.changed) >= Duration::from_millis(DEBOUNCE_MILLISECONDS)
            })
            .cloned()
    }
    pub fn take(&mut self) -> Option<Pending> {
        self.pending.take()
    }
    pub fn suppress(&mut self, key: Key) {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.key == key)
        {
            self.pending = None;
        }
        self.handled = Some(key);
    }
    pub fn cancel(&mut self) -> Option<Key> {
        let key = self.current.clone();
        if let Some(key) = &key {
            self.handled = Some(key.clone());
        }
        self.pending = None;
        key
    }
    pub fn retry(&mut self, pending: Pending) {
        if self.pending.is_none() && self.current.as_ref() == Some(&pending.key) {
            self.pending = Some(pending);
        }
    }
    pub fn unavailable(&mut self) {
        self.pending = None;
        self.current = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key(generation: &str, revision: &str) -> Key {
        Key {
            generation: generation.into(),
            selection: "selection".into(),
            revision: revision.into(),
        }
    }
    #[test]
    fn observing_a_publication_supersedes_only_changed_compiler_inputs() {
        let active = key("old", "saved");
        let published = key("new", "saved");
        assert_ne!(
            active, published,
            "Generation still binds retry suppression"
        );
        assert!(
            !published.supersedes(&active),
            "Own publication must finish its terminal handoff"
        );
        assert!(key("new", "newer-save").supersedes(&active));
        let mut changed_selection = published;
        changed_selection.selection = "different-targets".into();
        assert!(changed_selection.supersedes(&active));
    }
    #[test]
    fn bursts_coalesce_cancel_suppresses_pending_and_new_generation_releases_failure() {
        let now = Instant::now();
        let mut policy = Policy::default();
        policy.observe(key("a", "first"), true, now);
        policy.observe(key("a", "latest"), true, now + Duration::from_millis(100));
        assert!(policy.due(now + Duration::from_millis(300)).is_none());
        assert_eq!(
            policy
                .due(now + Duration::from_millis(401))
                .unwrap()
                .key
                .revision,
            "latest"
        );
        policy.cancel();
        policy.observe(key("a", "latest"), true, now + Duration::from_secs(1));
        assert!(policy.pending.is_none());
        policy.observe(
            key("new-selection-generation", "latest"),
            true,
            now + Duration::from_secs(2),
        );
        assert!(policy.pending.is_some());
        policy.suppress(key("new-selection-generation", "latest"));
        policy.observe(
            key("new-selection-generation", "latest"),
            true,
            now + Duration::from_secs(3),
        );
        assert!(policy.pending.is_none());
    }
}
