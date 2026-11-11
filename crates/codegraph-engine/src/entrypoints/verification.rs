//! Human CLI formatting and root mapping for the core read-only verifier.
use super::arguments::Analysis;
use anyhow::{Result, ensure};

pub fn run(arguments: Analysis) -> Result<i32> {
    let project = arguments.project.canonicalize()?;
    ensure!(project.is_dir(), "Verification project must be a directory");
    let mut roots: Vec<_> = arguments
        .roots
        .into_iter()
        .map(|(name, path)| (name, project.join(path)))
        .collect();
    roots.push(("repo".into(), project.clone()));
    let report = codegraph_core::artifact::verification::verify(&project, &roots)?;
    println!("{}", serde_json::to_string(&report)?);
    Ok(report.exit_code())
}
