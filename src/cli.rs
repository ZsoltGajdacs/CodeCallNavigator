//! Headless entrypoint using the same instrumented pipeline as the native app.
mod layers;
use anyhow::{Context, Result, ensure};
use codegraph::{discovery, metrics, store::SqliteStore};
use std::{path::PathBuf, sync::atomic::AtomicBool};

struct Arguments {
    paths: Vec<PathBuf>,
    timings: Option<PathBuf>,
    layers: layers::Arguments,
    project: Option<PathBuf>,
}

fn arguments(values: impl Iterator<Item = String>) -> Result<Arguments> {
    let mut values = values.peekable();
    let mut result = Arguments {
        paths: Vec::new(),
        timings: None,
        layers: layers::Arguments::default(),
        project: None,
    };
    let mut options = true;
    while let Some(value) = values.next() {
        match value.as_str() {
            "--" if options => options = false,
            "--project" if options => {
                ensure!(
                    result.project.is_none(),
                    "--project may only be specified once"
                );
                result.project = Some(values.next().context("--project requires a path")?.into());
            }
            "--timings" if options => {
                ensure!(
                    result.timings.is_none(),
                    "--timings may only be specified once"
                );
                result.timings = Some(
                    values
                        .next()
                        .context("--timings requires a JSON output path")?
                        .into(),
                );
            }
            "--layers" | "--layer-root" if options => {
                let argument = values
                    .next()
                    .with_context(|| format!("{value} requires a value"))?;
                result.layers.option(&value, argument)?;
            }
            _ => result.paths.push(value.into()),
        }
    }
    ensure!(
        !result.paths.is_empty(),
        "Usage: codegraph analyze [--project ROOT] [--timings REPORT.json] [--layers PRESET --layer-root ROOT] SOLUTION_OR_PROJECT_OR_POM_OR_TSCONFIG_OR_BUILD_ROOT..."
    );
    result.layers.validate()?;
    if let Some(path) = &result.timings {
        validate_report_path(path)?;
    }
    Ok(result)
}

pub fn analyze() -> Result<()> {
    let arguments = arguments(std::env::args().skip(2))?;
    let metrics = metrics::Metrics::default();
    let _scope = metrics.enter();
    let result = run(arguments.paths, arguments.layers, arguments.project);
    metrics.finish();
    if let Some(path) = arguments.timings {
        validate_report_path(&path)?;
        std::fs::write(&path, serde_json::to_vec_pretty(&metrics.report())?)
            .with_context(|| format!("Cannot write timing report {}", path.display()))?;
    }
    result
}

fn validate_report_path(path: &std::path::Path) -> Result<()> {
    let resolved = if path.exists() {
        path.canonicalize()?
    } else {
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| std::path::Path::new("."));
        parent
            .canonicalize()?
            .join(path.file_name().context("Timing report must name a file")?)
    };
    ensure!(
        !discovery::generated_graph_output(&resolved),
        "Timing reports cannot overwrite reserved graph files"
    );
    Ok(())
}

fn run(paths: Vec<PathBuf>, layers: layers::Arguments, project: Option<PathBuf>) -> Result<()> {
    let paths = if project.is_some() {
        paths
    } else {
        paths
            .into_iter()
            .map(|path| path.canonicalize())
            .collect::<std::io::Result<Vec<_>>>()?
    };
    let project = project_root(&paths, project)?;
    let paths = paths
        .into_iter()
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                project.join(path)
            }
        })
        .collect();
    let runtime = crate::runtime::Runtime::discover()?;
    let targets = discover(paths, &runtime)?;
    let cancelled = AtomicBool::new(false);
    let graph = crate::engine_client::analyze(
        &runtime,
        crate::engine_client::Analysis { project, targets },
        &cancelled,
        &|message| eprintln!("{message}"),
    )?;
    layers.report(&graph)?;
    let store = SqliteStore::open(crate::jobs::data_directory())?;
    let generation = {
        let _timer = metrics::stage("publication");
        store.publish(&graph, &cancelled)?
    };
    println!(
        "Published {generation}: {} symbols, {} relationships",
        graph.nodes.len(),
        graph.edges.len()
    );
    for diagnostic in graph.diagnostics {
        eprintln!("Analysis note: {diagnostic}");
    }
    Ok(())
}

