use crate::{discovery::excluded_directory, model::FileFingerprint};
use anyhow::{Context, Result};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

pub fn capture(paths: impl IntoIterator<Item = PathBuf>) -> Result<Vec<FileFingerprint>> {
    paths
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|path| fingerprint(&path, &[], false))
        .collect()
}

/// Capture explicit inputs, retaining absent paths and build-output exclusions.
/// Pass evaluated build roots, not just their nested class-output directories.
pub fn capture_excluding(
    paths: impl IntoIterator<Item = PathBuf>,
    output_roots: &[PathBuf],
) -> Result<Vec<FileFingerprint>> {
    paths
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|path| {
            let excluded_children = excluded_children(&path, output_roots);
            fingerprint(&path, &excluded_children, true)
        })
        .collect()
}

pub fn is_stale(items: &[FileFingerprint]) -> Result<bool> {
    for item in items {
        let path = Path::new(&item.path);
        let current = fingerprint(path, &item.excluded_children, true)?;
        if (
            current.missing,
            current.size,
            current.modified_ns,
            &current.hash,
        ) != (item.missing, item.size, item.modified_ns, &item.hash)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn source_paths(roots: &[PathBuf]) -> Result<Vec<PathBuf>> {
    collect_roots(roots, &[], false, false)
}

/// Explicit roots remain included even inside build outputs, allowing callers to
/// fingerprint generated sources after compilation without scanning class files.
pub fn source_paths_excluding(roots: &[PathBuf], output_roots: &[PathBuf]) -> Result<Vec<PathBuf>> {
    collect_roots(roots, output_roots, true, false)
}

/// Declared build inputs (resources/generator data/configuration) include all file types.
pub fn input_paths_excluding(roots: &[PathBuf], output_roots: &[PathBuf]) -> Result<Vec<PathBuf>> {
    collect_roots(roots, output_roots, true, true)
}

fn collect_roots(
    roots: &[PathBuf],
    output_roots: &[PathBuf],
    include_missing: bool,
    all_files: bool,
) -> Result<Vec<PathBuf>> {
    let mut paths = BTreeSet::new();
    for root in roots {
        let descendant_outputs: Vec<_> = output_roots
            .iter()
            .filter(|output| *output != root && output.starts_with(root))
            .cloned()
            .collect();
        if root.is_dir() {
            collect_sources(root, &descendant_outputs, all_files, &mut paths)?;
        } else if root.is_file() || include_missing {
            paths.insert(root.clone());
        }
    }
    Ok(paths.into_iter().collect())
}

fn collect_sources(
    root: &Path,
    output_roots: &[PathBuf],
    all_files: bool,
    paths: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    paths.insert(root.into());
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if output_roots
            .iter()
            .any(|output| entry.path().starts_with(output))
        {
            continue;
        }
        if kind.is_dir() && !excluded_directory(&name) {
            collect_sources(&entry.path(), output_roots, all_files, paths)?;
        } else if kind.is_file() && tracked_file(&entry.path(), all_files) {
            paths.insert(entry.path());
        }
    }
    Ok(())
}

fn tracked_file(path: &Path, all_files: bool) -> bool {
    all_files || relevant_file(path)
}

fn relevant_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    matches!(
        extension,
        "cs" | "java" | "csproj" | "sln" | "slnx" | "props" | "targets" | "gradle" | "kts"
    ) || matches!(
        name.as_str(),
        "global.json"
            | "nuget.config"
            | "lombok.config"
            | "gradle.properties"
            | "gradle-wrapper.properties"
            | "gradle-wrapper.jar"
            | "pom.xml"
            | "maven.config"
            | "jvm.config"
            | "extensions.xml"
            | "settings.xml"
            | "toolchains.xml"
            | "maven-wrapper.properties"
            | "maven-wrapper.jar"
            | "mvnw"
            | "mvnw.cmd"
    )
}

