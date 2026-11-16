//! Generation-bound expansion uses only the loaded canonical artifact and its saved sources.
use super::{
    cancellation,
    query::{self, Loaded},
    response_encoding::Reply,
};
use codecallnavigator_core::query::{self as core, Error, Query, Request, Response, Result};

pub(super) fn validate(request: &Request) -> Result<()> {
    let Query::FlowExpansion(expansion) = &request.query else {
        return Ok(());
    };
    if expansion.expansion_version != 1 {
        return Err(Error::new(
            "unsupported_expansion_version",
            "Unsupported flow expansion version",
        ));
    }
    if request
        .expected_generation
        .as_deref()
        .is_none_or(|generation| generation.trim().is_empty())
    {
        return Err(Error::new(
            "invalid_request",
            "Flow expansion requires expected_generation from its saved snapshot",
        ));
    }
    Ok(())
}
pub(super) fn respond(loaded: &mut Loaded, request: &Request) -> Option<Response> {
    let Query::FlowExpansion(expansion) = &request.query else {
        return None;
    };
    let result = query::check_generation(request, &loaded.generation).and_then(|()| {
        loaded.refresh_freshness()?;
        loaded.ensure_flows()?;
        let view = loaded.flow_view()?;
        let expanded = core::expand_flow_view(&view, expansion, &cancellation::CANCELLED)?;
        Reply {
            generation: &loaded.generation,
            freshness: None,
        }
        .finish_with(
            &expanded,
            Error::new(
                "response_too_large",
                "A complete flow expansion exceeds the 24 MiB response budget",
            ),
        )
    });
    Some(result.unwrap_or_else(|error| query::envelope(Some(loaded), Err(error))))
}
