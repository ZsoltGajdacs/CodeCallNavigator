//! Pair navigation never consults the currently installed graph or its generation.
use super::{Selection, Side, SideId, Workspace, requests::DetailWork};
impl Workspace {
    pub fn select(&mut self, selection: Selection) -> Result<(), String> {
        let bound = self
            .binding
            .clone()
            .ok_or("Reload the comparison before selecting a function")?;
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
        self.changes.preview = None;
        self.detail_message = "Preparing before and after flowcharts…".into();
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
    pub fn fit(&mut self, side: SideId) {
        let viewport = self.viewports[match side {
            SideId::Before => 0,
            SideId::After => 1,
        }];
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