fn fingerprint(
    path: &Path,
    excluded_children: &[String],
    allow_missing: bool,
) -> Result<FileFingerprint> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if allow_missing && error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(FileFingerprint {
                path: path.to_string_lossy().into_owned(),
                missing: true,
                excluded_children: excluded_children.to_vec(),
                ..Default::default()
            });
        }
        result => result.with_context(|| format!("Cannot fingerprint {}", path.display()))?,
    };
    let modified_ns = if metadata.is_dir() {
        0
    } else {
        metadata
            .modified()?
            .duration_since(UNIX_EPOCH)?
            .as_nanos()
            .min(u64::MAX as u128) as u64
    };
    let hash = if metadata.is_dir() {
        directory_hash(path, excluded_children)?
    } else {
        file_hash(path)?
    };
    Ok(FileFingerprint {
        path: path.to_string_lossy().into_owned(),
        size: if metadata.is_dir() { 0 } else { metadata.len() },
        modified_ns,
        hash,
        excluded_children: excluded_children.to_vec(),
        missing: false,
    })
}

fn excluded_children(path: &Path, outputs: &[PathBuf]) -> Vec<String> {
    outputs
        .iter()
        .filter_map(|output| output.strip_prefix(path).ok()?.components().next())
        .map(|child| child.as_os_str().to_string_lossy().into_owned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn directory_hash(path: &Path, excluded_children: &[String]) -> Result<String> {
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if excluded_directory(&name) || excluded_children.contains(&name) {
            continue;
        }
        if generated_maven_marker(&entry)? {
            continue;
        }
        names.insert(name);
    }
    Ok(
        blake3::hash(names.into_iter().collect::<Vec<_>>().join("\0").as_bytes())
            .to_hex()
            .to_string(),
    )
}

fn generated_maven_marker(entry: &fs::DirEntry) -> Result<bool> {
    if entry.file_name() != ".mvn" || !entry.file_type()?.is_dir() {
        return Ok(false);
    }
    // Maven 4 creates .mvn/target/project-local-repo during model import.
    // Configuration remains visible as soon as .mvn contains authored inputs.
    Ok(!maven_configuration_present(&entry.path())?)
}

fn maven_configuration_present(path: &Path) -> Result<bool> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_name() != "target" || !entry.file_type()?.is_dir() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn file_hash(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maven_root_marker_creation_keeps_sources_current_but_configuration_stales_them() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("pom.xml"), "<project/>").unwrap();
        let before = capture(source_paths(&[root.path().into()]).unwrap()).unwrap();
        fs::create_dir(root.path().join(".mvn")).unwrap();
        assert!(!is_stale(&before).unwrap());
        fs::create_dir_all(root.path().join(".mvn/target/project-local-repo")).unwrap();
        fs::write(
            root.path()
                .join(".mvn/target/project-local-repo/consumer.pom"),
            "generated",
        )
        .unwrap();
        assert!(!is_stale(&before).unwrap());
        fs::write(root.path().join(".mvn/maven.config"), "-Pproduction").unwrap();
        assert!(is_stale(&before).unwrap());
    }

    #[test]
    fn declared_resource_contents_are_build_inputs() {
        let root = tempfile::tempdir().unwrap();
        let input = root.path().join("generator-schema.yaml");
        fs::write(&input, "before").unwrap();
        let paths = input_paths_excluding(&[root.path().into()], &[]).unwrap();
        let before = capture_excluding(paths, &[]).unwrap();
        fs::write(input, "after").unwrap();
        assert!(is_stale(&before).unwrap());
    }

    #[test]
    fn detects_edits_additions_and_deletions() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("App.java");
        fs::write(&file, "class App {}").unwrap();
        let capture_now = || capture(source_paths(&[directory.path().into()]).unwrap()).unwrap();
        let original = capture_now();
        assert!(!is_stale(&original).unwrap());
        fs::write(&file, "class New {}").unwrap();
        assert!(is_stale(&original).unwrap());
        let before_addition = capture_now();
        fs::write(directory.path().join("New.java"), "class New {}").unwrap();
        assert!(is_stale(&before_addition).unwrap());
        let before_delete = capture_now();
        fs::remove_file(&file).unwrap();
        assert!(is_stale(&before_delete).unwrap());
    }

    #[test]
    fn creating_ignored_build_outputs_does_not_stale_sources() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("App.java"), "class App {}").unwrap();
        let before = capture(source_paths(&[directory.path().into()]).unwrap()).unwrap();
        fs::create_dir(directory.path().join("build")).unwrap();
        fs::write(directory.path().join("build/output.class"), "compiled").unwrap();
        assert!(!is_stale(&before).unwrap());
    }

    #[test]
    fn custom_outputs_do_not_hide_authored_edits_after_round_trip() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("src/App.java");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "class App {}").unwrap();
        let outputs = vec![directory.path().join("custom-output")];
        let paths = source_paths_excluding(&[directory.path().into()], &outputs).unwrap();
        let before = capture_excluding(paths, &outputs).unwrap();
        let before: Vec<FileFingerprint> =
            serde_json::from_slice(&serde_json::to_vec(&before).unwrap()).unwrap();
        fs::create_dir_all(outputs[0].join("classes")).unwrap();
        fs::write(outputs[0].join("classes/App.class"), "compiled").unwrap();
        assert!(!is_stale(&before).unwrap());
        fs::write(source, "class Edited {}").unwrap();
        assert!(is_stale(&before).unwrap());
    }

    #[test]
    fn explicit_generated_roots_are_tracked_inside_excluded_outputs() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("custom-output");
        let generated = output.join("generated");
        fs::create_dir_all(&generated).unwrap();
        let file = generated.join("Generated.java");
        fs::write(&file, "class Generated {}").unwrap();
        let paths = source_paths_excluding(
            &[directory.path().into(), generated.clone()],
            std::slice::from_ref(&output),
        )
        .unwrap();
        assert!(paths.contains(&file));
        let before = capture_excluding(paths, &[output]).unwrap();
        fs::write(file, "class Changed {}").unwrap();
        assert!(is_stale(&before).unwrap());
    }

    #[test]
    fn absent_explicit_inputs_detect_creation_and_old_fingerprints_still_load() {
        let directory = tempfile::tempdir().unwrap();
        let absent = directory.path().join("external/settings.xml");
        let paths = source_paths_excluding(std::slice::from_ref(&absent), &[]).unwrap();
        let before = capture_excluding(paths, &[]).unwrap();
        assert_eq!(before.len(), 1);
        assert!(!is_stale(&before).unwrap());
        fs::create_dir_all(absent.parent().unwrap()).unwrap();
        fs::write(absent, "<settings/>").unwrap();
        assert!(is_stale(&before).unwrap());
        let old: FileFingerprint =
            serde_json::from_str(r#"{"path":"old","size":0,"modified_ns":0,"hash":"x"}"#).unwrap();
        assert!(!old.missing);
        assert!(old.excluded_children.is_empty());
    }

    #[test]
    fn maven_configuration_edits_are_inputs_without_scanning_all_xml() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join(".mvn/wrapper")).unwrap();
        for name in [
            "pom.xml",
            ".mvn/maven.config",
            ".mvn/jvm.config",
            ".mvn/extensions.xml",
            ".mvn/wrapper/maven-wrapper.properties",
            "mvnw",
            "irrelevant.xml",
        ] {
            fs::write(directory.path().join(name), "original").unwrap();
        }
        let paths = source_paths(&[directory.path().into()]).unwrap();
        assert!(!paths.contains(&directory.path().join("irrelevant.xml")));
        let before = capture(paths).unwrap();
        fs::write(directory.path().join(".mvn/maven.config"), "-Pcustom").unwrap();
        assert!(is_stale(&before).unwrap());
    }
}
