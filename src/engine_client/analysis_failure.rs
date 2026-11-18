//! Preserve engine analysis failure context alongside transport errors.
use codecallnavigator::protocol::{AnalysisFailureCode, TargetFailure};
use std::{error::Error, fmt};

#[derive(Debug)]
pub(crate) struct AnalysisFailure {
    pub code: Option<AnalysisFailureCode>,
    pub message: String,
    pub target: Option<TargetFailure>,
}

impl fmt::Display for AnalysisFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(code) = self.code {
            write!(f, "{}: ", code.as_str())?;
        }
        f.write_str(&self.message)
    }
}

impl Error for AnalysisFailure {}
