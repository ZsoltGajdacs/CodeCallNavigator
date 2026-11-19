//! Transactional references to immutable graph manifests.
use super::{SnapshotIdentity, SnapshotRecord};
use crate::artifact::storage::{Manifest, publication};
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OpenFlags, Transaction, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
enum Location {
    Commit {
        clone_key: String,
        record: SnapshotRecord,
    },
    Working {
        checkout: String,
        project: String,
    },
}

#[derive(Serialize, Deserialize)]
pub(super) struct Entry {
    location: Location,
    pub metadata: Value,
    manifest: String,
}

impl Entry {
    pub fn working(
        checkout: String,
        project: String,
        metadata: &Value,
        bytes: &[u8],
    ) -> Result<Self> {
        Self::new(Location::Working { checkout, project }, metadata, bytes)
    }

    pub fn snapshot(
        clone_key: String,
        identity: SnapshotIdentity,
        metadata: &Value,
        bytes: &[u8],
    ) -> Result<Self> {
        let digest = hex(bytes);
        let id = variant_id(&clone_key, &identity, &digest, metadata)?;
        Self::new(
            Location::Commit {
                clone_key,
                record: SnapshotRecord {
                    id,
                    identity,
                    digest,
                },
            },
            metadata,
            bytes,
        )
    }

    fn new(location: Location, metadata: &Value, bytes: &[u8]) -> Result<Self> {
        Manifest::from_bytes(bytes)?;
        Ok(Self {
            location,
            metadata: metadata.clone(),
            manifest: std::str::from_utf8(bytes)?.into(),
        })
    }

    pub fn snapshot_record(&self) -> Option<&SnapshotRecord> {
        match &self.location {
            Location::Commit { record, .. } => Some(record),
            Location::Working { .. } => None,
        }
    }

    pub fn bytes(&self) -> &[u8] {
        self.manifest.as_bytes()
    }

    pub fn id(&self) -> Result<String> {
        match &self.location {
            Location::Commit { clone_key, record } => {
                ensure!(digest(clone_key), "Invalid snapshot clone key");
                ensure!(
                    hex(self.bytes()) == record.digest
                        && variant_id(clone_key, &record.identity, &record.digest, &self.metadata)?
                            == record.id,
                    "Graph snapshot catalog entry identity mismatch"
                );
                Ok(record.id.clone())
            }
            Location::Working { checkout, project } => {
                ensure!(
                    digest(checkout) && digest(project),
                    "Invalid working graph location"
                );
                Ok(hex(&serde_json::to_vec(&(
                    checkout,
                    project,
                    hex(self.bytes()),
                    &self.metadata,
                ))?))
            }
        }
    }

    pub fn path(&self, id: &str) -> Result<PathBuf> {
        ensure!(self.id()? == id, "Graph catalog entry identity mismatch");
        Ok(match &self.location {
            Location::Commit { clone_key, record } => revision_path(clone_key, record)?,
            Location::Working { checkout, project } => Path::new("worktrees")
                .join(checkout)
                .join(project)
                .join(id)
                .join("graph.json"),
        })
    }

    pub fn validate_commit(
        &self,
        clone_key: &str,
        commit: &str,
        project: &str,
    ) -> Result<&SnapshotRecord> {
        let Location::Commit {
            clone_key: stored,
            record,
        } = &self.location
        else {
            anyhow::bail!("Expected a verified commit snapshot");
        };
        ensure!(
            (
                stored.as_str(),
                record.identity.commit.as_str(),
                record.identity.project.as_str()
            ) == (clone_key, commit, project),
            "Snapshot belongs to another clone, project, or commit"
        );
        self.id()?;
        Ok(record)
    }
}

pub(super) fn revision_path(clone_key: &str, record: &SnapshotRecord) -> Result<PathBuf> {
    ensure!(
        digest(clone_key) && digest(&record.id) && commit_id(&record.identity.commit),
        "Invalid snapshot location"
    );
    Ok(Path::new("repos")
        .join(clone_key)
        .join("commits")
        .join(&record.identity.commit)
        .join(hex(record.identity.project.as_bytes()))
        .join(&record.id)
        .join("graph.json"))
}

fn variant_id(
    clone_key: &str,
    identity: &SnapshotIdentity,
    digest: &str,
    metadata: &Value,
) -> Result<String> {
    ensure!(commit_id(&identity.commit), "Invalid graph snapshot commit");
    Ok(hex(&serde_json::to_vec(&(
        clone_key, identity, digest, metadata,
    ))?))
}

