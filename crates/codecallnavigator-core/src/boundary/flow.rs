//! Attach boundary targets to existing flow calls without inventing call sites.
use super::{BoundaryUse, PROVENANCE};
use crate::{
    flow::{CallTarget, visit_calls_mut},
    model::GraphData,
};
pub(super) fn attach(graph: &mut GraphData, usage: &BoundaryUse, target: &str) -> bool {
    let Some(function) = graph.flow_inventory.as_mut().and_then(|inventory| {
        inventory
            .functions
            .iter_mut()
            .find(|function| function.symbol == usage.symbol && function.file == usage.site.file)
    }) else {
        return false;
    };
    let mut found = false;
    visit_calls_mut(function, &mut |call| {
        if Some(call.span) != usage.site.span {
            return;
        }
        let target = CallTarget {
            symbol: target.into(),
            certainty: "possible".into(),
            provenance: PROVENANCE.into(),
            kind: usage.kind.clone(),
        };
        if !call.targets.contains(&target) {
            call.targets.push(target);
            call.targets.sort_by(|a, b| {
                (&a.symbol, &a.kind, &a.provenance).cmp(&(&b.symbol, &b.kind, &b.provenance))
            });
        }
        let message = format!(
            "Possible {} boundary; runtime dispatch is not proven",
            usage.kind
        );
        call.diagnostic = Some(match &call.diagnostic {
            Some(previous) => format!("{previous}\n{message}"),
            None => message,
        });
        found = true;
    });
    found
}

pub(super) fn clear(graph: &mut GraphData) {
    if let Some(inventory) = &mut graph.flow_inventory {
        for function in &mut inventory.functions {
            visit_calls_mut(function, &mut |call| {
                call.targets
                    .retain(|target| target.provenance != PROVENANCE);
                call.diagnostic = call.diagnostic.as_ref().and_then(|text| {
                    let remaining = text
                        .lines()
                        .filter(|line| {
                            !line.starts_with("Possible http boundary;")
                                && !line.starts_with("Possible navigation boundary;")
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    (!remaining.is_empty()).then_some(remaining)
                });
            });
        }
    }
}
