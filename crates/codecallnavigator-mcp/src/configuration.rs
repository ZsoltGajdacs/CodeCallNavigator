use anyhow::{Result, bail, ensure};
use std::{collections::BTreeSet, path::PathBuf};

#[derive(Clone, Debug)]
pub struct Configuration {
    pub engine: PathBuf,
    pub project: PathBuf,
    pub roots: Vec<(String, PathBuf)>,
}

impl Configuration {
    pub fn parse(mut arguments: impl Iterator<Item = String>) -> Result<Self> {
        let mut engine = None;
        let mut project = None;
        let mut roots = Vec::new();
        let mut names = BTreeSet::new();
        while let Some(option) = arguments.next() {
            let value = arguments
                .next()
                .ok_or_else(|| anyhow::anyhow!("{option} requires a value"))?;
            ensure!(
                !value.trim().is_empty(),
                "{option} requires a nonempty value"
            );
            match option.as_str() {
                "--engine" => unique(&mut engine, value, &option)?,
                "--project" => unique(&mut project, value, &option)?,
                "--root" => {
                    let (name, path) = value
                        .split_once('=')
                        .ok_or_else(|| anyhow::anyhow!("--root requires NAME=PATH"))?;
                    ensure!(valid_root(name) && !path.is_empty(), "Invalid linked root");
                    ensure!(
                        names.insert(name.to_owned()),
                        "Duplicate linked root: {name}"
                    );
                    roots.push((name.to_owned(), path.into()));
                }
                _ => bail!("Unknown option: {option}"),
            }
        }
        Ok(Self {
            engine: engine.ok_or_else(|| anyhow::anyhow!("--engine is required"))?,
            project: project.ok_or_else(|| anyhow::anyhow!("--project is required"))?,
            roots,
        })
    }
}

fn unique(slot: &mut Option<PathBuf>, value: String, option: &str) -> Result<()> {
    ensure!(slot.is_none(), "{option} may only be specified once");
    *slot = Some(value.into());
    Ok(())
}
fn valid_root(name: &str) -> bool {
    !name.is_empty()
        && name != "repo"
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(args: &[&str]) -> Result<Configuration> {
        Configuration::parse(args.iter().map(|s| (*s).into()))
    }
    #[test]
    fn accepts_nonexistent_fixed_paths_without_reading_them() {
        let config = parse(&[
            "--engine",
            "/missing/cli",
            "--project",
            "/missing/project",
            "--root",
            "lib=/missing/lib",
        ])
        .unwrap();
        assert_eq!(config.roots[0].0, "lib");
        assert_eq!(config.project, PathBuf::from("/missing/project"));
    }
    #[test]
    fn rejects_ambiguous_or_reserved_configuration() {
        for args in [
            vec![],
            vec!["--engine", "cli"],
            vec![
                "--engine",
                "cli",
                "--project",
                "root",
                "--root",
                "repo=other",
            ],
            vec![
                "--engine",
                "cli",
                "--project",
                "root",
                "--root",
                "lib=a",
                "--root",
                "lib=b",
            ],
            vec!["--engine", "cli", "--engine", "other", "--project", "root"],
        ] {
            assert!(parse(&args).is_err());
        }
    }
}
