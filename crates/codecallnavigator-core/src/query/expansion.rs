//! Adapt exact portable compiler facts to the core assembler; no live source or hydration.
use super::{Error, FlowExpansionRequest, Result};
use crate::{
    artifact::{
        Artifact,
        flow::{Input, Reason, Unavailable, View},
    },
    flow::{self, ExpandedFlow, ExpansionRequest, FunctionSource},
};
use std::{
    collections::BTreeSet,
    sync::atomic::{AtomicBool, Ordering},
};

pub fn expand_flow(
    artifact: &Artifact,
    query: &FlowExpansionRequest,
    cancel: &AtomicBool,
) -> Result<ExpandedFlow> {
    cancelled(cancel)?;
    let view = View::new(artifact).map_err(unavailable)?;
    expand_flow_view(&view, query, cancel)
}

/// Expand against a session-owned typed inventory without re-decoding artifact JSON.
pub fn expand_flow_view(
    view: &View<'_>,
    query: &FlowExpansionRequest,
    cancel: &AtomicBool,
) -> Result<ExpandedFlow> {
    cancelled(cancel)?;
    let request = prepare(query)?;
    let inputs = inputs(view, query, cancel)?;
    let borrowed: Vec<_> = inputs
        .iter()
        .map(|input| FunctionSource {
            function: &input.function,
            content: input.content,
        })
        .collect();
    let result = flow::expand(&borrowed, &request, cancel).map_err(|error| {
        if cancel.load(Ordering::Relaxed) {
            Error::new("cancelled", "Flow expansion cancelled")
        } else {
            Error::new("invalid_request", format!("{error:#}"))
        }
    })?;
    cancelled(cancel)?;
    Ok(result)
}
fn prepare(query: &FlowExpansionRequest) -> Result<ExpansionRequest> {
    if query.expansion_version != 1 {
        return Err(Error::new(
            "unsupported_expansion_version",
            "Unsupported flow expansion version",
        ));
    }
    if query.symbol.trim().is_empty() {
        return Err(Error::new(
            "invalid_request",
            "Expansion requires a root symbol",
        ));
    }
    let mut request = ExpansionRequest::new(&query.symbol).with_limits(query.limits);
    for selection in &query.selections {
        request = request.select(selection.caller.clone(), selection.choice.clone());
    }
    request
        .validate()
        .map_err(|error| Error::new("invalid_request", format!("{error:#}")))?;
    Ok(request)
}
fn inputs<'a>(
    view: &'a View<'_>,
    query: &FlowExpansionRequest,
    cancel: &AtomicBool,
) -> Result<Vec<Input<'a>>> {
    let root = view
        .input(&query.symbol)
        .map_err(unavailable)?
        .ok_or_else(|| {
            Error::new(
                "flow_unavailable",
                "The selected method is no longer in this graph.",
            )
        })?;
    let mut inputs = vec![root];
    for target in targets(query)
        .into_iter()
        .filter(|target| *target != query.symbol)
    {
        cancelled(cancel)?;
        if let Some(input) = optional_input(view, target)? {
            inputs.push(input);
        }
    }
    Ok(inputs)
}
fn targets(query: &FlowExpansionRequest) -> BTreeSet<&str> {
    query
        .selections
        .iter()
        .flat_map(|selection| {
            selection
                .caller
                .0
                .iter()
                .chain(std::iter::once(&selection.choice))
        })
        .map(|choice| choice.target.symbol.as_str())
        .collect()
}
fn optional_input<'a>(view: &'a View<'_>, symbol: &str) -> Result<Option<Input<'a>>> {
    match view.input(symbol) {
        Ok(input) => Ok(input),
        Err(error)
            if matches!(
                error.reason,
                Reason::UnsupportedLanguage
                    | Reason::BodyUnavailable
                    | Reason::MissingInventory
                    | Reason::MissingFunction
                    | Reason::MissingSource
            ) =>
        {
            Ok(None)
        }
        Err(error) => Err(unavailable(error)),
    }
}
fn unavailable(error: Unavailable) -> Error {
    let code = if matches!(
        error.reason,
        Reason::MissingSource | Reason::DuplicateSource | Reason::InvalidSource
    ) {
        "source_unavailable"
    } else {
        "flow_unavailable"
    };
    Error::new(code, error.message)
}
fn cancelled(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        Err(Error::new("cancelled", "Flow expansion cancelled"))
    } else {
        Ok(())
    }
}
