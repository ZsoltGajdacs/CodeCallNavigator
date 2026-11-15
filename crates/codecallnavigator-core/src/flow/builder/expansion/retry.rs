//! Demotions grow monotonically; graph size need not. No failed graph/source text is cloned.
use super::*;
use budget::BudgetExceeded;
use std::rc::Rc;
pub(super) fn expand<'a>(
    prepared: Prepared<'a>,
    request: &ExpansionRequest,
    cancel: &'a AtomicBool,
) -> Result<ExpandedFlow> {
    let prepared = Rc::new(prepared);
    // Use exactly the same assembler, with every selection stopped, to prove
    // mandatory caller work fits before trying any speculative child bodies.
    drop(
        attempt::assemble(prepared.clone(), request, attempt::Mode::RootOnly, cancel)
            .context("Mandatory root flow does not fit expansion limits")?,
    );
    let count = prepared.requests.len();
    let mut demotions = vec![None; count];
    for _ in 0..=count {
        match attempt::assemble(
            prepared.clone(),
            request,
            attempt::Mode::Expand(demotions.clone()),
            cancel,
        ) {
            Ok(complete) => return complete.materialize(request.root(), cancel),
            Err(error) => {
                let Some(failure) = error.downcast_ref::<BudgetExceeded>().copied() else {
                    return Err(error);
                };
                let Some(candidate) = failure.candidate.filter(|index| *index < count) else {
                    return Err(error.context("No selected expansion can be demoted"));
                };
                ensure!(
                    demotions[candidate].is_none(),
                    "Expansion fallback could not make progress"
                );
                demotions[candidate] = Some(failure.limit);
            }
        }
    }
    anyhow::bail!("Expansion fallback exceeded the bounded request count")
}
