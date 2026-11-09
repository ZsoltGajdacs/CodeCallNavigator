//! Optional headless classification uses the same configuration and facts as Graph Studio.
use anyhow::{Context, Result, ensure};
use codegraph::{
    architecture::{self, Selection},
    model::GraphData,
};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Default)]
pub(super) struct Arguments {
    selection: Option<Selection>,
    root: Option<PathBuf>,
}
impl Arguments {
    pub fn option(&mut self, name: &str, value: String) -> Result<()> {
        match name {
            "--layers" => {
                ensure!(
                    self.selection.is_none(),
                    "--layers may only be specified once"
                );
                self.selection = Some(match value.as_str() {
                    "none" => Selection::None,
                    "spring" => Selection::Spring,
                    "react" => Selection::React,
                    "repository" => Selection::Repository,
                    _ => anyhow::bail!("--layers requires none, spring, react or repository"),
                });
            }
            "--layer-root" => {
                ensure!(
                    self.root.is_none(),
                    "--layer-root may only be specified once"
                );
                self.root = Some(value.into());
            }
            _ => unreachable!("known layer options only"),
        }
        Ok(())
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.root.is_none() || self.selection.is_some(),
            "--layer-root requires --layers"
        );
        Ok(())
    }
    pub fn report(&self, graph: &GraphData) -> Result<()> {
        let Some(selection) = self.selection else {
            return Ok(());
        };
        if selection == Selection::None {
            return Ok(());
        }
        let root = self
            .root
            .clone()
            .unwrap_or(std::env::current_dir()?)
            .canonicalize()
            .context("Cannot open layer configuration root")?;
        let loaded = architecture::load(&root, selection)?;
        let assignments = architecture::classify(graph, &loaded.config, loaded.origin)?;
        let mut counts = BTreeMap::new();
        for assignment in assignments.files.values() {
            *counts.entry(&assignment.layer).or_insert(0usize) += 1;
        }
        println!("Layers: {}", assignments.origin);
        for (layer, count) in counts {
            println!("  {layer}: {count} source files");
        }
        Ok(())
    }
}
