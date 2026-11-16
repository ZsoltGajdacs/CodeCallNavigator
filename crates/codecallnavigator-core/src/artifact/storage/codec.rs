use super::SCHEMA;
use crate::artifact::Artifact;
use crate::flow::FlowInventory;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::io::Read;

const MANIFEST_LIMIT: usize = 64 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Part {
    pub digest: String,
    pub compressed_bytes: u64,
    pub uncompressed_bytes: u64,
}

impl Part {
    pub fn path(&self, kind: &str) -> String {
        format!("objects/{}.{}.json.zst", self.digest, kind)
    }

    fn validate(&self) -> Result<()> {
        ensure!(
            self.digest.len() == 64
                && self
                    .digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
            "Invalid graph object digest"
        );
        ensure!(self.compressed_bytes > 0, "Empty compressed graph object");
        ensure!(
            self.uncompressed_bytes < u64::MAX,
            "Invalid graph object length"
        );
        Ok(())
    }

    pub(crate) fn decode(&self, input: impl Read) -> Result<Vec<u8>> {
        let mut compressed = Vec::new();
        input
            .take(self.compressed_bytes.saturating_add(1))
            .read_to_end(&mut compressed)?;
        ensure!(
            compressed.len() as u64 == self.compressed_bytes,
            "Compressed graph object length mismatch"
        );
        ensure!(
            blake3::hash(&compressed).to_hex().as_str() == self.digest,
            "Graph object digest mismatch"
        );
        let decoder = zstd::stream::read::Decoder::new(compressed.as_slice())
            .context("Invalid compressed graph object")?;
        let mut decoded = Vec::new();
        decoder
            .take(self.uncompressed_bytes + 1)
            .read_to_end(&mut decoded)?;
        ensure!(
            decoded.len() as u64 == self.uncompressed_bytes,
            "Uncompressed graph object length mismatch"
        );
        Ok(decoded)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub structure: Part,
    pub flows: Part,
}

impl Manifest {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        ensure!(
            bytes.len() <= MANIFEST_LIMIT,
            "Graph manifest exceeds 64 KiB"
        );
        let manifest: Self = serde_json::from_slice(bytes)
            .context("Invalid graph manifest; reanalyze this project")?;
        ensure!(
            manifest.schema == SCHEMA,
            "Unsupported graph artifact format; reanalyze this project"
        );
        manifest.structure.validate()?;
        manifest.flows.validate()?;
        Ok(manifest)
    }

    pub(crate) fn bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = serde_json::to_vec(self)?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    /// Reconstruct a complete artifact from independently retrieved immutable objects.
    pub fn complete_with(&self, mut read: impl FnMut(&str) -> Result<Vec<u8>>) -> Result<Artifact> {
        let bytes = read(&self.structure.path("structure"))?;
        let mut artifact = decode_structure(&self.structure.decode(bytes.as_slice())?)?;
        let bytes = read(&self.flows.path("flows"))?;
        artifact.graph["flow_inventory"] =
            serde_json::from_slice(&self.flows.decode(bytes.as_slice())?)?;
        let recorded: FlowCatalogue =
            serde_json::from_value(artifact.graph["flow_catalogue"].clone())?;
        ensure!(
            recorded == catalogue_value(&artifact.graph["flow_inventory"]),
            "Flow inventory does not match structural catalogue"
        );
        artifact
            .graph
            .as_object_mut()
            .context("Missing graph structure")?
            .remove("flow_catalogue");
        artifact.validate()?;
        Ok(artifact)
    }
}

/// The manifest bytes used for generation identity plus the exact compressed objects.
pub struct EncodedParts {
    pub(crate) manifest: Manifest,
    bytes: Vec<u8>,
    pub(crate) structure: Vec<u8>,
    pub(crate) flows: Vec<u8>,
}

