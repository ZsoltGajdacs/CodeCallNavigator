//! Sequential JVM engine execution, sharing the application's cancellation policy.
use super::{JavaInput, metadata::unescape};
use crate::analyzers::{
    AnalysisJob,
    process::{check_cancel, run_process},
};
use anyhow::{Context, Result};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, atomic::AtomicBool},
};

pub(super) fn run(
    inputs: &[JavaInput],
    owner: &super::build::JavaSourceSet,
    root: &Path,
    job: &AnalysisJob<'_>,
) -> Result<PathBuf> {
    let _stage = crate::metrics::stage("java.bytecode");
    fs::create_dir_all(root)?;
    let java = job
        .runtime
        .java_home
        .as_ref()
        .context("Java analysis requires JDK 25")?;
    let output = root.join("bytecode");
    let projects = inputs
        .iter()
        .enumerate()
        .filter(|(_, input)| {
            input.source_set.as_ref().is_some_and(|source| {
                source.project_dir == owner.project_dir && source.source_set == owner.source_set
            })
        })
        .map(|(index, _)| (index + 1).to_string())
        .collect::<Vec<_>>()
        .join(",");
    run_process(
        Command::new(java.join("bin/java"))
            .arg("-Xmx8g")
            .arg("-cp")
            .arg(job.runtime.bytecode_lib()?.join("*"))
            .arg("codecallnavigator.bytecode.Analyzer")
            .arg(&output)
            .arg("25")
            .arg("--project-inputs")
            .arg(projects)
            .args(inputs.iter().map(|input| &input.path)),
        root,
        "bytecode",
        job.cancel,
    )?;
    Ok(output)
}

#[derive(Default)]
pub(super) struct SourceInventories {
    parsed: HashMap<String, Arc<super::source_inventory::SourceInventory>>,
}

impl SourceInventories {
    pub(super) fn load(
        &mut self,
        inputs: &[JavaInput],
        output: &Path,
        job: &AnalysisJob<'_>,
    ) -> Result<super::source_inventory::Inventories> {
        let _stage = crate::metrics::stage("java.source_attribution");
        let mut result = HashMap::new();
        for (index, input) in inputs.iter().enumerate() {
            let Some(source) = &input.source_set else {
                continue;
            };
            let key = inventory_key(source, job.runtime)?;
            if !self.parsed.contains_key(&key) {
                let path = generate_inventory(source, &key, output, job)?;
                let inventory = super::source_inventory::SourceInventory::read_current(&path)?;
                self.parsed.insert(key.clone(), Arc::new(inventory));
                crate::metrics::count("java.source_inventories", 1);
            } else {
                crate::metrics::count("java.source_inventory_hits", 1);
            }
            result.insert(index + 1, self.parsed[&key].clone());
        }
        Ok(result)
    }
}

fn inventory_key(
    source: &super::build::JavaSourceSet,
    runtime: &super::super::AnalyzerRuntime,
) -> Result<String> {
    let adapter = runtime.bytecode_lib()?.join("codecallnavigator-bytecode.jar");
    let digest = blake3::hash(&fs::read(adapter)?).to_hex().to_string();
    let input = (super::source_inventory::VERSION, source, digest);
    Ok(blake3::hash(&serde_json::to_vec(&input)?)
        .to_hex()
        .to_string())
}

fn generate_inventory(
    source: &super::build::JavaSourceSet,
    key: &str,
    output: &Path,
    job: &AnalysisJob<'_>,
) -> Result<PathBuf> {
    let java = job
        .runtime
        .java_home
        .as_ref()
        .context("Source inventory requires JDK 25")?;
    let cache = job.path.join("source-inventories");
    fs::create_dir_all(&cache)?;
    let inventory = cache.join(format!("{key}.tsv"));
    cached_inventory(&inventory, job.cancel, |temporary| {
        run_process(
            Command::new(java.join("bin/java"))
                .arg("-Xmx2g")
                .arg("--add-exports=jdk.compiler/com.sun.tools.javac.code=ALL-UNNAMED")
                .arg("--add-exports=jdk.compiler/com.sun.tools.javac.tree=ALL-UNNAMED")
                .arg("-cp")
                .arg(job.runtime.bytecode_lib()?.join("*"))
                .arg("codecallnavigator.bytecode.SourceInventory")
                .arg(temporary)
                .arg(std::env::join_paths(
                    source.class_dirs.iter().chain(&source.compile_classpath),
                )?)
                .arg(std::env::join_paths(&source.class_dirs)?)
                .args(source.source_roots.iter().chain(&source.generated_roots)),
            output,
            &format!("source-inventory-{key}"),
            job.cancel,
        )?;
        Ok(())
    })?;
    Ok(inventory)
}

#[cfg(test)]
pub(super) fn source_inventories(
    inputs: &[JavaInput],
    output: &Path,
    job: &AnalysisJob<'_>,
) -> Result<()> {
    for (index, input) in inputs.iter().enumerate() {
        let Some(source) = &input.source_set else {
            continue;
        };
        let key = inventory_key(source, job.runtime)?;
        let inventory = generate_inventory(source, &key, output, job)?;
        fs::copy(inventory, output.join(format!("source-{}.tsv", index + 1)))?;
    }
    Ok(())
}

