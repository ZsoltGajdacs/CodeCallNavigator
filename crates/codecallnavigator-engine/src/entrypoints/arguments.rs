use anyhow::{Context, Result, bail, ensure};
use std::path::PathBuf;

pub enum Command {
    Capabilities,
    Query,
    QuerySession,
    Observe {
        automatic: bool,
    },
    Help,
    Discover(PathBuf),
    ClearCache(PathBuf),
    Analyze(Analysis),
    Watch(Analysis),
    Verify(Analysis),
    Compare {
        project: PathBuf,
        base: String,
        head: String,
        base_snapshot: Option<String>,
        head_snapshot: Option<String>,
    },
    SnapshotStatus(Snapshot),
    PrepareSnapshot(Snapshot),
    Serve,
    ServeAnalysisSession,
    AnalyzeNative(Vec<PathBuf>),
}

pub struct Analysis {
    pub project: PathBuf,
    pub roots: Vec<(String, PathBuf)>,
    pub targets: Vec<PathBuf>,
}

pub struct Snapshot {
    pub project: PathBuf,
    pub revision: String,
    pub targets: Vec<PathBuf>,
}

pub fn parse(values: impl Iterator<Item = String>) -> Result<Command> {
    let mut values = values.peekable();
    let command = values.next().unwrap_or_else(|| "--help".into());
    match command.as_str() {
        "observe" => parse_observe(values),
        "query-session" => {
            ensure!(values.next().is_none(), "query-session takes no arguments");
            Ok(Command::QuerySession)
        }
        "capabilities" | "query" => {
            ensure!(values.next().is_none(), "Unexpected query command argument");
            Ok(if command == "capabilities" {
                Command::Capabilities
            } else {
                Command::Query
            })
        }
        "--help" | "-h" => {
            ensure!(values.next().is_none(), "Unexpected help argument");
            Ok(Command::Help)
        }
        "discover" => {
            let project = values.next().context("discover requires PROJECT")?;
            ensure!(values.next().is_none(), "discover accepts one PROJECT");
            Ok(Command::Discover(project.into()))
        }
        "analyze" => parse_analyze(values).map(Command::Analyze),
        "watch" => parse_analyze(values).map(Command::Watch),
        "verify" => {
            let arguments = parse_analyze(values)?;
            ensure!(
                arguments.targets.is_empty(),
                "verify accepts no analysis targets"
            );
            Ok(Command::Verify(arguments))
        }
        "clear-cache" => {
            ensure!(
                values.next().as_deref() == Some("--project"),
                "clear-cache requires --project PROJECT"
            );
            let project = take_value(&mut values, "--project")?;
            ensure!(values.next().is_none(), "Unexpected clear-cache argument");
            Ok(Command::ClearCache(project.into()))
        }
        "compare" => parse_compare(values),
        "snapshot-status" => parse_snapshot(values, false).map(Command::SnapshotStatus),
        "prepare-snapshot" => parse_snapshot(values, true).map(Command::PrepareSnapshot),
        "serve-once" => {
            ensure!(values.next().is_none(), "serve-once takes no arguments");
            Ok(Command::Serve)
        }
        "analysis-session" => {
            ensure!(
                values.next().is_none(),
                "analysis-session takes no arguments"
            );
            Ok(Command::ServeAnalysisSession)
        }
        "analyze-native" => {
            let paths: Vec<_> = values.map(PathBuf::from).collect();
            ensure!(!paths.is_empty(), "analyze-native requires TARGET...");
            Ok(Command::AnalyzeNative(paths))
        }
        other => bail!("Unknown engine command: {other}"),
    }
}

fn parse_observe(mut values: impl Iterator<Item = String>) -> Result<Command> {
    let automatic = match values.next().as_deref() {
        None => false,
        Some("--auto-refresh") => true,
        Some(_) => bail!("observe accepts only --auto-refresh"),
    };
    ensure!(values.next().is_none(), "Unexpected observe argument");
    Ok(Command::Observe { automatic })
}

fn take_value(values: &mut impl Iterator<Item = String>, option: &str) -> Result<String> {
    values
        .next()
        .with_context(|| format!("{option} requires a value"))
}

