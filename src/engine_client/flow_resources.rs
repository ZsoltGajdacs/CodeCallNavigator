//! Explicit session resource control; idle policy belongs to the desktop.
use super::{QueryContext, query_process::ReleaseGuard};
use anyhow::{Context, Result, bail, ensure};
use codecallnavigator::query::{FlowResourceAction, FlowResourceStatus, Query};
use std::sync::atomic::{AtomicBool, AtomicU64};

impl QueryContext {
    pub(crate) fn release_flows_if_current(
        &self,
        revision: &AtomicU64,
        expected: u64,
        cancel: &AtomicBool,
    ) -> Result<()> {
        let request = self.wire_request(Query::FlowResources {
            action: FlowResourceAction::Unload,
        });
        let Some(response) =
            self.client
                .release_flows(&request, ReleaseGuard::new(revision, expected), cancel)?
        else {
            return Ok(());
        };
        if let Some(error) = response.error {
            // The old generation is already gone; it has no resources to retain.
            if error.code == "generation_mismatch" || error.code == "graph_missing" {
                return Ok(());
            }
            bail!("{}: {}", error.code, error.message);
        }
        ensure!(
            response.generation == self.descriptor.generation,
            "CLI released another graph generation"
        );
        let status: FlowResourceStatus = serde_json::from_value(
            response
                .result
                .context("CLI omitted flow residency status")?,
        )
        .context("Invalid CLI flow residency status")?;
        ensure!(
            !status.loaded,
            "CLI retained the flow inventory after release"
        );
        ensure!(
            status.retained_bytes == 0,
            "CLI retained flow allocations after release"
        );
        Ok(())
    }
}
