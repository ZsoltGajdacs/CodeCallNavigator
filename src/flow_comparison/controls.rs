//! Editable comparison inputs never retarget an already captured revision pair.
use super::{Panel, Selection, Workspace};
use crate::{app::State, engine_client::flow_comparison::CatalogPage};
use rune::TextEdit;
pub struct Controls {
    pub visible: bool,
    pub catalog_open: bool,
    pub panel: Panel,
    pub base: TextEdit,
    pub head: TextEdit,
    pub search: TextEdit,
    pub choosing_after: bool,
    pub message: String,
    previous: Vec<usize>,
}
impl Default for Controls {
    fn default() -> Self {
        Self {
            visible: false,
            catalog_open: false,
            panel: Panel::Flows,
            base: TextEdit::new("HEAD"),
            head: TextEdit::new("WORKTREE"),
            search: TextEdit::new(""),
            choosing_after: false,
            message: String::new(),
            previous: Vec::new(),
        }
    }
}
impl State {
    pub fn show_flow_comparison(&mut self) {
        self.flow_residency.activity();
        self.close_source();
        self.flow_workspace.preserve_current_scroll();
        self.flow_comparison.controls.visible = true;
        self.check_impact_preview_support();
    }
    pub fn load_flow_comparison(&mut self) {
        self.flow_residency.activity();
        self.flow_comparison.impact.clear();
        self.flow_comparison.controls.panel = Panel::Flows;
        let refs = self.flow_comparison.control_refs();
        let result = if self.flow_comparison.descriptor().is_some() {
            self.flow_comparison.load_refs(refs)
        } else {
            self.open_selected_flow_comparison(refs)
        };
        if let Err(error) = result {
            self.flow_comparison.controls.message = error;
        }
    }
}
enum PageDirection {
    Next,
    Previous,
}
impl Controls {
    pub(super) fn reset_binding(&mut self) {
        self.previous.clear();
        self.search = TextEdit::new("");
        self.choosing_after = false;
        self.message.clear();
    }
}
impl Workspace {
    pub fn reload_control(&mut self) {
        let result = self.reload();
        self.action(result);
    }
    pub fn cancel_control(&mut self) {
        if self.impact.pending() {
            self.impact.cancel();
            return;
        }
        if self.snapshots.pending() {
            self.snapshots.cancel();
            return;
        }
        self.close();
        self.controls.visible = true;
        self.controls.message = "Comparison loading cancelled".into();
    }
    pub fn search_control(&mut self) {
        let result = self.page(CatalogPage {
            text: self.controls.search.text.clone(),
            offset: 0,
            limit: 50,
        });
        if result.is_ok() {
            self.controls.previous.clear();
        }
        self.action(result);
    }
    pub fn next_control(&mut self) {
        self.turn_page(PageDirection::Next);
    }
    pub fn previous_control(&mut self) {
        self.turn_page(PageDirection::Previous);
    }
    fn turn_page(&mut self, direction: PageDirection) {
        let Some(page) = &self.catalog else {
            return;
        };
        let offset = match direction {
            PageDirection::Next => page.next_offset,
            PageDirection::Previous => self.controls.previous.last().copied(),
        };
        let Some(offset) = offset else {
            return;
        };
        let old = page.offset;
        let request = CatalogPage {
            text: page.text.clone(),
            offset,
            limit: page.limit,
        };
        let result = self.page(request);
        if result.is_ok() {
            match direction {
                PageDirection::Next => self.controls.previous.push(old),
                PageDirection::Previous => {
                    self.controls.previous.pop();
                }
            }
        }
        self.action(result);
    }
    pub fn has_previous_page(&self) -> bool {
        !self.controls.previous.is_empty()
    }
    pub fn choose_control(&mut self, symbol: String) {
        let selection = if self.controls.choosing_after {
            let Some(pair) = &self.current else {
                return;
            };
            Selection {
                symbol: pair.selection.symbol.clone(),
                head_symbol: Some(symbol),
            }
        } else {
            Selection {
                symbol,
                head_symbol: None,
            }
        };
        self.controls.choosing_after = false;
        self.controls.catalog_open = false;
        let result = self.select(selection);
        self.action(result);
    }
    pub fn reset_control(&mut self) {
        let result = self.reset_counterpart();
        self.action(result);
    }
    fn action(&mut self, result: Result<(), String>) {
        self.controls.message = result.err().unwrap_or_default();
    }
}
#[cfg(test)]
#[path = "controls_tests.rs"]
mod tests;