fn project_option(project: &mut Option<PathBuf>, value: String) -> Result<()> {
    ensure!(project.is_none(), "--project may only be specified once");
    *project = Some(value.into());
    Ok(())
}

fn parse_analyze(mut values: impl Iterator<Item = String>) -> Result<Analysis> {
    let (mut project, mut roots, mut targets) = (None, Vec::new(), Vec::new());
    let mut options = true;
    while let Some(value) = values.next() {
        match value.as_str() {
            "--" if options => options = false,
            "--project" if options => {
                project_option(&mut project, take_value(&mut values, &value)?)?
            }
            "--root" if options => {
                let root = take_value(&mut values, &value)?;
                let (name, path) = root.split_once('=').context("--root requires NAME=PATH")?;
                ensure!(
                    name != "repo",
                    "Logical root repo is reserved for --project"
                );
                roots.push((name.into(), PathBuf::from(path)));
            }
            _ if options && value.starts_with('-') => bail!("Unknown analyze option: {value}"),
            _ => targets.push(PathBuf::from(value)),
        }
    }
    Ok(Analysis {
        project: project.context("analyze requires --project PROJECT")?,
        roots,
        targets,
    })
}

fn parse_compare(mut values: impl Iterator<Item = String>) -> Result<Command> {
    let (mut project, mut base, mut head) = (None, None, None);
    let (mut base_snapshot, mut head_snapshot) = (None, None);
    while let Some(option) = values.next() {
        let value = take_value(&mut values, &option)?;
        match option.as_str() {
            "--project" => project_option(&mut project, value)?,
            "--base" => {
                ensure!(base.is_none(), "Repeated --base");
                base = Some(value);
            }
            "--head" => {
                ensure!(head.is_none(), "Repeated --head");
                head = Some(value);
            }
            "--base-snapshot" if base_snapshot.is_none() => base_snapshot = Some(value),
            "--head-snapshot" if head_snapshot.is_none() => head_snapshot = Some(value),
            _ => bail!("Unknown compare option: {option}"),
        }
    }
    Ok(Command::Compare {
        project: project.context("compare requires --project")?,
        base: base.context("compare requires --base")?,
        head: head.context("compare requires --head")?,
        base_snapshot,
        head_snapshot,
    })
}

fn parse_snapshot(mut values: impl Iterator<Item = String>, prepare: bool) -> Result<Snapshot> {
    let (mut project, mut revision, mut targets) = (None, None, Vec::new());
    while let Some(value) = values.next() {
        match value.as_str() {
            "--project" => project_option(&mut project, take_value(&mut values, &value)?)?,
            "--revision" if revision.is_none() => revision = Some(take_value(&mut values, &value)?),
            "--target" if prepare => targets.push(take_value(&mut values, &value)?.into()),
            _ => bail!("Unknown snapshot argument: {value}"),
        }
    }
    Ok(Snapshot {
        project: project.context("Snapshot command requires --project")?,
        revision: revision.context("Snapshot command requires --revision")?,
        targets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn command(values: &[&str]) -> Result<Command> {
        parse(values.iter().map(|value| value.to_string()))
    }
    #[test]
    fn clear_cache_requires_one_explicit_project() {
        assert!(command(&["clear-cache"]).is_err());
        assert!(command(&["clear-cache", "/repo"]).is_err());
        assert!(command(&["clear-cache", "--project"]).is_err());
        assert!(command(&["clear-cache", "--project", "/repo", "--all"]).is_err());
        assert!(matches!(
            command(&["clear-cache", "--project", "/repo"]).unwrap(),
            Command::ClearCache(project) if project == std::path::Path::new("/repo")
        ));
    }
    #[test]
    fn destinations_are_fixed_and_options_are_unambiguous() {
        assert!(command(&["analyze", "--project", "/repo", "--output", "other.json"]).is_err());
        assert!(command(&["analyze", "--project", "/repo", "--root", "repo=/other"]).is_err());
        assert!(command(&["analyze", "--project", "/repo", "--project", "/other"]).is_err());
        let Command::Analyze(parsed) =
            command(&["analyze", "--project", "/repo", "--", "--project"]).unwrap()
        else {
            panic!("analysis command")
        };
        assert_eq!(parsed.targets, vec![PathBuf::from("--project")]);
    }
}
