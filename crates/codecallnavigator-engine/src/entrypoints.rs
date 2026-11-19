//! Human CLI and one-request machine protocol share the same analysis operation.
mod analysis;
mod analysis_session;
mod arguments;
mod cancellation;
mod comparison;
mod flow_comparison;
mod flow_expansion;
mod impact_preview;
mod inference;
mod linked_roots;
mod machine;
mod observation_stream;
mod observation_transport;
mod observer;
mod query;
mod query_executor;
mod query_session;
mod refresh_child;
mod refresh_coordinator;
mod refresh_policy;
mod response_encoding;
mod session_child;
mod session_inputs;
mod session_notice;
mod session_stream;
mod snapshot_retention;
mod snapshot_worktree;
mod snapshots;
mod verification;
use anyhow::Result;
use arguments::Command;
use codecallnavigator_core::{artifact, discovery};

pub fn main() {
    match run() {
        Ok(status) => std::process::exit(status),
        Err(error) => {
            eprintln!("{error:#}");
            std::process::exit(2);
        }
    }
}

pub fn run() -> Result<i32> {
    let command = arguments::parse(std::env::args().skip(1))?;
    // Read-only queries own no build subprocesses. Keep default signal handling
    // so an idle session blocked on stdin also terminates promptly on cancellation.
    if !matches!(
        command,
        Command::Query
            | Command::QuerySession
            | Command::Observe { automatic: false }
            | Command::GraphPath(_)
    ) {
        cancellation::install();
    }
    match command {
        Command::Capabilities => {
            println!("{}", codecallnavigator_core::query::capabilities());
            Ok(0)
        }
        Command::Query => query::run(),
        Command::QuerySession => query_session::run(),
        Command::Observe { automatic } => observation_stream::run(automatic),
        Command::Help => {
            print_help();
            Ok(0)
        }
        Command::Discover(project) => run_discovery(&project),
        Command::GraphPath(project) => {
            println!("{}", artifact::working_manifest_path(&project)?.display());
            Ok(0)
        }
        Command::ClearCache(project) => {
            let removed = codecallnavigator_core::boundary::clear_inference_cache(&project)?;
            println!("Cleared inference cache ({removed} files removed)");
            Ok(0)
        }
        Command::Verify(arguments) => verification::run(arguments),
        Command::Analyze(arguments) => run_analysis(arguments),
        Command::Watch(arguments) => session_stream::cli(arguments),
        Command::SnapshotStatus(arguments) => {
            print_snapshot(&snapshots::status(&arguments.project, &arguments.revision)?)
        }
        Command::PrepareSnapshot(arguments) => print_snapshot(&snapshots::prepare(
            &arguments.project,
            &arguments.revision,
            arguments.targets,
            &|message| eprintln!("{message}"),
        )?),
        Command::ImpactPreview { project, query } => impact_preview::cli(project, query),
        Command::Compare {
            project,
            base,
            head,
            base_snapshot,
            head_snapshot,
        } => {
            let base =
                artifact::revision::inspect_cached(&project, &base, base_snapshot.as_deref())?
                    .into_snapshot()?;
            let head =
                artifact::revision::inspect_cached(&project, &head, head_snapshot.as_deref())?
                    .into_snapshot()?;
            let result = artifact::compare(&base.artifact, &head.artifact)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            Ok(i32::from(!result.equal))
        }
        Command::Serve => machine::serve_once(),
        Command::ServeAnalysisSession => session_stream::serve(),
        // Hidden reference-runner hook; not an alternate persistent graph output.
        Command::AnalyzeNative(paths) => run_reference_analysis(paths),
    }
}

fn run_reference_analysis(paths: Vec<std::path::PathBuf>) -> Result<i32> {
    let project = std::env::current_dir()?;
    let targets = analysis::select_targets(&project, paths)?;
    let graph = analysis::native(targets, &|message| eprintln!("{message}"))?;
    println!("{}", serde_json::to_string(&graph)?);
    Ok(0)
}

fn print_snapshot(snapshot: &impl serde::Serialize) -> Result<i32> {
    println!("{}", serde_json::to_string_pretty(snapshot)?);
    Ok(0)
}

fn run_discovery(project: &std::path::Path) -> Result<i32> {
    let targets = discovery::discover(project)?;
    let selected = discovery::default_selection(&targets);
    println!(
        "{}",
        serde_json::to_string(
            &serde_json::json!({"targets":targets,"default_selection":selected})
        )?
    );
    Ok(0)
}

fn run_analysis(arguments: arguments::Analysis) -> Result<i32> {
    let project = arguments.project.canonicalize()?;
    let targets = analysis::select_targets(&project, arguments.targets)?;
    let complete = analysis::analyze(&project, arguments.roots, targets, &|message| {
        eprintln!("{message}")
    })?;
    if let codecallnavigator_core::protocol::Event::Complete { digest, .. } = complete {
        println!("Published working graph ({digest})");
    }
    Ok(0)
}

fn print_help() {
    let program = if std::env::args().next().is_some_and(|path| {
        std::path::Path::new(&path)
            .file_stem()
            .is_some_and(|name| name == "ccnavigator")
    }) {
        "ccnavigator"
    } else {
        "codecallnavigator-engine"
    };
    println!(
        "{program} capabilities\n{program} query < request.json\n{program} query-session < requests.ndjson\n{program} observe [--auto-refresh] < observation-request.ndjson\n{program} discover PROJECT\n{program} analyze --project PROJECT [--root NAME=PATH] TARGET...\n{program} watch --project PROJECT [--root NAME=PATH] TARGET...\n{program} clear-cache --project PROJECT\n{program} graph-path --project PROJECT\n{program} verify --project PROJECT [--root NAME=PATH]\n{program} compare --project PROJECT --base REF --head REF|WORKTREE [--base-snapshot ID] [--head-snapshot ID]\n{program} snapshot-status --project PROJECT --revision REF\n{program} prepare-snapshot --project PROJECT --revision REF [--target PATH]...\n{program} impact-preview --project PROJECT --base REF [--head REF|WORKTREE] [--base-snapshot ID]"
    );
}
