//! Configured ownership has one authority; dependency contexts only fill gaps.
use super::{SourceOwner, SourceOwners};
use anyhow::{Result, ensure};
use std::{
    collections::{HashSet, hash_map::Entry},
    path::PathBuf,
};

#[derive(Clone, Copy)]
pub(super) enum Origin {
    ConfiguredNormal,
    ContextFallback,
}

pub(super) struct Claim {
    pub file: PathBuf,
    pub assembly: String,
    pub owner: SourceOwner,
}

#[derive(Hash, PartialEq, Eq)]
struct Identity {
    file: PathBuf,
    assembly: String,
}

impl Claim {
    fn identity(&self) -> Identity {
        Identity {
            file: self.file.clone(),
            assembly: self.assembly.clone(),
        }
    }
}

#[derive(Default)]
pub(super) struct Owners {
    configured: HashSet<Identity>,
    resolved: SourceOwners,
}

impl Owners {
    pub fn resolved(&self) -> &SourceOwners {
        &self.resolved
    }

    pub fn record(&mut self, claim: Claim, origin: Origin) -> Result<()> {
        match origin {
            Origin::ConfiguredNormal => self.configured(claim),
            Origin::ContextFallback => self.fallback(claim),
        }
    }

    fn configured(&mut self, claim: Claim) -> Result<()> {
        if self.configured.insert(claim.identity()) {
            self.resolved
                .entry(claim.file)
                .or_default()
                .insert(claim.assembly, claim.owner);
            return Ok(());
        }
        self.compatible(claim)
    }

    fn fallback(&mut self, claim: Claim) -> Result<()> {
        if self.configured.contains(&claim.identity()) {
            return Ok(());
        }
        self.compatible(claim)
    }

    fn compatible(&mut self, claim: Claim) -> Result<()> {
        let file = claim.file.display().to_string();
        match self
            .resolved
            .entry(claim.file)
            .or_default()
            .entry(claim.assembly)
        {
            Entry::Vacant(entry) => {
                entry.insert(claim.owner);
            }
            Entry::Occupied(entry) => {
                ensure!(
                    entry.get() == &claim.owner,
                    "C# evaluation contexts disagree on source ownership for {file} in assembly {}; the graph cannot represent conflicting fallback classifications",
                    entry.key()
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim(is_test: bool) -> Claim {
        Claim {
            file: "/source.cs".into(),
            assembly: "Application".into(),
            owner: SourceOwner {
                project: "Application".into(),
                is_test,
                is_generated: false,
            },
        }
    }

    #[test]
    fn normal_authority_wins_even_if_it_arrives_after_a_fallback() {
        let mut owners = Owners::default();
        owners.record(claim(true), Origin::ContextFallback).unwrap();
        owners
            .record(claim(false), Origin::ConfiguredNormal)
            .unwrap();
        owners.record(claim(true), Origin::ContextFallback).unwrap();
        assert!(!owners.resolved()[std::path::Path::new("/source.cs")]["Application"].is_test);
    }

    #[test]
    fn incompatible_fallback_only_classifications_fail_instead_of_last_write_winning() {
        let mut owners = Owners::default();
        owners.record(claim(true), Origin::ContextFallback).unwrap();
        assert!(
            owners
                .record(claim(false), Origin::ContextFallback)
                .is_err()
        );
    }
}
