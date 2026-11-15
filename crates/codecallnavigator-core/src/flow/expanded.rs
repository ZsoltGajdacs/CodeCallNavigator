//! Immutable expanded-result contracts. Assembly and input eligibility belong to the builder.
mod address;
mod blocked;
mod chart_validation;
mod result;
mod sources;
mod step_validation;
mod validation;
pub use address::*;
pub use result::*;
pub use sources::{SourceId, SourceOwner, SourceSnapshot, SourceTable};
mod limits;
mod request;
pub use super::builder::expand;
pub use limits::ExpansionLimits;
pub(crate) use request::RequestedCall;
pub use request::{ExpansionRequest, FunctionSource};