pub(super) fn hex(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

fn lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn digest(value: &str) -> bool {
    value.len() == 64 && lower_hex(value)
}

fn commit_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && lower_hex(value)
}

pub(super) fn open(root: &Path, writable: bool) -> Result<Connection> {
    let path = root.join("catalog.sqlite3");
    match fs::symlink_metadata(&path) {
        Ok(value) => ensure!(value.is_file(), "Graph catalog must be a regular file"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && writable => (),
        Err(error) => return Err(error.into()),
    }
    let flags = if writable {
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
    } else {
        // A reader may need SQLite to recover a hot rollback journal left
        // by an interrupted writer. Opening without CREATE keeps misses read-only.
        OpenFlags::SQLITE_OPEN_READ_WRITE
    };
    let mut db = Connection::open_with_flags(path, flags)?;
    let identifier: u32 = db.pragma_query_value(None, "application_id", |row| row.get(0))?;
    let version: u32 = db.pragma_query_value(None, "user_version", |row| row.get(0))?;
    const APPLICATION_ID: u32 = u32::from_be_bytes(*b"CCGL");
    if writable && (identifier, version) == (0, 0) {
        initialize(&mut db, APPLICATION_ID)?;
    } else {
        ensure!(
            (identifier, version) == (APPLICATION_ID, 1),
            "Unsupported graph library catalog"
        );
    }
    db.pragma_update(None, "foreign_keys", true)?;
    Ok(db)
}

fn initialize(db: &mut Connection, application: u32) -> Result<()> {
    let tx = db.transaction()?;
    let empty: bool = tx.query_row(
        "SELECT NOT EXISTS(SELECT 1 FROM sqlite_master)",
        [],
        |row| row.get(0),
    )?;
    ensure!(
        empty,
        "Refusing to use an existing database as a graph catalog"
    );
    tx.execute_batch(
        "
        CREATE TABLE bundles (
            id TEXT PRIMARY KEY, entry TEXT NOT NULL,
            clone TEXT, project TEXT, sha TEXT, retained INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX commit_variants ON bundles(clone, project, sha, retained);
        CREATE TABLE working (
            project TEXT PRIMARY KEY,
            current TEXT NOT NULL REFERENCES bundles(id),
            previous TEXT REFERENCES bundles(id),
            binding TEXT NOT NULL
        );
        CREATE INDEX working_current ON working(current);
        CREATE INDEX working_previous ON working(previous);
    ",
    )?;
    tx.pragma_update(None, "application_id", application)?;
    tx.pragma_update(None, "user_version", 1)?;
    tx.commit()?;
    Ok(())
}

pub(super) fn insert(tx: &Transaction<'_>, id: &str, entry: &Entry, retained: bool) -> Result<()> {
    let (clone, project, sha) = match &entry.location {
        Location::Commit { clone_key, record } => (
            Some(clone_key),
            Some(&record.identity.project),
            Some(&record.identity.commit),
        ),
        Location::Working { .. } => (None, None, None),
    };
    tx.execute(
        "INSERT INTO bundles (id, entry, clone, project, sha, retained) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(id) DO UPDATE SET entry=excluded.entry, clone=excluded.clone, project=excluded.project,
        sha=excluded.sha, retained=MAX(bundles.retained, excluded.retained)",
        params![id, serde_json::to_string(entry)?, clone, project, sha, retained],
    )?;
    Ok(())
}

struct WorkingReference {
    current: String,
    previous: Option<String>,
}

impl WorkingReference {
    fn load(db: &Connection, project: &str) -> Result<Option<Self>> {
        use rusqlite::OptionalExtension;
        let row = db
            .query_row(
                "SELECT current, previous, binding FROM working WHERE project=?1",
                [project],
                |row| {
                    Ok((
                        Self {
                            current: row.get(0)?,
                            previous: row.get(1)?,
                        },
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?;
        row.map(|(reference, binding)| {
            ensure!(
                reference.binding(project)? == binding,
                "Working graph catalog reference mismatch"
            );
            Ok(reference)
        })
        .transpose()
    }

    fn binding(&self, project: &str) -> Result<String> {
        Ok(hex(&serde_json::to_vec(&(
            project,
            &self.current,
            &self.previous,
        ))?))
    }
}

pub(super) fn set_current(tx: &Transaction<'_>, project: &str, id: &str) -> Result<()> {
    let previous = WorkingReference::load(tx, project)?.map(|reference| reference.current);
    if previous.as_deref() == Some(id) {
        return Ok(());
    }
    let reference = WorkingReference {
        current: id.into(),
        previous,
    };
    tx.execute(
        "INSERT INTO working (project, current, previous, binding) VALUES (?1, ?2, ?3, ?4)
        ON CONFLICT(project) DO UPDATE SET current=excluded.current,
            previous=excluded.previous, binding=excluded.binding",
        params![
            project,
            reference.current,
            reference.previous,
            reference.binding(project)?
        ],
    )?;
    Ok(())
}

fn read_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<(String, String)> {
    Ok((row.get(0)?, row.get(1)?))
}

fn checked((id, raw): (String, String)) -> Result<(String, Entry)> {
    let entry: Entry = serde_json::from_str(&raw).context("Invalid graph catalog entry")?;
    ensure!(entry.id()? == id, "Graph catalog entry identity mismatch");
    Ok((id, entry))
}

pub(super) fn working(db: &Connection, project: &str) -> Result<(String, Entry)> {
    let reference =
        WorkingReference::load(db, project)?.ok_or(crate::artifact::storage::MissingGraph)?;
    let raw = db
        .query_row(
            "SELECT id, entry FROM bundles WHERE id=?1",
            [&reference.current],
            read_entry,
        )
        .context("Missing bundle referenced by working graph")?;
    let (id, entry) = checked(raw)?;
    if let Location::Working {
        project: stored, ..
    } = &entry.location
    {
        ensure!(
            stored == project,
            "Graph belongs to another working project"
        );
    }
    Ok((id, entry))
}

pub(super) fn variants(
    db: &Connection,
    clone: &str,
    commit: &str,
    project: &str,
) -> Result<Vec<SnapshotRecord>> {
    let mut stmt = db.prepare("SELECT id, entry FROM bundles WHERE clone=?1 AND sha=?2 AND project=?3 AND retained=1 ORDER BY id")?;
    stmt.query_map(params![clone, commit, project], read_entry)?
        .map(|raw| {
            let (_, entry) = checked(raw?)?;
            Ok(entry.validate_commit(clone, commit, project)?.clone())
        })
        .collect()
}

pub(super) fn snapshot(db: &Connection, clone: &str, record: &SnapshotRecord) -> Result<Entry> {
    let raw = db
        .query_row(
            "SELECT id, entry FROM bundles WHERE id=?1 AND retained=1",
            [&record.id],
            read_entry,
        )
        .context("Cannot find retained graph snapshot")?;
    let (_, entry) = checked(raw)?;
    ensure!(
        entry.validate_commit(clone, &record.identity.commit, &record.identity.project)? == record,
        "Local graph snapshot identity changed"
    );
    Ok(entry)
}

struct Bundle {
    id: String,
    manifest: Manifest,
    pinned: bool,
    retained: bool,
}

impl Bundle {
    fn parts(&self) -> [(String, u64); 2] {
        [
            (
                self.manifest.structure.path("structure"),
                self.manifest.structure.compressed_bytes,
            ),
            (
                self.manifest.flows.path("flows"),
                self.manifest.flows.compressed_bytes,
            ),
        ]
    }
}

fn load_inventory(tx: &Transaction<'_>) -> Result<Vec<Bundle>> {
    let mut stmt = tx.prepare(
        "SELECT id, entry, retained, EXISTS(
        SELECT 1 FROM working WHERE current=bundles.id OR previous=bundles.id)
        FROM bundles ORDER BY rowid",
    )?;
    let rows = stmt.query_map([], |row| Ok((read_entry(row)?, row.get(2)?, row.get(3)?)))?;
    rows.map(|row| {
        let (raw, retained, pinned) = row?;
        let (id, entry) = checked(raw)?;
        Ok(Bundle {
            id,
            manifest: Manifest::from_bytes(entry.bytes())?,
            pinned,
            retained,
        })
    })
    .collect()
}

#[derive(Default)]
struct HistoryObjects {
    references: HashMap<String, usize>,
    bytes: u64,
}

impl HistoryObjects {
    fn count(bundles: &[&Bundle], pinned: &HashSet<String>) -> Result<Self> {
        let mut usage = Self::default();
        for (path, size) in bundles
            .iter()
            .flat_map(|bundle| bundle.parts())
            .filter(|(path, _)| !pinned.contains(path))
        {
            let count = usage.references.entry(path).or_default();
            if *count == 0 {
                usage.bytes = usage
                    .bytes
                    .checked_add(size)
                    .context("Graph library size overflow")?;
            }
            *count += 1;
        }
        Ok(usage)
    }

    fn remove(&mut self, path: &str, size: u64) {
        if let Some(count) = self.references.get_mut(path) {
            *count -= 1;
            if *count == 0 {
                self.bytes -= size;
            }
        }
    }
}

pub(super) fn evict(tx: &Transaction<'_>, newest: &str, budget: u64) -> Result<()> {
    let bundles = load_inventory(tx)?;
    let pinned = bundles
        .iter()
        .filter(|bundle| bundle.pinned)
        .flat_map(Bundle::parts)
        .map(|(path, _)| path)
        .collect();
    let history: Vec<_> = bundles
        .iter()
        .filter(|bundle| bundle.retained && !bundle.pinned)
        .collect();
    let mut usage = HistoryObjects::count(&history, &pinned)?;
    for bundle in history.into_iter().filter(|bundle| bundle.id != newest) {
        if usage.bytes <= budget {
            break;
        }
        tx.execute("UPDATE bundles SET retained=0 WHERE id=?1", [&bundle.id])?;
        for (path, size) in bundle.parts() {
            usage.remove(&path, size);
        }
    }
    ensure!(
        usage.bytes <= budget,
        "Graph snapshot exceeds the library size limit"
    );
    tx.execute(
        "DELETE FROM bundles WHERE retained=0 AND NOT EXISTS (
        SELECT 1 FROM working WHERE current=bundles.id OR previous=bundles.id)",
        [],
    )?;
    Ok(())
}

pub(super) fn prune(root: &Path, db: &Connection) -> Result<()> {
    let mut stmt = db.prepare("SELECT id, entry FROM bundles")?;
    let mut live = HashSet::new();
    for raw in stmt.query_map([], read_entry)? {
        let (id, entry) = checked(raw?)?;
        let manifest = Manifest::from_bytes(entry.bytes())?;
        live.insert(root.join(entry.path(&id)?));
        live.insert(root.join(manifest.structure.path("structure")));
        live.insert(root.join(manifest.flows.path("flows")));
    }
    for item in fs::read_dir(root.join("objects"))? {
        let item = item?;
        if item
            .file_name()
            .to_str()
            .is_some_and(publication::owned_object)
        {
            prune_file(&item.path(), &live)?;
        }
    }
    use Level::{Commits, Digest, Sha};
    prune_manifests(&root.join("worktrees"), &[Digest, Digest, Digest], &live)?;
    prune_manifests(
        &root.join("repos"),
        &[Digest, Commits, Sha, Digest, Digest],
        &live,
    )
}

enum Level {
    Digest,
    Commits,
    Sha,
}

impl Level {
    fn owns(&self, name: &str) -> bool {
        match self {
            Self::Digest => digest(name),
            Self::Commits => name == "commits",
            Self::Sha => commit_id(name),
        }
    }
}

fn library_directory(path: &Path) -> Result<Option<fs::ReadDir>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(Some(fs::read_dir(path)?)),
        Ok(_) => Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn prune_file(path: &Path, live: &HashSet<PathBuf>) -> Result<bool> {
    if live.contains(path) || !fs::symlink_metadata(path).is_ok_and(|entry| entry.is_file()) {
        return Ok(false);
    }
    fs::remove_file(path)?;
    Ok(true)
}

fn prune_manifests(folder: &Path, levels: &[Level], live: &HashSet<PathBuf>) -> Result<()> {
    let Some(children) = library_directory(folder)? else {
        return Ok(());
    };
    let Some((level, rest)) = levels.split_first() else {
        if prune_file(&folder.join("graph.json"), live)? {
            let _ = fs::remove_dir(folder);
        }
        return Ok(());
    };
    for child in children {
        let child = child?;
        if child
            .file_name()
            .to_str()
            .is_some_and(|name| level.owns(name))
        {
            prune_manifests(&child.path(), rest, live)?;
        }
    }
    Ok(())
}
