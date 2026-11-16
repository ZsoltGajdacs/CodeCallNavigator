//! Independent test-data encoding, without graph-library or engine dependencies.
use serde_json::{Value, json};
use std::{fs, path::Path};

pub fn publish(root: &Path, mut artifact: Value) {
    let directory = root.join(".codecallnavigator");
    fs::create_dir_all(directory.join("objects")).unwrap();
    artifact["schema"] = json!("codecallnavigator.graph.v2");
    let flows = artifact["graph"]
        .as_object_mut()
        .unwrap()
        .remove("flow_inventory")
        .unwrap_or(Value::Null);
    assert!(
        flows.is_null(),
        "Comparison fixture encodes an empty inventory"
    );
    artifact["graph"]["flow_catalogue"] = json!({"inventory_version":null,"symbols":[]});
    let mut manifest = json!({"schema":"codecallnavigator.graph.manifest.v2"});
    for (kind, value) in [("structure", artifact), ("flows", flows)] {
        let raw = serde_json::to_vec(&value).unwrap();
        let compressed = zstd::stream::encode_all(raw.as_slice(), 9).unwrap();
        let digest = blake3::hash(&compressed).to_hex().to_string();
        fs::write(
            directory.join(format!("objects/{digest}.{kind}.json.zst")),
            &compressed,
        )
        .unwrap();
        manifest[kind] = json!({"digest":digest,"compressed_bytes":compressed.len(),"uncompressed_bytes":raw.len()});
    }
    fs::write(
        directory.join("graph.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
}

pub fn load(root: &Path) -> Value {
    let directory = root.join(".codecallnavigator");
    let manifest: Value =
        serde_json::from_slice(&fs::read(directory.join("graph.json")).unwrap()).unwrap();
    let compressed = fs::read(directory.join(format!(
        "objects/{}.structure.json.zst",
        manifest["structure"]["digest"].as_str().unwrap()
    )))
    .unwrap();
    let mut artifact: Value =
        serde_json::from_slice(&zstd::stream::decode_all(compressed.as_slice()).unwrap()).unwrap();
    artifact["graph"]
        .as_object_mut()
        .unwrap()
        .remove("flow_catalogue");
    artifact
}
