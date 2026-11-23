//! Attribute changed call facts only to uniquely corresponding steps in identical source.
use crate::flow_workspace::Ready;
use codecallnavigator::flow::{CallSite, FlowStep, SourceSpan};
use std::collections::{HashMap, HashSet};

type StepKey<'a> = (SourceSpan, &'a str, &'a str);

#[derive(Clone, Debug, Default)]
pub(super) struct CallChanges([Vec<bool>; 2]);

impl CallChanges {
    pub fn between(before: &Ready, after: &Ready) -> Self {
        if before.chart.file != after.chart.file || before.source.text != after.source.text {
            return Self::default();
        }
        let old = &before.chart.steps;
        let new = &after.chart.steps;
        let old_steps = index(old, before.source.text.len());
        let new_steps = index(new, after.source.text.len());
        let mut changes = [vec![false; old.len()], vec![false; new.len()]];
        for (key, old_index) in old_steps {
            let Some((old_index, new_index)) =
                old_index.zip(new_steps.get(&key).copied().flatten())
            else {
                continue;
            };
            if calls_changed(&old[old_index].calls, &new[new_index].calls) {
                changes[0][old_index] = true;
                changes[1][new_index] = true;
            }
        }
        Self(changes)
    }

    pub fn at(&self, side: super::SideId, step: usize) -> bool {
        self.0[side.index()].get(step).copied().unwrap_or(false)
    }
}

fn index(steps: &[FlowStep], source_len: usize) -> HashMap<StepKey<'_>, Option<usize>> {
    let mut result = HashMap::new();
    for (number, step) in steps.iter().enumerate() {
        if step.span.start < step.span.end && step.span.end <= source_len {
            result
                .entry((step.span, step.kind.as_str(), step.label.as_str()))
                .and_modify(|unique| *unique = None)
                .or_insert(Some(number));
        }
    }
    result
}

fn calls_changed(old: &[CallSite], new: &[CallSite]) -> bool {
    old.iter().zip(new).any(|(old, new)| {
        same_site(old, new)
            && old.targets != new.targets
            && old.targets.iter().collect::<HashSet<_>>()
                != new.targets.iter().collect::<HashSet<_>>()
    })
}

fn same_site(before: &CallSite, after: &CallSite) -> bool {
    (before.span, before.label.as_str()) == (after.span, after.label.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow_comparison::SideId;
    use codecallnavigator::flow::{CallTarget, FlowChart};
    use std::path::Path;

    #[test]
    fn identical_source_marks_only_unique_steps_with_changed_call_targets() {
        let direct = target("Service.Check", "direct");
        let possible = target("Service.Check", "possible");
        let other = target("Other.Check", "direct");
        let before = ready("Check();", vec![step(vec![direct.clone(), other.clone()])]);
        let after = ready("Check();", vec![step(vec![possible, other.clone()])]);

        let changed = CallChanges::between(&before, &after);
        assert!(changed.at(SideId::Before, 0));
        assert!(changed.at(SideId::After, 0));

        let reordered = ready("Check();", vec![step(vec![other, direct.clone()])]);
        assert!(!CallChanges::between(&before, &reordered).at(SideId::After, 0));

        let repeated = ready(
            "Check();",
            vec![step(vec![direct.clone()]), step(vec![direct])],
        );
        assert!(!CallChanges::between(&repeated, &after).at(SideId::After, 0));

        let different_text = ready("Other();", after.chart.steps.clone());
        assert!(!CallChanges::between(&before, &different_text).at(SideId::After, 0));
    }

    fn target(symbol: &str, certainty: &str) -> CallTarget {
        CallTarget {
            symbol: symbol.into(),
            certainty: certainty.into(),
            ..Default::default()
        }
    }

    fn step(targets: Vec<CallTarget>) -> FlowStep {
        let span = SourceSpan { start: 0, end: 7 };
        FlowStep {
            kind: "action".into(),
            label: "Check()".into(),
            span,
            calls: vec![CallSite {
                span,
                label: "Check()".into(),
                targets,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn ready(text: &str, steps: Vec<FlowStep>) -> Ready {
        let file = "Service.cs";
        Ready {
            chart: FlowChart {
                file: file.into(),
                steps,
                ..Default::default()
            },
            source: crate::source::from_snapshot(text.into(), Path::new(file)),
        }
    }
}
