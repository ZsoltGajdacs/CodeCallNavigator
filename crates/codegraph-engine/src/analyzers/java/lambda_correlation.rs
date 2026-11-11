//! Validate compiler correlation evidence once at the source-inventory boundary.
use super::flow_inventory::SourceFlow;
use anyhow::{Result, ensure};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

type Sources<'a> = HashMap<&'a str, (usize, &'a SourceFlow)>;

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(super) enum CorrelationStatus {
    Verified,
    Rejected,
    Unavailable,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Correlation {
    owner: String,
    pub status: CorrelationStatus,
    pub reason: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBinding {
    implementation: String,
    sources: Vec<String>,
}

pub(super) struct LambdaBinding {
    pub implementation: String,
    pub sources: Vec<usize>,
}

#[derive(Default)]
pub(super) struct LambdaCorrelations {
    pub bindings: Vec<LambdaBinding>,
    owners: HashMap<String, Correlation>,
}

impl LambdaCorrelations {
    pub fn permits_line_fallback(&self, owner: &str) -> bool {
        self.owners
            .get(owner)
            .is_none_or(|proof| proof.status == CorrelationStatus::Unavailable)
    }

    pub fn diagnostics(&self) -> impl Iterator<Item = String> + '_ {
        self.owners
            .values()
            .filter(|proof| proof.status != CorrelationStatus::Verified)
            .map(|proof| {
                format!(
                    "Java lambda compiled/source correlation unavailable for {}: {}",
                    proof.owner, proof.reason
                )
            })
    }
}

#[derive(Default)]
pub(super) struct PendingCorrelations {
    bindings: Vec<RawBinding>,
    owners: Vec<Correlation>,
}

impl PendingCorrelations {
    pub fn binding(&mut self, json: &str) -> Result<()> {
        self.bindings.push(serde_json::from_str(json)?);
        Ok(())
    }

    pub fn correlation(&mut self, json: &str) -> Result<()> {
        self.owners.push(serde_json::from_str(json)?);
        Ok(())
    }

    pub fn validate(self, flows: &[SourceFlow], enabled: bool) -> Result<LambdaCorrelations> {
        ensure!(
            enabled || (self.bindings.is_empty() && self.owners.is_empty()),
            "Java lambda correlation requires kind_v8"
        );
        if !enabled {
            return Ok(LambdaCorrelations::default());
        }
        let sources = source_index(flows)?;
        let owners = owner_index(self.owners)?;
        let mut claimed = HashSet::new();
        let mut implementations = HashSet::new();
        let mut bindings = Vec::new();
        for raw in self.bindings {
            ensure!(
                implementations.insert(raw.implementation.clone()),
                "Duplicate Java lambda implementation correlation"
            );
            bindings.push(validate_binding(raw, &sources, &owners, &mut claimed)?);
        }
        validate_coverage(flows, &owners, &claimed, enabled)?;
        Ok(LambdaCorrelations { bindings, owners })
    }
}

fn source_index(flows: &[SourceFlow]) -> Result<Sources<'_>> {
    let mut sources = HashMap::new();
    let mut owner_files = HashMap::new();
    for (index, source) in flows
        .iter()
        .enumerate()
        .filter(|(_, flow)| flow.is_lambda())
    {
        ensure!(
            Path::new(&source.function.file).is_absolute(),
            "Java lambda source file must be absolute"
        );
        let old_file =
            owner_files.insert(source.lambda_owner.as_str(), source.function.file.as_str());
        ensure!(
            old_file.is_none_or(|file| file == source.function.file),
            "Java lambda owner {} crosses source files: {} and {}",
            source.lambda_owner,
            old_file.unwrap_or_default(),
            source.function.file
        );
        let prefix = format!("@lambda:{}:", source.lambda_owner);
        ensure!(
            source
                .function
                .symbol
                .strip_prefix(&prefix)
                .is_some_and(|offset| offset.parse::<u64>().is_ok()),
            "Invalid Java lambda source symbol"
        );
        ensure!(
            sources
                .insert(source.function.symbol.as_str(), (index, source))
                .is_none(),
            "Duplicate Java lambda source symbol"
        );
    }
    Ok(sources)
}

fn owner_index(records: Vec<Correlation>) -> Result<HashMap<String, Correlation>> {
    let mut owners = HashMap::new();
    for record in records {
        ensure!(
            !record.owner.is_empty(),
            "Missing Java lambda correlation owner"
        );
        ensure!(
            owners.insert(record.owner.clone(), record).is_none(),
            "Duplicate Java lambda correlation owner"
        );
    }
    Ok(owners)
}

fn implementation_owner(implementation: &str) -> Result<&str> {
    let (signature, returns) = implementation.rsplit_once('|').unwrap_or(("", ""));
    let (owner, method) = signature.split_once(':').unwrap_or(("", ""));
    ensure!(
        !owner.is_empty()
            && !returns.is_empty()
            && method.starts_with("lambda$")
            && method.contains('(')
            && method.ends_with(')'),
        "Invalid Java lambda implementation descriptor"
    );
    Ok(owner)
}

fn validate_binding(
    raw: RawBinding,
    sources: &Sources<'_>,
    owners: &HashMap<String, Correlation>,
    claimed: &mut HashSet<usize>,
) -> Result<LambdaBinding> {
    let owner = implementation_owner(&raw.implementation)?;
    ensure!(
        owners
            .get(owner)
            .is_some_and(|proof| proof.status == CorrelationStatus::Verified),
        "Java lambda binding lacks verified owner"
    );
    ensure!(
        !raw.sources.is_empty(),
        "Empty Java lambda correlation group"
    );
    let mut members = Vec::new();
    for symbol in raw.sources {
        let &(index, source) = sources
            .get(symbol.as_str())
            .ok_or_else(|| anyhow::anyhow!("Unknown Java lambda correlation source: {symbol}"))?;
        ensure!(
            source.lambda_owner == owner,
            "Java lambda correlation crosses type owners"
        );
        ensure!(
            claimed.insert(index),
            "Conflicting Java lambda source correlation"
        );
        members.push(index);
    }
    Ok(LambdaBinding {
        implementation: raw.implementation,
        sources: members,
    })
}

fn validate_coverage(
    flows: &[SourceFlow],
    owners: &HashMap<String, Correlation>,
    claimed: &HashSet<usize>,
    enabled: bool,
) -> Result<()> {
    for (index, source) in flows
        .iter()
        .enumerate()
        .filter(|(_, flow)| flow.is_lambda())
    {
        let proof = owners.get(&source.lambda_owner);
        ensure!(
            !enabled || proof.is_some(),
            "Missing Java lambda owner correlation status"
        );
        ensure!(
            !proof.is_some_and(|proof| proof.status == CorrelationStatus::Verified)
                || claimed.contains(&index),
            "Incomplete verified Java lambda correlation group"
        );
    }
    Ok(())
}
