//! Human CLI and one-request machine protocol share the same analysis operation.
mod analysis;
mod arguments;
mod cancellation;
mod inference;
mod linked_roots;
mod machine;
mod query;
mod query_session;
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
    if !matches!(command, Command::Query | Command::QuerySession) {
        cancellation::install();
    }
    match command {
        Command::Capabilities => {
            println!("{}", codecallnavigator_core::query::capabilities());
            Ok(0)
        }
        Command::Query => query::run(),
        Command::QuerySession => query_session::run(),
        Command::Help => {
            print_help();
            Ok(0)
        }
        Command::Discover(project) => {
            let targets = discovery::discover(&project)?;
            let selected = discovery::default_selection(&targets);
            println!(
                "{}",
                serde_json::to_string(
                    &serde_json::json!({"targets":targets,"default_selection":selected})
                )?
            );
            Ok(0)
        }
        Command::ClearCache(project) => {
            let removed = codecallnavigator_core::boundary::clear_inference_cache(&project)?;
            println!("Cleared inference cache ({removed} files removed)");
            Ok(0)
        }
        Command::Verify(arguments) => verification::run(arguments),
        Command::Analyze(arguments) => {
            let project = arguments.project.canonicalize()?;
            let targets = analysis::select_targets(&project, arguments.targets)?;
            let complete = analysis::analyze(&project, arguments.roots, targets, &|message| {
                eprintln!("{message}")
            })?;
            if let codecallnavigator_core::protocol::Event::Complete { digest, .. } = complete {
                println!(
                    "Published {} ({digest})",
                    project.join(artifact::GRAPH_PATH).display()
                );
            }
            Ok(0)
        }
        Command::Compare {
            project,
            base,
            head,
        } => {
            let result = artifact::compare_git(&project, &base, &head)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            Ok(i32::from(!result.equal))
        }
        Command::Serve => machine::serve_once(),
        // Hidden reference-runner hook; not an alternate persistent graph output.
        Command::AnalyzeNative(paths) => {
            let project = std::env::current_dir()?;
            let targets = analysis::select_targets(&project, paths)?;
            let graph = analysis::native(targets, &|message| eprintln!("{message}"))?;
            println!("{}", serde_json::to_string(&graph)?);
            Ok(0)
        }
    }
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
        "{program} capabilities\n{program} query < request.json\n{program} query-session < requests.ndjson\n{program} discover PROJECT\n{program} analyze --project PROJECT [--root NAME=PATH] TARGET...\n{program} clear-cache --project PROJECT\n{program} verify --project PROJECT [--root NAME=PATH]\n{program} compare --project PROJECT --base REF --head REF|WORKTREE"
    );
}
