//! Source restoration waits for its own tab to mount; each pair retains its active endpoint.
use super::{Side, SideId, Workspace};
use crate::flow_workspace::{Session, SourceKeys};
pub(super) enum ScrollRequest {
    Restore { key: String, offset: [f32; 2] },
    Selection { key: String },
}
impl Workspace {
    fn active_session(&self) -> Option<&Session> {
        let pair = self.current.as_ref()?;
        match pair.side(pair.active_source) {
            Side::Ready(ready) => Some(&ready.session),
            _ => None,
        }
    }
    pub(super) fn arm_source_restore(&mut self) {
        self.pending_scroll = self.active_session().map(|session| ScrollRequest::Restore {
            key: session.scroll_key.clone(),
            offset: session.scroll.get(),
        });
    }
    pub fn activate_source(&mut self, side: SideId) {
        if let Some(pair) = &mut self.current {
            if pair.active_source != side {
                self.mounted_source = None;
            }
            pair.active_source = side;
        }
        self.arm_source_restore();
    }
    pub fn source_mounted(&mut self, side: SideId, key: &str) {
        let Some(pair) = &self.current else {
            return;
        };
        if pair.active_source != side {
            return;
        }
        let Side::Ready(ready) = pair.side(side) else {
            return;
        };
        if ready.session.scroll_key == key {
            self.mounted_source = Some(key.into());
        }
    }
    pub fn restore_scroll(&mut self, tx: &mut rune::Tx) -> bool {
        let Some(session) = self.active_session() else {
            self.pending_scroll = None;
            return false;
        };
        if self.mounted_source.as_ref() != Some(&session.scroll_key) {
            return false;
        }
        let Some(request) = self.pending_scroll.take() else {
            return false;
        };
        self.active_session()
            .is_some_and(|session| request.apply(session, tx))
    }
    pub fn select_step(&mut self, side: SideId, index: usize, tx: &mut rune::Tx) {
        let Some(pair) = &mut self.current else {
            return;
        };
        let Side::Ready(ready) = pair.side_mut(side) else {
            return;
        };
        if !ready.session.select_index(index) {
            return;
        }
        let key = ready.session.scroll_key.clone();
        self.pending_scroll = Some(if ready.session.selected_source_span().is_some() {
            ScrollRequest::Selection { key }
        } else {
            ScrollRequest::Restore {
                key,
                offset: ready.session.scroll.get(),
            }
        });
        if pair.active_source != side {
            self.mounted_source = None;
        }
        pair.active_source = side;
        self.restore_scroll(tx);
    }
}
impl ScrollRequest {
    fn apply(self, session: &Session, tx: &mut rune::Tx) -> bool {
        match self {
            Self::Restore {
                key,
                offset: [x, y],
            } if key == session.scroll_key => tx.scroll_to_instant(key, x, y),
            Self::Selection { key } if key == session.scroll_key => {
                session.center_selected(&SourceKeys::Comparison(key), tx)
            }
            _ => return false,
        }
        true
    }
}
