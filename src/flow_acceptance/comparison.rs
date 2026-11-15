//! Opt-in version two comparison acceptance, independent of the ordinary version one driver.
pub(super) mod config;
mod driver;
mod observation;
pub(super) mod protocol;
pub(super) mod report;
pub(super) use driver::Driver;
mod source_observation;
mod surface;
