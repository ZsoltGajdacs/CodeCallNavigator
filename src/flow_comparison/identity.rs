//! Worker-resolved physical project identity is distinct from the captured display descriptor.
use super::work::Workers;
use crate::engine_client::QueryContext;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Installation {
    Same,
    Different,
    Pending,
}
enum Resolution {
    Pending,
    Resolved(PathBuf),
    Unavailable,
}
pub(super) struct Identity {
    resolution: Resolution,
    pending: Option<PathBuf>,
    sender: mpsc::Sender<(u64, PathBuf)>,
    receiver: mpsc::Receiver<(u64, PathBuf)>,
}
impl Default for Identity {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            resolution: Resolution::Pending,
            pending: None,
            sender,
            receiver,
        }
    }
}
pub(super) struct Reporter {
    serial: u64,
    sender: mpsc::Sender<(u64, PathBuf)>,
    workers: Arc<Workers>,
}
impl Identity {
    pub fn captured(&self) -> Option<&Path> {
        match &self.resolution {
            Resolution::Resolved(path) => Some(path),
            _ => None,
        }
    }
    pub fn reset(&mut self) {
        self.resolution = Resolution::Pending;
        self.pending = None;
    }
    pub fn reporter(&self, serial: u64, workers: Arc<Workers>) -> Reporter {
        Reporter {
            serial,
            sender: self.sender.clone(),
            workers,
        }
    }
    pub fn installed(&mut self, path: &Path) -> Installation {
        match &self.resolution {
            Resolution::Resolved(captured) if captured == path => Installation::Same,
            Resolution::Resolved(_) | Resolution::Unavailable => Installation::Different,
            Resolution::Pending => {
                self.pending = Some(path.into());
                Installation::Pending
            }
        }
    }
    pub fn resolved(&mut self, path: PathBuf) -> Installation {
        let same_captured = match &self.resolution {
            Resolution::Resolved(captured) => captured == &path,
            Resolution::Pending => true,
            Resolution::Unavailable => false,
        };
        let matches = same_captured
            && self
                .pending
                .as_ref()
                .is_none_or(|installed| installed == &path);
        self.resolution = Resolution::Resolved(path);
        self.pending = None;
        if matches {
            Installation::Same
        } else {
            Installation::Different
        }
    }
    pub fn completed(&mut self, physical: Option<&Path>) -> Installation {
        if let Some(path) = physical {
            return self.resolved(path.into());
        }
        if self.captured().is_some() {
            return Installation::Same;
        }
        self.resolution = Resolution::Unavailable;
        if self.pending.take().is_some() {
            Installation::Different
        } else {
            Installation::Pending
        }
    }
    pub fn poll(&mut self, serial: u64) -> Option<Installation> {
        let mut changed = None;
        while let Ok((observed, path)) = self.receiver.try_recv() {
            if observed == serial {
                changed = Some(self.resolved(path));
            }
        }
        changed
    }
}
impl Reporter {
    pub fn resolve(self, mut context: QueryContext) -> anyhow::Result<QueryContext> {
        let project = context.descriptor.project.canonicalize()?;
        context.descriptor.project = project.clone();
        let _ = self.sender.send((self.serial, project));
        self.workers.wake();
        Ok(context)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_unresolved_completion_cannot_wait_for_identity_after_a_later_installation() {
        let mut identity = Identity::default();
        identity.completed(None);
        assert!(matches!(
            identity.installed(Path::new("/installed-project")),
            Installation::Different
        ));
    }
}
