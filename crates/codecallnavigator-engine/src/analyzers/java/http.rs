//! Compiler-qualified Spring mappings retain unsupported declarations as competitors.
use super::{
    flow_inventory::BindingContext, normalize::identity_prefix, source_inventory::Inventories,
};
use crate::model::{GraphData, SourceSite};
use anyhow::Result;
use codecallnavigator_core::boundary::{BoundaryInventory, RouteFact};
use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct SourceRoute {
    symbol: Option<String>,
    path: Option<String>,
    methods: Vec<String>,
    site: SourceSite,
    restrictions: Vec<String>,
}

pub(super) fn attach(
    context: BindingContext<'_>,
    inventories: &Inventories,
    graph: &mut GraphData,
) -> Result<()> {
    let mut routes = Vec::new();
    for (&input, inventory) in inventories {
        let Some(source) = context.inputs[input - 1].source_set.as_ref() else {
            continue;
        };
        let prefix = identity_prefix(context.build, source);
        if context.owner.is_some_and(|owner| owner != prefix) {
            continue;
        }
        for route in &inventory.http_routes {
            let mut restrictions = route.restrictions.clone();
            let symbol = route
                .symbol
                .as_ref()
                .map(|key| format!("{prefix}{key}"))
                .filter(|id| graph.nodes.iter().any(|node| &node.id == id));
            if symbol.is_none() {
                restrictions.push("spring.handler_identity_unavailable".into());
            }
            routes.push(RouteFact {
                symbol,
                build: context.build.path.to_string_lossy().into_owned(),
                application: source
                    .application
                    .clone()
                    .unwrap_or_else(|| source.project.clone()),
                framework: "spring".into(),
                kind: "http".into(),
                path: route.path.clone(),
                methods: route.methods.clone(),
                site: route.site.clone(),
                restrictions,
                order: Some(0),
            });
        }
    }
    if !routes.is_empty() {
        let entries = graph.entry_points.get_or_insert_with(Default::default);
        entries
            .framework_entries
            .extend(routes.iter().filter_map(|route| route.symbol.clone()));
        entries.framework_entries.sort();
        entries.framework_entries.dedup();
        codecallnavigator_core::boundary::merge(
            &mut graph.boundaries,
            Some(BoundaryInventory {
                version: 1,
                routes,
                uses: Vec::new(),
            }),
        )?;
        graph.diagnostics.push("Spring HTTP routes describe compiler-qualified controller mappings; deployment context paths, global MVC prefixes, bean registration and custom runtime routing are not proven. Unsupported constraints and inherited/composed mappings remain ambiguity blockers.".into());
    }
    Ok(())
}
