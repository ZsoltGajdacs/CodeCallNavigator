//! Endpoint provenance comes from the bound CLI response, never the current graph.
use crate::{
    app::State,
    engine_client::flow_comparison::{Endpoint, EndpointKind, EndpointStatus},
};
use rune::{El, col, row, text};
use rune_kit::Kit;
pub(super) fn view(state: &State, k: &Kit) -> El {
    let Some(header) = state.flow_comparison.header() else {
        return rune::el();
    };
    row()
        .key("comparison-provenance")
        .w_full()
        .gap(12.0)
        .px(10.0)
        .pb(4.0)
        .child(endpoint("Before", &header.base, k))
        .child(endpoint("After", &header.head, k))
}
fn endpoint(title: &str, endpoint: &Endpoint, k: &Kit) -> El {
    let commit = endpoint.commit.as_deref().unwrap_or("saved WORKTREE");
    let summary = match endpoint.kind {
        EndpointKind::Commit => format!(
            "{title}: {} · commit {}",
            endpoint.requested_ref,
            &commit[..commit.len().min(12)]
        ),
        EndpointKind::Worktree => format!("{title}: {} · saved WORKTREE", endpoint.requested_ref),
    };
    col()
        .grow(1.0)
        .min_w(0.0)
        .gap(1.0)
        .child(
            text(summary)
                .font(k.ty.small_medium)
                .color(k.c.text_base)
                .w_full()
                .truncate()
                .help(format!(
                    "Requested: {}\nResolved commit: {commit}",
                    endpoint.requested_ref
                )),
        )
        .child(
            text(format!(
                "{} · schema {}",
                endpoint.path,
                endpoint.schema.as_deref().unwrap_or("unknown")
            ))
            .font(k.ty.small)
            .color(k.c.text_muted)
            .w_full()
            .truncate(),
        )
        .child(
            text(format!("Artifact {}", endpoint.digest))
                .font(k.ty.small)
                .color(k.c.text_muted)
                .w_full()
                .truncate()
                .help(&endpoint.digest),
        )
}
pub(super) fn partial(state: &State) -> Option<String> {
    let page = state.flow_comparison.catalog.as_ref()?;
    if page.complete {
        return None;
    }
    let mut reasons = Vec::new();
    for (name, status) in [
        ("Before", &page.endpoints.before),
        ("After", &page.endpoints.after),
    ] {
        if let EndpointStatus::Unavailable(failure) = status {
            reasons.push(format!("{name}: {}", failure.message));
        }
    }
    Some(format!("Partial comparison · {}", reasons.join(" · ")))
}
