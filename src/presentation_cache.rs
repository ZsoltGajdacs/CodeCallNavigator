//! Write-only retention of successful CLI snapshots for isolated profiling and rollback.
//! Production graph queries never read these SQLite generations.
use crate::engine_client::Snapshot;
use anyhow::{Context, Result, ensure};
use codecallnavigator::store::SqliteStore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationCache {
    pub source_generation: String,
    pub cache_generation: String,
}

pub fn retain(
    store: &SqliteStore,
    snapshot: &Snapshot,
    cancel: &AtomicBool,
) -> Result<PresentationCache> {
    ensure!(
        !cancel.load(Ordering::Acquire),
        "Presentation cache retention cancelled"
    );
    let generation = snapshot
        .context
        .descriptor
        .generation
        .as_deref()
        .context("CLI snapshot omitted its source generation")?;
    let address = serde_json::to_value(&snapshot.context.descriptor)?;
    let mut saved = store
        .load_session()?
        .unwrap_or_else(|| serde_json::json!({}));
    if !saved.is_object() {
        saved = serde_json::json!({"recovered_session":saved});
    }
    let marker = match reusable(store, &saved, &address, generation)? {
        Some(marker) => marker,
        None => PresentationCache {
            source_generation: generation.into(),
            cache_generation: store
                .publish(&snapshot.graph.data, cancel)
                .context("CLI snapshot loaded, but its profiling cache could not be retained")?,
        },
    };
    // Preserve editable root, navigation and unknown settings exactly, including
    // malformed navigation that prevented typed session restoration at startup.
    saved["graph_context"] = address;
    saved["presentation_cache"] = serde_json::to_value(&marker)?;
    if saved.get("root").is_none() {
        saved["root"] = snapshot
            .context
            .descriptor
            .project
            .to_string_lossy()
            .into_owned()
            .into();
    }
    store
        .save_session(&saved)
        .context("CLI snapshot loaded, but its profiling cache marker could not be saved")?;
    Ok(marker)
}

fn reusable(
    store: &SqliteStore,
    saved: &Value,
    address: &Value,
    generation: &str,
) -> Result<Option<PresentationCache>> {
    if saved.get("graph_context") != Some(address) {
        return Ok(None);
    }
    let Some(marker) = saved
        .get("presentation_cache")
        .cloned()
        .and_then(|value| serde_json::from_value::<PresentationCache>(value).ok())
    else {
        return Ok(None);
    };
    if marker.source_generation != generation {
        return Ok(None);
    }
    if store.active_generation()?.as_deref() != Some(marker.cache_generation.as_str()) {
        return Ok(None);
    }
    if !store.generation_exists(&marker.cache_generation)? {
        return Ok(None);
    }
    Ok(Some(marker))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{engine_client::QueryContext, runtime::Runtime};
    use codecallnavigator::{
        artifact,
        model::{GraphData, Node},
    };
    use std::{fs, path::Path};

    fn snapshot(root: &Path) -> Snapshot {
        let graph = GraphData {
            nodes: vec![Node {
                id: "leaf".into(),
                language: "java".into(),
                kind: "method".into(),
                label: "leaf".into(),
                signature: "leaf()".into(),
                project: "Example".into(),
                build: root.join("pom.xml").to_string_lossy().into_owned(),
                file: root.join("Example.java").to_string_lossy().into_owned(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let artifact = artifact::canonical(&graph, &[("repo".into(), root.into())]).unwrap();
        fs::create_dir(root.join(".codecallnavigator")).unwrap();
        fs::write(root.join(artifact::GRAPH_PATH), artifact.bytes().unwrap()).unwrap();
        QueryContext::for_project(&Runtime::discover().unwrap(), root.into())
            .snapshot(&AtomicBool::new(false))
            .unwrap()
    }

    #[test]
    fn identical_portable_bytes_in_another_checkout_do_not_reuse_hydrated_cache() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let store = SqliteStore::open(cache.path()).unwrap();
        let first = snapshot(first.path());
        let second = snapshot(second.path());
        assert_eq!(
            first.context.descriptor.generation,
            second.context.descriptor.generation
        );
        let before = retain(&store, &first, &AtomicBool::new(false)).unwrap();
        let after = retain(&store, &second, &AtomicBool::new(false)).unwrap();
        assert_ne!(
            before.cache_generation, after.cache_generation,
            "Another physical checkout reused the previous hydrated cache"
        );
        let retained = store.load_generation(&after.cache_generation).unwrap();
        assert_eq!(
            retained.nodes[0].file, second.graph.data.nodes[0].file,
            "Profiling retained another checkout's hydrated source location"
        );
        let saved = store.load_session().unwrap().unwrap();
        assert_eq!(
            saved["graph_context"]["project"],
            second.context.descriptor.project.to_string_lossy().as_ref()
        );
    }

    #[test]
    fn reopening_recreates_a_missing_derived_generation() {
        let project = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let snapshot = snapshot(project.path());
        let store = SqliteStore::open(cache.path()).unwrap();
        let before = retain(&store, &snapshot, &AtomicBool::new(false)).unwrap();
        fs::remove_file(cache.path().join(&before.cache_generation)).unwrap();
        drop(store);

        let reopened = SqliteStore::open(cache.path()).unwrap();
        let after = retain(&reopened, &snapshot, &AtomicBool::new(false)).unwrap();
        assert_ne!(before.cache_generation, after.cache_generation);
        assert!(cache.path().join(&after.cache_generation).is_file());
        assert_eq!(
            reopened.load_session().unwrap().unwrap()["presentation_cache"],
            serde_json::to_value(after).unwrap()
        );
    }

    #[test]
    fn malformed_scalar_metadata_is_preserved_when_retention_recovers() {
        let project = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let store = SqliteStore::open(cache.path()).unwrap();
        let snapshot = snapshot(project.path());
        let original = serde_json::json!("malformed settings");
        store.save_session(&original).unwrap();
        let marker = retain(&store, &snapshot, &AtomicBool::new(false)).unwrap();
        let saved = store.load_session().unwrap().unwrap();
        assert_eq!(saved["recovered_session"], original);
        assert_eq!(
            store.active_generation().unwrap().as_deref(),
            Some(marker.cache_generation.as_str())
        );
        crate::workspace::save_session(&store, serde_json::json!({"root":"pending edit"})).unwrap();
        assert_eq!(
            store.load_session().unwrap().unwrap()["recovered_session"],
            original
        );
    }

    #[test]
    fn cancelled_retention_cannot_report_success_or_replace_metadata() {
        let project = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let store = SqliteStore::open(cache.path()).unwrap();
        let snapshot = snapshot(project.path());
        let saved = serde_json::json!({"root":"pending"});
        store.save_session(&saved).unwrap();
        assert!(retain(&store, &snapshot, &AtomicBool::new(true)).is_err());
        assert!(store.active_generation().unwrap().is_none());
        assert_eq!(store.load_session().unwrap(), Some(saved));
    }
}
