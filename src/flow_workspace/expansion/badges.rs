//! Present explicit stopped outcomes once per published result, never infer activation.
use codecallnavigator::flow::{ExpandedFlow, ExpansionLimit, ExpansionStop, OccurrenceOutcome};
use std::collections::HashMap;
pub(super) fn collect(result: &ExpandedFlow) -> HashMap<usize, String> {
    let mut markers: HashMap<usize, Vec<&ExpansionStop>> = HashMap::new();
    for occurrence in &result.occurrences {
        if let (Some(marker), OccurrenceOutcome::Stopped { reason, .. }) =
            (occurrence.boundaries.marker, &occurrence.outcome)
        {
            markers.entry(marker).or_default().push(reason);
        }
    }
    markers
        .into_iter()
        .map(|(step, reasons)| {
            let label = if reasons.len() == 1 {
                label(reasons[0]).into()
            } else {
                format!("Stopped choices ({})", reasons.len())
            };
            (step, label)
        })
        .collect()
}
fn label(reason: &ExpansionStop) -> &'static str {
    match reason {
        ExpansionStop::Cycle => "Cycle",
        ExpansionStop::Ineligible => "Not expanded",
        ExpansionStop::Unavailable => "Body unavailable",
        ExpansionStop::Unselected => "Not selected",
        ExpansionStop::Limit { limit } => match limit {
            ExpansionLimit::Depth => "Depth limit",
            ExpansionLimit::Requests => "Request limit",
            ExpansionLimit::Steps => "Step limit",
            ExpansionLimit::Links => "Link limit",
            ExpansionLimit::SourceBytes => "Source limit",
        },
    }
}
