use super::super::comparison::endpoints::EndpointRequest;
use codecallnavigator_core::query::{Error, FlowComparisonSelection, Result};

pub(super) fn validate(
    endpoints: &EndpointRequest<'_>,
    selection: &FlowComparisonSelection,
) -> Result<()> {
    endpoints.validate_base()?;
    endpoints.validate_references()?;
    endpoints.validate_expected()?;
    match selection {
        FlowComparisonSelection::Catalog {
            text,
            offset,
            limit,
        } => {
            if !(1..=500).contains(limit) {
                return Err(Error::new(
                    "invalid_request",
                    "Flow catalog limit must be 1..500",
                ));
            }
            if text.len() > 1024 {
                return Err(Error::new(
                    "invalid_request",
                    "Flow catalog text exceeds 1024 bytes",
                ));
            }
            if *offset > 0 && endpoints.expected.is_none() {
                return Err(Error::new(
                    "invalid_request",
                    "Paged flow catalogs require expected_comparison",
                ));
            }
        }
        FlowComparisonSelection::Detail {
            symbol,
            head_symbol,
        } => {
            identity(symbol)?;
            if let Some(symbol) = head_symbol {
                identity(symbol)?;
            }
        }
    }
    Ok(())
}
fn identity(symbol: &str) -> Result<()> {
    if symbol.trim().is_empty() || symbol.len() > 16384 {
        return Err(invalid_identity());
    }
    if symbol.contains(['\0', '\n', '\r']) {
        return Err(invalid_identity());
    }
    Ok(())
}

fn invalid_identity() -> Error {
    Error::new(
        "invalid_request",
        "Flow symbol must contain 1..16384 bytes without control line separators",
    )
}