impl EncodedParts {
    pub fn encode(artifact: &Artifact) -> Result<Self> {
        let mut value: serde_json::Value = serde_json::from_slice(&artifact.bytes()?)?;
        let flows = value["graph"]
            .as_object_mut()
            .context("Missing artifact graph")?
            .remove("flow_inventory")
            .unwrap_or(serde_json::Value::Null);
        value["graph"]["flow_catalogue"] = serde_json::to_value(catalogue_value(&flows))?;
        let (structure, structure_bytes) = encode_part(&value)?;
        let (flow_part, flows_bytes) = encode_part(&flows)?;
        let manifest = Manifest {
            schema: SCHEMA.into(),
            structure,
            flows: flow_part,
        };
        let bytes = manifest.bytes()?;
        Ok(Self {
            manifest,
            bytes,
            structure: structure_bytes,
            flows: flows_bytes,
        })
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct FlowCatalogue {
    inventory_version: Option<u32>,
    symbols: Vec<String>,
}

fn catalogue(inventory: Option<&FlowInventory>) -> FlowCatalogue {
    let mut symbols: Vec<_> = inventory
        .into_iter()
        .flat_map(|inventory| &inventory.functions)
        .map(|function| function.symbol.clone())
        .collect();
    symbols.sort();
    symbols.dedup();
    FlowCatalogue {
        inventory_version: inventory.map(|inventory| inventory.version),
        symbols,
    }
}

fn catalogue_value(inventory: &serde_json::Value) -> FlowCatalogue {
    let mut symbols: Vec<_> = inventory["functions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|function| function["symbol"].as_str().map(str::to_owned))
        .collect();
    symbols.sort();
    symbols.dedup();
    FlowCatalogue {
        inventory_version: inventory["version"].as_u64().map(|version| version as u32),
        symbols,
    }
}

pub(super) fn validate_catalogue(
    structure: &Artifact,
    inventory: Option<&FlowInventory>,
) -> Result<()> {
    let recorded: FlowCatalogue = serde_json::from_value(structure.graph["flow_catalogue"].clone())
        .context("Invalid structural flow catalogue")?;
    ensure!(
        recorded == catalogue(inventory),
        "Flow inventory does not match structural catalogue"
    );
    Ok(())
}

pub(super) fn validate_structure_catalogue(structure: &Artifact) -> Result<()> {
    ensure!(
        structure.graph.get("flow_inventory").is_none(),
        "Structural object contains embedded flow inventory"
    );
    let catalogue: FlowCatalogue =
        serde_json::from_value(structure.graph["flow_catalogue"].clone())
            .context("Invalid structural flow catalogue")?;
    ensure!(
        catalogue
            .inventory_version
            .is_none_or(|version| version == crate::flow::INVENTORY_VERSION),
        "Unsupported catalogue inventory version"
    );
    ensure!(
        catalogue.symbols.windows(2).all(|pair| pair[0] < pair[1]),
        "Flow catalogue symbols must be unique and sorted"
    );
    ensure!(
        catalogue.inventory_version.is_some() || catalogue.symbols.is_empty(),
        "Absent inventory has flow symbols"
    );
    let nodes: std::collections::HashSet<_> = structure.graph["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|node| node["id"].as_str())
        .collect();
    ensure!(
        catalogue
            .symbols
            .iter()
            .all(|symbol| nodes.contains(symbol.as_str())),
        "Flow catalogue refers to an unknown symbol"
    );
    Ok(())
}

pub(super) fn decode_structure(bytes: &[u8]) -> Result<Artifact> {
    let artifact: Artifact =
        serde_json::from_slice(bytes).context("Invalid structural graph JSON")?;
    // Reject embedded bodies before the general artifact validator can deserialize them.
    validate_structure_catalogue(&artifact)?;
    artifact.validate()?;
    Ok(artifact)
}

impl AsRef<[u8]> for EncodedParts {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

fn encode_part(value: &serde_json::Value) -> Result<(Part, Vec<u8>)> {
    let decoded = serde_json::to_vec(value)?;
    // encode_all uses a single-threaded encoder and no time-dependent headers.
    let encoded = zstd::stream::encode_all(decoded.as_slice(), 9)?;
    let part = Part {
        digest: blake3::hash(&encoded).to_hex().to_string(),
        compressed_bytes: encoded.len() as u64,
        uncompressed_bytes: decoded.len() as u64,
    };
    Ok((part, encoded))
}
