//! Filesystem observation consumes immutable CLI artifacts; it owns no graph cache.
use super::query::freshness;
use codecallnavigator_core::{
    artifact::{Artifact, Roots, verification},
    freshness::{self as contract, Coverage, CoverageKind, FreshnessObservation, Mode},
    query::{self, Error},
};
use serde_json::Value;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

pub(super) struct Observed {
    pub current: FreshnessObservation,
    pub legacy: Value,
}

pub(super) fn observe(
    artifact: &Artifact,
    generation: &str,
    roots: &Roots,
    cancel: &AtomicBool,
) -> query::Result<Observed> {
    let started = Instant::now();
    let observed =
        verification::observe_artifact(artifact, generation, roots, cancel).map_err(|error| {
            let code = if cancel.load(Ordering::Acquire) {
                "cancelled"
            } else {
                "observation_failed"
            };
            Error::new(code, format!("Cannot observe saved inputs: {error:#}"))
        })?;
    let bounded = freshness::bounded(observed.report)?;
    let current = FreshnessObservation {
        version: contract::VERSION,
        generation: generation.into(),
        input_revision: observed.input_revision,
        contract_valid: bounded.report.contract_valid,
        input_status: bounded.report.input_status,
        reasons: bounded.report.reasons.clone(),
        reasons_total: bounded.total,
        omitted_reasons: bounded.omitted,
        coverage: Coverage {
            kind: CoverageKind::ObservedInputs,
            complete: observed.complete,
            certifies_build: false,
        },
        mode: Mode::default(),
        verification_milliseconds: started.elapsed().as_secs_f64() * 1000.0,
    };
    Ok(Observed {
        current,
        legacy: bounded.value()?,
    })
}
