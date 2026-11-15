//! Publish only matching request serials and bindings, clearing stale actionable results.
use super::{
    Phase, Workspace,
    requests::{CatalogAnswer, StringError},
};
impl Workspace {
    pub fn poll(&mut self) -> bool {
        let identity = self.identity.poll(self.catalog_clock.serial);
        if identity == Some(super::identity::Installation::Different) {
            self.close();
            return true;
        }
        let catalog = self.publish_catalog();
        let detail = self.publish_detail();
        identity.is_some() | catalog | detail
    }
    fn publish_catalog(&mut self) -> bool {
        let Some(output) = self.catalog_job.take() else {
            return false;
        };
        output.map(|output| self.accept_catalog(output)).value
    }
    fn accept_catalog(&mut self, output: super::requests::CatalogOutput) -> bool {
        if output.serial != self.catalog_clock.serial {
            return false;
        }
        if self.identity.completed(output.physical_project.as_deref())
            == super::identity::Installation::Different
        {
            // Discard every outcome while Project still pins the client, before retirement.
            drop(output);
            self.close();
            return true;
        }
        match output.result {
            Ok(CatalogAnswer::Opened(answer)) => {
                let super::requests::Opened { bound, page } = *answer;
                self.binding = Some(bound);
                self.catalog = Some(std::sync::Arc::new(page));
                self.phase = Phase::Bound;
            }
            Ok(CatalogAnswer::Page(page)) => {
                if self
                    .header()
                    .is_none_or(|header| header.comparison_id != page.header.comparison_id)
                {
                    return false;
                }
                self.catalog = Some(std::sync::Arc::from(page));
            }
            Err(error) => self.failed(StringError::Query(error)),
        }
        true
    }
    fn publish_detail(&mut self) -> bool {
        let Some(output) = self.detail_job.take() else {
            return false;
        };
        output.map(|output| self.accept_detail(output)).value
    }
    fn accept_detail(&mut self, output: super::requests::DetailOutput) -> bool {
        if output.serial != self.detail_clock.serial
            || self
                .header()
                .is_none_or(|header| header.comparison_id != output.binding)
        {
            return false;
        }
        match output.result {
            Ok(prepared) => {
                self.visit += 1;
                self.presentation_revision += 1;
                let namespace = format!("comparison:{}:{}", output.binding, self.visit);
                self.current = Some(prepared.install(output.selection, &namespace, self.viewports));
                self.detail_message.clear();
                self.arm_source_restore();
            }
            Err(error) => self.failed(error),
        }
        true
    }
    fn failed(&mut self, error: StringError) {
        self.controls.catalog_open = false;
        self.controls.choosing_after = false;
        self.cancel_requests();
        self.binding = None;
        self.catalog = None;
        self.current = None;
        self.history.clear();
        self.detail_message.clear();
        self.pending_scroll = None;
        self.phase = if error.mismatch() {
            Phase::ReloadRequired(error.message())
        } else {
            Phase::Failed(error.message())
        };
    }
}
#[cfg(test)]
#[path = "publication_race_tests.rs"]
mod race_tests;
