//! Human CLI and one-request machine protocol share the same analysis operation.
mod analysis;
mod arguments;
mod cancellation;
mod linked_roots;
mod machine;
use anyhow::Result;
use arguments::Command;
use codegraph_core::{artifact, discovery};

pub fn run() -> Result<i32> {
    cancellation::install();
    match arguments::parse(std::env::args().skip(1))? {
        Command::Help => {
            println!(
                "codegraph-engine discover PROJECT\ncodegraph-engine analyze --project PROJECT [--root NAME=PATH] TARGET...\ncodegraph-engine compare --project PROJECT --base REF --head REF|WORKTREE"
            );
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
        Command::Analyze(arguments) => {
            let project = arguments.project.canonicalize()?;
            let targets = analysis::select_targets(&project, arguments.targets)?;
            let complete = analysis::analyze(&project, arguments.roots, targets, &|message| {
                eprintln!("{message}")
            })?;
            if let codegraph_core::protocol::Event::Complete { digest, .. } = complete {
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