fn discover(
    paths: Vec<PathBuf>,
    runtime: &crate::runtime::Runtime,
) -> Result<Vec<discovery::BuildTarget>> {
    let _timer = metrics::stage("discovery");
    let mut targets = Vec::new();
    for path in paths {
        let path = path.canonicalize()?;
        if path.is_dir() {
            let found = crate::engine_client::discover(runtime, &path)?;
            let selected = discovery::default_selection(&found);
            targets.extend(
                found
                    .into_iter()
                    .filter(|target| selected.contains(&target.identity())),
            );
        } else {
            targets.push(discovery::target_from_file(&path)?);
        }
    }
    Ok(targets)
}

fn project_root(paths: &[PathBuf], explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(project) = explicit {
        let project = project.canonicalize()?;
        ensure!(project.is_dir(), "--project must name a directory");
        return Ok(project);
    }
    let mut roots = paths.iter().map(|path| -> Result<PathBuf> {
        let path = path.canonicalize()?;
        Ok(if path.is_dir() {
            path
        } else {
            path.parent()
                .context("Build path has no parent")?
                .to_owned()
        })
    });
    let mut common = roots.next().context("No analysis paths")??;
    for root in roots {
        let root = root?;
        while !root.starts_with(&common) {
            ensure!(common.pop(), "Analysis paths have no common project root");
        }
    }
    Ok(common)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timing_option_requires_a_unique_destination_and_preserves_paths() {
        let parse = |values: &[&str]| arguments(values.iter().map(|value| (*value).into()));
        let arguments = parse(&["--timings", "report.json", "a/pom.xml", "b"]).unwrap();
        assert_eq!(arguments.timings, Some("report.json".into()));
        assert_eq!(
            arguments.paths,
            vec![PathBuf::from("a/pom.xml"), PathBuf::from("b")]
        );
        assert!(parse(&["--timings"]).is_err());
        assert!(parse(&["a", "--timings", "one", "--timings", "two"]).is_err());
        assert_eq!(
            parse(&["--", "--timings"]).unwrap().paths,
            vec![PathBuf::from("--timings")]
        );
    }
    #[test]
    fn layer_options_validate_before_starting_analysis() {
        let parse = |v: &[&str]| arguments(v.iter().map(|s| s.to_string()));
        assert!(parse(&["--layers", "spring", "--layer-root", "/repo", "project"]).is_ok());
        assert!(parse(&["--layers", "unknown", "project"]).is_err());
        assert!(parse(&["--layer-root", "/repo", "project"]).is_err());
        assert!(parse(&["--layers", "spring", "--layers", "react", "project"]).is_err());
    }

    #[test]
    fn explicit_project_is_unique_and_common_owner_keeps_sibling_builds_together() {
        let parse = |values: &[&str]| arguments(values.iter().map(|value| (*value).into()));
        assert_eq!(
            parse(&["--project", "root", "app/tsconfig.json"])
                .unwrap()
                .project,
            Some("root".into())
        );
        assert!(parse(&["--project", "one", "--project", "two", "app"]).is_err());
        let root = tempfile::tempdir().unwrap();
        let project = root.path().canonicalize().unwrap();
        std::fs::create_dir(project.join("app")).unwrap();
        std::fs::create_dir(project.join("library")).unwrap();
        let config = project.join("app/tsconfig.json");
        std::fs::write(&config, "{}").unwrap();
        assert_eq!(
            project_root(&[config, project.join("library")], None).unwrap(),
            project
        );
    }

    #[test]
    fn timing_reports_cannot_overwrite_graph_files_through_direct_or_symlink_paths() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join(".codegraph");
        std::fs::create_dir(&directory).unwrap();
        let graph = directory.join("graph.json");
        std::fs::write(&graph, b"previous graph").unwrap();
        let alias = root.path().join("report.json");
        symlink(&graph, &alias).unwrap();
        let directory_alias = root.path().join("alias");
        symlink(&directory, &directory_alias).unwrap();
        for path in [
            &graph,
            &alias,
            &directory.join("graph.lock"),
            &directory.join(".graph-publish-test"),
            &directory_alias.join("graph.json"),
        ] {
            assert!(
                validate_report_path(path).is_err(),
                "Reserved path accepted: {}",
                path.display()
            );
        }
        assert!(validate_report_path(&root.path().join("timings.json")).is_ok());
        assert_eq!(std::fs::read(graph).unwrap(), b"previous graph");
    }
}
