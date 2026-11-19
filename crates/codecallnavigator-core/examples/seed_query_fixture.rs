//! Install the frozen query benchmark input into an isolated working library.
use anyhow::{Context, Result, ensure};
use codecallnavigator_core::artifact::{self, storage::Manifest};
use std::{fs, path::PathBuf, sync::atomic::AtomicBool};

fn main() -> Result<()> {
    let arguments: Vec<_> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let [project, manifest] = arguments.as_slice() else {
        anyhow::bail!("Expected a benchmark project and portable fixture manifest");
    };
    let library = std::env::var_os("CODECALLNAVIGATOR_GRAPH_LIBRARY")
        .context("The benchmark requires an isolated graph library")?;
    ensure!(
        PathBuf::from(library).starts_with(std::env::temp_dir()),
        "The benchmark library must be inside the run's temporary directory"
    );
    let directory = manifest
        .parent()
        .context("Missing fixture bundle directory")?;
    let graph = Manifest::from_bytes(&fs::read(manifest)?)?
        .complete_with(|name| Ok(fs::read(directory.join(name))?))?;
    artifact::publish(project, &graph, &AtomicBool::new(false))
}
