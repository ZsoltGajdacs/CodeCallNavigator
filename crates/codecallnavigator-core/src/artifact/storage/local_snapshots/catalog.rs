use super::{Manifest, SnapshotIdentity, SnapshotRecord, open_object, real_directory};
use crate::artifact::{EncodedArtifact, storage::publication::owned_object};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::{self, DirEntry, ReadDir},
    io::Read,
    path::{Path, PathBuf},
};

const ENTRY_LIMIT: u64 = 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Entry {
    pub record: SnapshotRecord,
    pub manifest: String,
    pub inserted: u64,
}

impl Entry {
    pub fn new(identity: SnapshotIdentity, encoded: &EncodedArtifact) -> Result<Self> {
        let bytes = encoded.as_ref();
        let digest = blake3::hash(bytes).to_hex().to_string();
        let id = identity_digest(&identity, &digest)?;
        Ok(Self {
            record: SnapshotRecord {
                id,
                identity,
                digest,
            },
            manifest: std::str::from_utf8(bytes)?.into(),
            inserted: 0,
        })
    }

    pub fn read(path: &Path) -> Result<Self> {
        let (entry, _) = Self::read_measured(path)?;
        Ok(entry)
    }

    fn read_measured(path: &Path) -> Result<(Self, u64)> {
        let mut bytes = Vec::new();
        open_object(path)?
            .take(ENTRY_LIMIT + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= ENTRY_LIMIT,
            "Graph snapshot catalog entry is too large"
        );
        let entry: Self = serde_json::from_slice(&bytes)
            .with_context(|| format!("Invalid graph snapshot entry {}", path.display()))?;
        entry.manifest()?;
        ensure!(
            identity_digest(&entry.record.identity, &entry.record.digest)? == entry.record.id
                && id_from_path(path) == Some(&entry.record.id),
            "Graph snapshot catalog entry identity mismatch"
        );
        Ok((entry, bytes.len() as u64))
    }

    pub fn manifest(&self) -> Result<Manifest> {
        let bytes = self.manifest.as_bytes();
        ensure!(
            blake3::hash(bytes).to_hex().as_str() == self.record.digest,
            "Graph snapshot manifest digest mismatch"
        );
        Manifest::from_bytes(bytes)
    }
}

