//! Attributed MVC endpoints retain unsupported competitors for shared boundary matching.
use crate::{
    discovery::BuildTarget,
    model::{EntryPoints, GraphData, SourceSite},
};
use anyhow::{Context, Result, ensure};
use codegraph_core::boundary::{BoundaryInventory, RouteFact};
use serde::Deserialize;
use std::{collections::HashSet, path::Path};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawRoute {
    symbol: Option<String>,
    application: String,
    path: Option<String>,
    methods: Vec<String>,
    site: RawSite,
    restrictions: Vec<String>,
    order: Option<i32>,
}

#[derive(Deserialize)]
struct RawSite {
    file: String,
    line: u32,
    column: u32,
}

pub(super) fn append(
    build: &BuildTarget,
    root: &Path,
    graph: &mut GraphData,
    routes: Vec<RawRoute>,
) -> Result<()> {
    if routes.is_empty() {
        return Ok(());
    }
    let symbols: HashSet<_> = graph.nodes.iter().map(|node| node.id.as_str()).collect();
    let routes = routes
        .into_iter()
        .map(|route| normalize(build, root, route, &symbols))
        .collect::<Result<Vec<_>>>()?;
    let entries = &mut graph
        .entry_points
        .get_or_insert_with(EntryPoints::default)
        .framework_entries;
    entries.extend(routes.iter().filter_map(|route| route.symbol.clone()));
    entries.sort();
    entries.dedup();
    codegraph_core::boundary::merge(
        &mut graph.boundaries,
        Some(BoundaryInventory {
            version: 1,
            routes,
            uses: Vec::new(),
        }),
    )?;
    Ok(())
}

fn normalize(
    build: &BuildTarget,
    root: &Path,
    mut route: RawRoute,
    symbols: &HashSet<&str>,
) -> Result<RouteFact> {
    ensure!(
        !route.application.trim().is_empty(),
        "ASP.NET route omitted application owner"
    );
    let symbol = route
        .symbol
        .map(|id| format!("{}|{id}", build.path.display()));
    let symbol = match symbol {
        Some(id) if symbols.contains(id.as_str()) => Some(id),
        Some(_) => {
            route
                .restrictions
                .push("aspnet.missing-native-action".into());
            None
        }
        None => None,
    };
    ensure!(
        route.site.line > 0 && route.site.column > 0 && !route.site.file.is_empty(),
        "ASP.NET route omitted source site"
    );
    let (file, line) = super::location(&format!("{}:{}", route.site.file, route.site.line), root);
    ensure!(!file.is_empty(), "ASP.NET route has invalid source file");
    route.restrictions.sort();
    route.restrictions.dedup();
    Ok(RouteFact {
        symbol,
        build: build
            .path
            .to_str()
            .context("Build path is not UTF-8")?
            .into(),
        application: route.application,
        framework: "aspnet".into(),
        kind: "http".into(),
        path: route.path,
        methods: route.methods,
        site: SourceSite {
            file,
            line,
            column: Some(route.site.column),
            span: None,
        },
        restrictions: route.restrictions,
        order: route.order,
    })
}
