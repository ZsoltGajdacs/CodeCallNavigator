use super::{Manifest, publication::Guard};
use crate::{
    artifact::{Artifact, GRAPH_PATH},
    flow::FlowInventory,
};
use anyhow::{Context, Result};
use std::{
    fs::{self, File, OpenOptions},
    io::{Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::Mutex,
};

/// Open handles pin one immutable generation even after its paths are pruned.
/// Flow open failures are deferred so structural operations remain available.
pub struct Captured {
    bytes: Vec<u8>,
    manifest: Manifest,
    structure: Mutex<File>,
    flows: Mutex<Option<File>>,
    project: PathBuf,
}

impl Captured {
    pub fn open(project: &Path) -> Result<Self> {
        Self::inspect(project)?.1
    }

    /// Preserve exact manifest bytes even when a referenced part is unavailable.
    pub fn inspect(project: &Path) -> Result<(Vec<u8>, Result<Self>)> {
        let _guard = Guard::read(project)?;
        let bytes = fs::read(project.join(GRAPH_PATH)).context("Cannot read graph manifest")?;
        let captured = Self::from_manifest(project, bytes.clone());
        // A fresh checkout has no publication lock yet. If a publisher created
        // it while handles were being captured, retry under that new guard.
        if !_guard.coordinated() && Guard::exists(project) {
            return Self::inspect(project);
        }
        Ok((bytes, captured))
    }

    fn from_manifest(project: &Path, bytes: Vec<u8>) -> Result<Self> {
        let manifest = Manifest::from_bytes(&bytes)?;
        let directory = project.join(".codecallnavigator");
        super::publication::validate_object_directory(&directory)?;
        let structure = open_object(&directory.join(manifest.structure.path("structure")))
            .context("Cannot open graph structure object")?;
        let flows = open_object(&directory.join(manifest.flows.path("flows"))).ok();
        Ok(Self {
            bytes,
            manifest,
            structure: Mutex::new(structure),
            flows: Mutex::new(flows),
            project: project.into(),
        })
    }

    pub fn manifest_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn structure(&self) -> Result<Artifact> {
        let mut reader = self
            .structure
            .lock()
            .map_err(|_| anyhow::anyhow!("Graph structure reader poisoned"))?;
        reader.seek(SeekFrom::Start(0))?;
        super::codec::decode_structure(&self.manifest.structure.decode(&mut *reader)?)
    }

    pub fn load_flows(&self) -> Result<Option<FlowInventory>> {
        self.load_flows_for(&self.structure()?)
    }

    pub fn load_flows_for(&self, structure: &Artifact) -> Result<Option<FlowInventory>> {
        let mut pinned = self
            .flows
            .lock()
            .map_err(|_| anyhow::anyhow!("Graph flow reader poisoned"))?;
        if let Some(reader) = pinned.as_mut()
            && let Ok(inventory) = self.decode_flows(reader, structure)
        {
            return Ok(inventory);
        }
        // Reanalysis can repair a corrupt object by atomic inode replacement
        // without changing the manifest. Reopen once, only for this generation.
        let mut replacement = self.retry_flows()?;
        let inventory = self.decode_flows(&mut replacement, structure)?;
        *pinned = Some(replacement);
        Ok(inventory)
    }

    fn decode_flows(
        &self,
        reader: &mut File,
        structure: &Artifact,
    ) -> Result<Option<FlowInventory>> {
        reader.seek(SeekFrom::Start(0))?;
        let bytes = self.manifest.flows.decode(reader)?;
        let mut inventory: Option<FlowInventory> =
            serde_json::from_slice(&bytes).context("Invalid graph flow inventory")?;
        if let Some(inventory) = &mut inventory {
            structure.validate_flow_inventory(inventory)?;
        }
        super::codec::validate_catalogue(structure, inventory.as_ref())?;
        Ok(inventory)
    }

    fn retry_flows(&self) -> Result<File> {
        let _guard = Guard::read(&self.project)?;
        anyhow::ensure!(
            fs::read(self.project.join(GRAPH_PATH))? == self.bytes,
            "Graph generation changed before unavailable flows could be reopened"
        );
        super::publication::validate_object_directory(&self.project.join(".codecallnavigator"))?;
        open_object(
            &self
                .project
                .join(".codecallnavigator")
                .join(self.manifest.flows.path("flows")),
        )
        .context("Cannot reopen graph flow object")
    }

    pub fn complete(&self) -> Result<Artifact> {
        let mut artifact = self.structure()?;
        artifact.graph["flow_inventory"] = serde_json::to_value(self.load_flows_for(&artifact)?)?;
        artifact
            .graph
            .as_object_mut()
            .context("Missing graph structure")?
            .remove("flow_catalogue");
        artifact.validate()?;
        Ok(artifact)
    }
}

pub(super) fn open_object(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    anyhow::ensure!(
        file.metadata()?.is_file(),
        "Graph object is not a regular file"
    );
    Ok(file)
}
