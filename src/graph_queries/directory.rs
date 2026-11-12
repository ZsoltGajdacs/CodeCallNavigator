//! Live explorer data comes from the CLI, independently of the analyzed graph root.
use crate::{
    engine_client::QueryContext,
    explorer::loader::{Entry, Kind},
};
use anyhow::{Context, Result, ensure};
use codecallnavigator::query::Query;
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

pub enum DirectoryQueries {
    Cli(QueryContext),
    Fixture,
}

#[derive(Deserialize)]
struct Page {
    path: PathBuf,
    more: bool,
    entries: Vec<WireEntry>,
}
#[derive(Deserialize)]
struct WireEntry {
    path: String,
    label: String,
    kind: String,
}

impl DirectoryQueries {
    pub fn read(&self, path: &str, cancel: &AtomicBool) -> Result<(String, Vec<Entry>)> {
        let Self::Cli(context) = self else {
            let canonical = std::fs::canonicalize(path)?.to_string_lossy().into_owned();
            return Ok((
                canonical,
                crate::explorer::loader::read_fixture(path, cancel).map_err(anyhow::Error::msg)?,
            ));
        };
        let mut entries = Vec::new();

        loop {
            let page: Page = context.request(
                Query::Explorer {
                    path: PathBuf::from(path),
                    offset: entries.len(),
                    limit: 10000,
                },
                cancel,
            )?;
            ensure!(
                !page.more || !page.entries.is_empty(),
                "CLI explorer pagination made no progress"
            );
            let canonical = page.path.to_string_lossy().into_owned();
            for entry in page.entries {
                let kind = match entry.kind.as_str() {
                    "directory" => Kind::Directory,
                    "symlink" => Kind::Symlink,
                    "file" => Kind::File,
                    _ => anyhow::bail!("CLI returned an invalid explorer entry kind"),
                };
                let relative = Path::new(&entry.path)
                    .strip_prefix(&page.path)
                    .context("CLI explorer entry is outside its directory")?;
                entries.push(Entry {
                    path: Path::new(path)
                        .join(relative)
                        .to_string_lossy()
                        .into_owned(),
                    label: entry.label,
                    kind,
                });
            }
            if !page.more {
                return Ok((canonical, entries));
            }
        }
    }
}
