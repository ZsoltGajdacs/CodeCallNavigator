//! Disposable decisions; callers commit only after publishing a successful graph.
use super::{RouteFact, cache_key, cache_store};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub rejected: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Decision {
    Winner { route: String },
    Unresolved,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    decision: Decision,
    checksum: String,
}

pub struct DecisionCache {
    path: PathBuf,
    implementation: String,
    previous: BTreeMap<String, String>,
    current: BTreeMap<String, String>,
    stats: CacheStats,
    warnings: Vec<String>,
}
impl DecisionCache {
    /// Read only. Missing/corrupt/version-mismatched caches cannot affect analysis.
    pub fn open(project: &Path) -> Self {
        let project = project
            .canonicalize()
            .unwrap_or_else(|_| project.to_path_buf());
        let path = project.join(".codecallnavigator/inference.sqlite3");
        let mut session = Self {
            path,
            implementation: cache_key::implementation(),
            previous: BTreeMap::new(),
            current: BTreeMap::new(),
            stats: CacheStats::default(),
            warnings: Vec::new(),
        };
        match super::cache_paths::validate(&session.path) {
            Ok(()) if session.path.exists() => session.load(),
            Ok(()) => {}
            Err(error) => session.reject(format!("Inference cache ignored: {error:#}")),
        }
        session
    }
    fn load(&mut self) {
        match cache_store::read(&self.path, &self.implementation) {
            Ok(rows) => self.previous = rows,
            Err(error) => self.reject(format!("Inference cache ignored: {error:#}")),
        }
    }
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
    /// Call only under the project analysis lock, after successful graph publication.
    /// Cache failure is a volatile warning, never a failed graph publication.
    pub fn commit(&mut self) {
        if let Err(error) = cache_store::write(&self.path, &self.implementation, &self.current) {
            self.warnings
                .push(format!("Inference cache was not saved: {error:#}"));
        }
    }
    pub(super) fn implementation(&self) -> &str {
        &self.implementation
    }
    pub(super) fn lookup<'a>(
        &mut self,
        key: &str,
        candidates: &[&'a RouteFact],
    ) -> Option<Option<&'a RouteFact>> {
        let value = self.previous.get(key)?;
        match decode(key, value, candidates) {
            Ok(winner) => {
                self.stats.hits += 1;
                self.current.insert(key.into(), value.clone());
                Some(winner)
            }
            Err(error) => {
                self.reject(format!("Inference cache decision ignored: {error:#}"));
                None
            }
        }
    }
    pub(super) fn record(&mut self, key: String, winner: Option<&RouteFact>) -> Result<()> {
        self.stats.misses += 1;
        let decision = match winner {
            Some(route) => Decision::Winner {
                route: cache_key::digest(route)?,
            },
            None => Decision::Unresolved,
        };
        let checksum = checksum(&key, &decision)?;
        self.current
            .insert(key, serde_json::to_string(&Record { decision, checksum })?);
        Ok(())
    }
    fn reject(&mut self, warning: String) {
        self.stats.rejected += 1;
        self.warnings.push(warning);
    }
}
fn checksum(key: &str, decision: &Decision) -> Result<String> {
    cache_key::digest(&(key, decision))
}
fn decode<'a>(
    key: &str,
    value: &str,
    candidates: &[&'a RouteFact],
) -> Result<Option<&'a RouteFact>> {
    let record: Record = serde_json::from_str(value)?;
    ensure!(
        record.checksum == checksum(key, &record.decision)?,
        "Inference decision checksum differs"
    );
    match record.decision {
        Decision::Unresolved => Ok(None),
        Decision::Winner { route } => {
            let winner = candidates
                .iter()
                .find(|candidate| cache_key::digest(*candidate).is_ok_and(|digest| digest == route))
                .copied();
            let winner = winner
                .ok_or_else(|| anyhow::anyhow!("Cached target is absent from fresh route facts"))?;
            ensure!(
                super::resolve::is_winner(winner, candidates),
                "Cached target cannot win current route facts"
            );
            Ok(Some(winner))
        }
    }
}