fn cached_inventory(
    inventory: &Path,
    cancel: &AtomicBool,
    generate: impl FnOnce(&Path) -> Result<()>,
) -> Result<()> {
    check_cancel(cancel)?;
    if inventory.is_file() {
        return Ok(());
    }
    let temporary = tempfile::NamedTempFile::new_in(
        inventory
            .parent()
            .context("Inventory cache requires a parent directory")?,
    )?;
    generate(temporary.path())?;
    check_cancel(cancel)?;
    temporary
        .persist(inventory)
        .map_err(|failure| failure.error)?;
    Ok(())
}

// Parse the Java Properties subset emitted by the adapter.
pub(super) fn status(path: &Path) -> Result<HashMap<String, String>> {
    Ok(fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.to_owned(), unescape(value)))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::Ordering};

    fn assert_empty(directory: &Path) {
        assert_eq!(fs::read_dir(directory).unwrap().count(), 0);
    }

    #[test]
    fn rebuilt_adapter_invalidates_source_inventory_cache_without_schema_change() {
        let directory = tempfile::tempdir().unwrap();
        let library = directory.path().join("analyzers/java-bytecode/lib");
        fs::create_dir_all(&library).unwrap();
        let jar = library.join("codecallnavigator-bytecode.jar");
        let runtime = super::super::super::AnalyzerRuntime {
            bundle_root: directory.path().into(),
            java_home: None,
        };
        let source = serde_json::from_value(serde_json::json!({
            "project": "fixture", "sourceSet": "main", "compileClasspath": [],
            "projectDir": "/repo", "isTest": false, "sourceRoots": ["/repo/src"],
            "generatedRoots": [], "classDirs": ["/repo/classes"]
        }))
        .unwrap();
        fs::write(&jar, "previous adapter").unwrap();
        let previous = inventory_key(&source, &runtime).unwrap();
        assert_eq!(previous, inventory_key(&source, &runtime).unwrap());
        fs::write(&jar, "rebuilt adapter").unwrap();
        assert_ne!(previous, inventory_key(&source, &runtime).unwrap());
        assert_eq!(super::super::source_inventory::VERSION, 8);
    }

    #[test]
    fn failed_inventory_is_cleaned_and_retry_is_cached() {
        let directory = tempfile::tempdir().unwrap();
        let inventory = directory.path().join("inventory.tsv");
        let cancel = AtomicBool::new(false);
        let failure = cached_inventory(&inventory, &cancel, |temporary| {
            fs::write(temporary, "partial")?;
            anyhow::bail!("source attribution failed");
        });
        assert!(failure.is_err());
        assert_empty(directory.path());
        cached_inventory(&inventory, &cancel, |temporary| {
            assert!(!inventory.exists());
            fs::write(temporary, "complete")?;
            Ok(())
        })
        .unwrap();
        cached_inventory(&inventory, &cancel, |_| {
            panic!("valid cache must be reused")
        })
        .unwrap();
        assert_eq!(fs::read_to_string(inventory).unwrap(), "complete");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn cancellation_after_generation_does_not_publish() {
        let directory = tempfile::tempdir().unwrap();
        let inventory = directory.path().join("inventory.tsv");
        let cancel = AtomicBool::new(false);
        let result = cached_inventory(&inventory, &cancel, |temporary| {
            fs::write(temporary, "complete")?;
            cancel.store(true, Ordering::Relaxed);
            Ok(())
        });
        assert!(result.is_err());
        assert_empty(directory.path());
    }

    #[test]
    fn publication_failure_cleans_temporary_before_error_is_dropped() {
        let directory = tempfile::tempdir().unwrap();
        let inventory = directory.path().join("inventory.tsv");
        let cancel = AtomicBool::new(false);
        let failure = cached_inventory(&inventory, &cancel, |temporary| {
            fs::write(temporary, "complete")?;
            fs::create_dir(&inventory)?;
            Ok(())
        })
        .unwrap_err();
        assert!(inventory.is_dir());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        drop(failure);
    }

    #[test]
    fn interrupted_inventory_process_does_not_poison_retry() {
        let directory = tempfile::tempdir().unwrap();
        let cache = directory.path().join("cache");
        fs::create_dir(&cache).unwrap();
        let inventory = cache.join("inventory.tsv");
        let cancel = Arc::new(AtomicBool::new(false));
        let started = directory.path().join("started");
        let trigger = cancel.clone();
        let marker = started.clone();
        let cancellation = std::thread::spawn(move || {
            for _ in 0..200 {
                if marker.exists() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            trigger.store(true, Ordering::Relaxed);
        });
        let result = cached_inventory(&inventory, &cancel, |temporary| {
            run_process(
                Command::new("/bin/sh")
                    .args([
                        "-c",
                        "printf partial > \"$1\"; touch \"$2\"; sleep 30",
                        "inventory",
                    ])
                    .arg(temporary)
                    .arg(&started),
                directory.path(),
                "interrupted-inventory",
                &cancel,
            )?;
            Ok(())
        });
        cancellation.join().unwrap();
        assert!(result.is_err());
        assert!(
            started.exists(),
            "cancellation must interrupt a running writer"
        );
        assert_empty(&cache);
        cancel.store(false, Ordering::Relaxed);
        cached_inventory(&inventory, &cancel, |temporary| {
            fs::write(temporary, "complete")?;
            Ok(())
        })
        .unwrap();
        assert_eq!(fs::read_to_string(inventory).unwrap(), "complete");
    }
}
