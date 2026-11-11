//! Explain the source facts establishing an inferred relationship.
use codecallnavigator::{
    graph::GraphIndex,
    model::{Edge, RelationshipEvidence, SourceSite},
};
use rune::{El, col, text};
use rune_kit::Kit;

pub(super) fn panel(edge: &Edge, graph: &GraphIndex, k: &Kit) -> El {
    let mut panel = col().w_full().gap(6.0);
    if edge.evidence.is_empty() {
        return panel;
    }
    panel = panel.child(
        text("Relationship evidence")
            .font(k.ty.ui_medium)
            .color(k.c.text_base),
    );
    for evidence in &edge.evidence {
        for line in lines(evidence, graph) {
            panel = panel.child(
                text(line)
                    .w_full()
                    .wrap_anywhere()
                    .selectable(true)
                    .font(k.ty.text_xsmall)
                    .color(k.c.text_muted),
            );
        }
    }
    panel
}

fn lines(evidence: &RelationshipEvidence, graph: &GraphIndex) -> Vec<String> {
    let mut lines = vec![
        format!("Rule: {} · v{}", evidence.rule, evidence.version),
        format!("Semantics: {}", evidence.semantics),
        format!("Evidence site: {}", site(&evidence.invocation)),
    ];
    if let Some(registration) = &evidence.registration {
        lines.push(format!("Registration: {}", site(registration)));
    }
    if let Some(channel) = &evidence.channel {
        lines.push(format!("Channel: {channel}"));
    }
    for (name, value) in &evidence.qualifiers {
        lines.push(format!("{name}: {value}"));
    }
    for id in &evidence.supporting_symbols {
        if let Some(index) = graph.node_index(id) {
            lines.push(format!(
                "Supporting symbol: {}",
                graph.data.nodes[index as usize].signature
            ));
        }
    }
    lines
}

fn site(site: &SourceSite) -> String {
    let mut location = format!("{}:{}", site.file, site.line);
    if let Some(column) = site.column {
        location.push_str(&format!(":{column}"));
    }
    if let Some(span) = site.span {
        location.push_str(&format!(" · bytes {}..{}", span.start, span.end));
    }
    location
}
