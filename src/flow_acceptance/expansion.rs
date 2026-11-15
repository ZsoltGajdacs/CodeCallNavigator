//! Opt-in native expansion journey over compiler-produced immutable inputs.
pub(super) mod config;
mod driver;
mod observation;
pub(super) mod report;
pub(super) use driver::Driver;
