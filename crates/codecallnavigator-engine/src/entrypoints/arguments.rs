use anyhow::{Context, Result, bail, ensure};
use std::path::PathBuf;

pub enum Command {
    Capabilities,
    Query,
    QuerySession,
    Help,
    Discover(PathBuf),
    ClearCache(PathBuf),
    Analyze(Analysis),
    Verify(Analysis),
    Compare {
        project: PathBuf,
        base: String,
        head: String,
    },
    Serve,
    AnalyzeNative(Vec<PathBuf>),
}

pub struct Analysis {
    pub project: PathBuf,
    pub roots: Vec<(String, PathBuf)>,
    pub targets: Vec<PathBuf>,
}

pub fn parse(values: impl Iterator<Item = String>) -> Result<Command> {
    let mut values = values.peekable();
    let command = values.next().unwrap_or_else(|| "--help".into());
    match command.as_str() {
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
        "analyze" => parse_analyze(values),
        "verify" => {
            let Command::Analyze(arguments) = parse_analyze(values)? else {
                unreachable!()
            };
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
        "serve-once" => {
            ensure!(values.next().is_none(), "serve-once takes no arguments");
            Ok(Command::Serve)
        }
        "analyze-native" => {
            let paths: Vec<_> = values.map(PathBuf::from).collect();
            ensure!(!paths.is_empty(), "analyze-native requires TARGET...");
            Ok(Command::AnalyzeNative(paths))
        }
        other => bail!("Unknown engine command: {other}"),
    }
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

fn parse_analyze(mut values: impl Iterator<Item = String>) -> Result<Command> {
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
    Ok(Command::Analyze(Analysis {
        project: project.context("analyze requires --project PROJECT")?,
        roots,
        targets,
    }))
}

fn parse_compare(mut values: impl Iterator<Item = String>) -> Result<Command> {
    let (mut project, mut base, mut head) = (None, None, None);
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
            _ => bail!("Unknown compare option: {option}"),
        }
    }
    Ok(Command::Compare {
        project: project.context("compare requires --project")?,
        base: base.context("compare requires --base")?,
        head: head.context("compare requires --head")?,
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
