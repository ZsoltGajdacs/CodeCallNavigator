//! Pair navigation never consults the currently installed graph or its generation.
use super::{
    Selection, Side, SideId, Workspace,
    requests::{DetailSelection, DetailWork},
};
impl Workspace {
    pub fn select(&mut self, selection: Selection) -> Result<(), String> {
        self.select_detail(DetailSelection::Function(selection))
    }

    pub(crate) fn select_class(&mut self, id: String) -> Result<(), String> {
        self.select_detail(DetailSelection::Class {
            id,
            depth: self.controls.class_depth,
        })
    }

    pub(crate) fn class_depth(&self) -> u32 {
        self.controls.class_depth
    }

    pub(crate) fn maximum_class_depth(&self) -> u32 {
        self.controls.maximum_class_depth
    }

    pub(crate) fn set_class_depth(&mut self, depth: u32) {
        self.controls.class_depth_open = false;
        let Some(id) = self.class_selection().map(str::to_owned) else {
            return;
        };
        if depth == self.controls.class_depth || !(1..=self.maximum_class_depth()).contains(&depth)
        {
            return;
        }
        match self.select_detail(DetailSelection::Class { id, depth }) {
            Ok(()) => self.controls.class_depth = depth,
            Err(error) => self.changes.message = error,
        }
    }

    fn select_detail(&mut self, selection: DetailSelection) -> Result<(), String> {
        self.require_idle_worktree_refresh()?;
        let bound = self
            .binding
            .clone()
            .ok_or("Reload the comparison before selecting a change")?;
        let project = self
            .project
            .as_ref()
            .ok_or("No comparison project is open")?;
        let dot = project.runtime.dot.clone();
        if let Some(previous) = self.current.take() {
            self.history.push(previous);
            if self.history.len() > 100 {
                self.history.remove(0);
            }
        }
        self.previews.close();
        self.controls.class_depth_open = false;
        self.detail_message = match &selection {
            DetailSelection::Function(_) => "Preparing before and after flowcharts…",
            DetailSelection::Class { .. } => "Preparing before and after class flows…",
        }
        .into();
        self.class_pair = None;
        self.detail_selection = Some(selection.clone());
        self.pending_scroll = None;
        self.mounted_source = None;
        let cancel = self.detail_clock.restart();
        let serial = self.detail_clock.serial;
        let work = self.workers.track(DetailWork {
            bound,
            selection,
            dot,
            cancel,
            serial,
        });
        self.detail_job
            .request(work, |work| work.map(DetailWork::run));
        Ok(())
    }
    pub fn back(&mut self) -> bool {
        let Some(previous) = self.history.pop() else {
            return false;
        };
        self.detail_clock.cancel();
        self.detail_job.cancel();
        self.class_pair = None;
        self.controls.class_depth_open = false;
        self.detail_selection = Some(DetailSelection::Function(previous.selection.clone()));
        self.current = Some(previous);
        self.presentation_revision += 1;
        self.mounted_source = None;
        self.detail_message.clear();
        self.arm_source_restore();
        self.prefetch_selected_source();
        true
    }
    pub fn reset_counterpart(&mut self) -> Result<(), String> {
        let symbol = self
            .current
            .as_ref()
            .ok_or("No function pair is open")?
            .selection
            .symbol
            .clone();
        self.select(Selection {
            symbol,
            head_symbol: None,
        })
    }
    pub fn follow(&mut self, side: SideId, symbol: &str) -> Result<(), String> {
        let pair = self.current.as_ref().ok_or("No function pair is open")?;
        let Side::Ready(ready) = pair.side(side) else {
            return Err("This endpoint has no available flow".into());
        };
        let target = ready
            .targets
            .iter()
            .find(|node| node.symbol == symbol)
            .ok_or("Target does not belong to this endpoint")?;
        if target.body_unavailable {
            return Err("No body was recorded for this target".into());
        }
        self.select(Selection {
            symbol: target.symbol.clone(),
            head_symbol: None,
        })
    }
    pub fn can_fit(&self, side: SideId) -> bool {
        if let Some(pair) = self.class_pair() {
            return pair.has_root(side);
        }
        self.current
            .as_ref()
            .is_some_and(|pair| matches!(pair.side(side), Side::Ready(_)))
    }

    pub fn fit(&mut self, side: SideId) {
        let viewport = self.viewports[side.index()];
        if let Some(pair) = &mut self.class_pair {
            pair.fit(side, viewport);
            return;
        }
        let Some(pair) = &mut self.current else {
            return;
        };
        let Side::Ready(ready) = pair.side_mut(side) else {
            return;
        };
        if let Some(chart) = &ready.session.ready {
            ready.session.camera.fit(chart.chart.size, viewport);
        }
    }
}
