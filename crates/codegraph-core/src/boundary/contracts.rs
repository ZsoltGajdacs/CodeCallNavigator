use crate::model::SourceSite;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryInventory {
    pub version: u32,
    #[serde(default)]
    pub routes: Vec<RouteFact>,
    #[serde(default)]
    pub uses: Vec<BoundaryUse>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteFact {
    pub symbol: Option<String>,
    pub build: String,
    /// Stable adapter-owned application label, scoped by build, never a checkout path.
    pub application: String,
    pub framework: String,
    pub kind: String,
    /// Shared template uses {name} for one segment. Unknown paths block the owner.
    pub path: Option<String>,
    #[serde(default)]
    pub methods: Vec<String>,
    pub site: SourceSite,
    #[serde(default)]
    pub restrictions: Vec<String>,
    /// Default is zero. Unknown order blocks competing targets.
    pub order: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryUse {
    pub symbol: String,
    pub build: String,
    pub application: String,
    pub kind: String,
    pub path: String,
    pub method: Option<String>,
    pub site: SourceSite,
    pub rule: String,
    /// True only for an actual call with an exact flow span; JSX links are false.
    pub flow: bool,
}

impl BoundaryInventory {
    pub fn validate(&self, symbols: &HashSet<&str>) -> Result<()> {
        ensure!(self.version == 1, "Unsupported boundary inventory version");
        for route in &self.routes {
            owner(&route.build, &route.application, &route.kind)?;
            ensure!(
                matches!(route.framework.as_str(), "next" | "spring" | "aspnet"),
                "Unknown boundary framework"
            );
            if let Some(symbol) = &route.symbol {
                ensure!(
                    symbols.contains(symbol.as_str()),
                    "Route references missing symbol: {symbol}"
                );
            }
            ensure!(
                route.methods.iter().all(|method| method
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase())
                    && !method.is_empty()),
                "Invalid route method"
            );
            route.site.validate()?;
        }
        for usage in &self.uses {
            owner(&usage.build, &usage.application, &usage.kind)?;
            ensure!(
                symbols.contains(usage.symbol.as_str()),
                "Boundary use references missing symbol"
            );
            ensure!(!usage.rule.trim().is_empty(), "Boundary use needs a rule");
            ensure!(
                !usage.flow || usage.site.span.is_some(),
                "Boundary flow needs exact call span"
            );
            usage.site.validate()?;
        }
        Ok(())
    }
}
fn owner(build: &str, application: &str, kind: &str) -> Result<()> {
    ensure!(
        !build.is_empty() && !application.trim().is_empty(),
        "Boundary fact needs build/application owner"
    );
    ensure!(
        matches!(kind, "http" | "navigation"),
        "Invalid boundary kind"
    );
    Ok(())
}

pub fn merge(
    target: &mut Option<BoundaryInventory>,
    incoming: Option<BoundaryInventory>,
) -> Result<()> {
    let Some(mut incoming) = incoming else {
        return Ok(());
    };
    ensure!(
        incoming.version == 1,
        "Unsupported boundary inventory version"
    );
    for route in &mut incoming.routes {
        route.methods.sort();
        route.methods.dedup();
        route.restrictions.sort();
        route.restrictions.dedup();
    }
    dedup(&mut incoming.routes);
    dedup(&mut incoming.uses);
    if let Some(target) = target {
        target.routes.append(&mut incoming.routes);
        target.uses.append(&mut incoming.uses);
        dedup(&mut target.routes);
        dedup(&mut target.uses);
    } else {
        *target = Some(incoming);
    }
    Ok(())
}
fn dedup<T: Serialize>(items: &mut Vec<T>) {
    items.sort_by_cached_key(|item| {
        serde_json::to_string(item).expect("boundary fact serialization")
    });
    items.dedup_by(|a, b| serde_json::to_string(a).unwrap() == serde_json::to_string(b).unwrap());
}