fn identity_digest(identity: &SnapshotIdentity, digest: &str) -> Result<String> {
    ensure!(
        matches!(identity.commit.len(), 40 | 64) && lower_hex(&identity.commit),
        "Invalid graph snapshot commit"
    );
    let bytes = serde_json::to_vec(&(identity, digest))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

pub(super) fn digest(value: &str) -> bool {
    value.len() == 64 && lower_hex(value)
}

fn lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn id_from_path(path: &Path) -> Option<&str> {
    let id = path.file_name()?.to_str()?.strip_suffix(".json")?;
    digest(id).then_some(id)
}

pub(super) fn entry_path(repository: &Path, id: &str) -> PathBuf {
    repository.join(format!("{id}.json"))
}

pub(super) fn read_directory(path: &Path) -> Result<Option<ReadDir>> {
    match real_directory(path) {
        Ok(()) => Ok(Some(fs::read_dir(path)?)),
        Err(error) if missing(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(super) fn missing(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<std::io::Error>()
        .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound)
}

pub(super) fn entries(repository: &Path) -> Result<Vec<(PathBuf, Entry)>> {
    entry_paths(repository)?
        .into_iter()
        .map(|path| Entry::read(&path).map(|entry| (path, entry)))
        .collect()
}

fn entry_paths(repository: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in read_directory(repository)?.into_iter().flatten() {
        let path = entry?.path();
        if id_from_path(&path).is_some() {
            paths.push(path);
        }
    }
    Ok(paths)
}

pub(super) struct Catalog {
    entries: BTreeMap<PathBuf, InventoryItem>,
}

pub(super) struct InventoryItem {
    pub entry: Entry,
    pub bytes: u64,
}

impl Catalog {
    pub fn load(root: &Path) -> Result<Self> {
        let mut entries = BTreeMap::new();
        for repository in fs::read_dir(root.join("repos"))? {
            let repository = repository?;
            if !repository.file_name().to_str().is_some_and(digest) {
                continue;
            }
            for path in entry_paths(&repository.path())? {
                let (entry, bytes) = Entry::read_measured(&path)?;
                entries.insert(path, InventoryItem { entry, bytes });
            }
        }
        Ok(Self { entries })
    }

    pub fn insertion(&self, path: &Path) -> Result<u64> {
        if let Some(previous) = self.entries.get(path) {
            return Ok(previous.entry.inserted);
        }
        self.entries
            .values()
            .map(|item| item.entry.inserted)
            .max()
            .unwrap_or_default()
            .checked_add(1)
            .context("Local graph snapshot catalog insertion count exhausted")
    }

    pub fn evict_to_fit(
        &self,
        path: &Path,
        retained: &InventoryItem,
        limit: u64,
    ) -> Result<Vec<PathBuf>> {
        ensure!(
            retained.bytes <= ENTRY_LIMIT,
            "Graph snapshot catalog entry is too large"
        );
        let mut budget = Budget::default();
        budget.add(retained)?;
        ensure!(
            budget.used <= limit,
            "Graph snapshot exceeds the local cache size limit"
        );
        let mut oldest = self
            .entries
            .iter()
            .filter(|(existing, _)| existing.as_path() != path)
            .collect::<Vec<_>>();
        oldest.sort_by_key(|(_, item)| item.entry.inserted);
        for (_, item) in &oldest {
            budget.add(item)?;
        }
        let mut evictions = Vec::new();
        for (path, item) in oldest {
            if budget.used <= limit {
                break;
            }
            budget.remove(item)?;
            evictions.push(path.clone());
        }
        Ok(evictions)
    }

    pub fn apply(
        &mut self,
        retained: PathBuf,
        item: InventoryItem,
        evictions: Vec<PathBuf>,
    ) -> Result<()> {
        self.entries.insert(retained, item);
        for path in evictions {
            fs::remove_file(&path)
                .with_context(|| format!("Cannot evict graph snapshot {}", path.display()))?;
            self.entries.remove(&path);
        }
        Ok(())
    }

    pub fn prune(&self, root: &Path) -> Result<()> {
        let referenced = self.referenced_objects(root)?;
        for entry in fs::read_dir(root.join("objects"))? {
            let entry = entry?;
            if unused_object(&entry, &referenced)? {
                fs::remove_file(entry.path())
                    .context("Cannot prune unused graph snapshot object")?;
            }
        }
        Ok(())
    }

    fn referenced_objects(&self, root: &Path) -> Result<HashSet<PathBuf>> {
        let mut referenced = HashSet::new();
        for item in self.entries.values() {
            let manifest = item.entry.manifest()?;
            referenced.extend(objects(&manifest).map(|(path, _)| root.join(path)));
        }
        Ok(referenced)
    }
}

fn unused_object(entry: &DirEntry, referenced: &HashSet<PathBuf>) -> Result<bool> {
    let reserved = entry
        .file_name()
        .to_str()
        .is_some_and(|name| owned_object(name) || name.starts_with(".graph-publish-"));
    Ok(reserved && !referenced.contains(&entry.path()) && entry.file_type()?.is_file())
}

#[derive(Default)]
struct Budget {
    used: u64,
    references: HashMap<String, ObjectUsage>,
}

struct ObjectUsage {
    count: usize,
    bytes: u64,
}

impl Budget {
    fn add(&mut self, item: &InventoryItem) -> Result<()> {
        self.used = self
            .used
            .checked_add(item.bytes)
            .context("Graph cache size overflow")?;
        for (path, bytes) in objects(&item.entry.manifest()?) {
            let usage = self
                .references
                .entry(path)
                .or_insert(ObjectUsage { count: 0, bytes });
            ensure!(
                usage.bytes == bytes,
                "Incompatible size for a shared graph object"
            );
            if usage.count == 0 {
                self.used = self
                    .used
                    .checked_add(bytes)
                    .context("Graph cache size overflow")?;
            }
            usage.count += 1;
        }
        Ok(())
    }

    fn remove(&mut self, item: &InventoryItem) -> Result<()> {
        self.used -= item.bytes;
        for (path, _) in objects(&item.entry.manifest()?) {
            let usage = self
                .references
                .get_mut(&path)
                .context("Unknown graph object")?;
            usage.count -= 1;
            if usage.count == 0 {
                self.used -= usage.bytes;
            }
        }
        Ok(())
    }
}

fn objects(manifest: &Manifest) -> [(String, u64); 2] {
    [
        (
            manifest.structure.path("structure"),
            manifest.structure.compressed_bytes,
        ),
        (
            manifest.flows.path("flows"),
            manifest.flows.compressed_bytes,
        ),
    ]
}
