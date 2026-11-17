use crate::{
    discovery::{excluded_directory, generated_graph_output},
    model::FileFingerprint,
};
use anyhow::{Context, Result};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::UNIX_EPOCH,
};

#[cfg(test)]
mod output_tests;
mod policy;
pub use policy::{InputPolicy, ScanRoot};
#[cfg(test)]
mod policy_tests;

pub fn capture(paths: impl IntoIterator<Item = PathBuf>) -> Result<Vec<FileFingerprint>> {
    let _stage = crate::metrics::stage("fingerprints.capture");
    paths
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|path| !generated_graph_output(path))
        .map(|path| fingerprint(&path, &[], false, InputPolicy::ScopedV1))
        .collect()
}

/// Capture explicit inputs, retaining absent paths and build-output exclusions.
/// Pass evaluated build roots, not just their nested class-output directories.
pub fn capture_excluding(
    paths: impl IntoIterator<Item = PathBuf>,
    output_roots: &[PathBuf],
) -> Result<Vec<FileFingerprint>> {
    let _stage = crate::metrics::stage("fingerprints.capture");
    paths
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|path| !generated_graph_output(path))
        .map(|path| {
            let excluded_children = excluded_children(&path, output_roots);
            fingerprint(&path, &excluded_children, true, InputPolicy::ScopedV1)
        })
        .collect()
}

/// A captured analysis input changed, so its result must not be published.
#[derive(Debug)]
pub struct InputsChanged {
    message: &'static str,
    path: String,
    change: &'static str,
}
impl InputsChanged {
    /// Preserve typed invalidation for evaluated membership changes as well as
    /// file changes; genuine observation failures must keep their own error.
    pub fn new(message: &'static str, path: impl Into<String>, change: &'static str) -> Self {
        Self {
            message,
            path: path.into(),
            change,
        }
    }
}
impl std::fmt::Display for InputsChanged {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} Changed input: {:?} ({}).",
            self.message, self.path, self.change
        )
    }
}
impl std::error::Error for InputsChanged {}

pub fn require_unchanged(items: &[FileFingerprint], message: &'static str) -> Result<()> {
    if let Some((item, change)) = first_changed(items)? {
        return Err(InputsChanged::new(message, item.path.clone(), change).into());
    }
    Ok(())
}

pub fn is_stale(items: &[FileFingerprint]) -> Result<bool> {
    Ok(first_changed(items)?.is_some())
}

fn first_changed(items: &[FileFingerprint]) -> Result<Option<(&FileFingerprint, &'static str)>> {
    let _stage = crate::metrics::stage("fingerprints.verify");
    for item in items {
        let path = Path::new(&item.path);
        let current = fingerprint(path, &item.excluded_children, true, item.observation_policy)?;
        let change = classify_change(item, &current, path.is_dir());
        if let Some(change) = change {
            return Ok(Some((item, change)));
        }
    }
    Ok(None)
}

fn classify_change(
    before: &FileFingerprint,
    current: &FileFingerprint,
    directory: bool,
) -> Option<&'static str> {
    match (before.missing, current.missing) {
        (false, true) => return Some("removed"),
        (true, false) => return Some("created"),
        _ => {}
    }
    if before.kind != current.kind {
        return Some("input kind changed");
    }
    if before.hash != current.hash {
        return Some(hash_change(directory));
    }
    None
}

fn hash_change(directory: bool) -> &'static str {
    if directory {
        "directory membership or link target changed"
    } else {
        "content or link target changed"
    }
}

pub fn source_paths(roots: &[PathBuf]) -> Result<Vec<PathBuf>> {
    collect_roots(roots, &[], false, FilePolicy::Sources)
}

/// Explicit roots remain included even inside build outputs, allowing callers to
/// fingerprint generated sources after compilation without scanning class files.
pub fn source_paths_excluding(roots: &[PathBuf], output_roots: &[PathBuf]) -> Result<Vec<PathBuf>> {
    collect_roots(roots, output_roots, true, FilePolicy::Sources)
}

/// Declared build inputs (resources/generator data/configuration) include all file types.
pub fn input_paths_excluding(roots: &[PathBuf], output_roots: &[PathBuf]) -> Result<Vec<PathBuf>> {
    collect_roots(roots, output_roots, true, FilePolicy::BuildInputs)
}

/// Recursive observed scopes include all file types and symlink entries, without
/// descending nested directory symlinks. Explicit scope roots remain traversable.
#[cfg(test)]
pub(crate) fn observation_paths(
    roots: &[PathBuf],
    output_roots: &[PathBuf],
    cancel: &AtomicBool,
) -> Result<Vec<PathBuf>> {
    collect_roots(
        roots,
        output_roots,
        true,
        FilePolicy::ObservedScopes(cancel),
    )
}

/// Discover roots with explicit ownership intent. Required roots remain eligible
/// inside hidden/generated directories; all descendants use the shared policy.
pub fn collect_inputs(
    roots: &[ScanRoot],
    outputs: &[PathBuf],
    sources_only: bool,
) -> Result<Vec<PathBuf>> {
    let paths = roots
        .iter()
        .filter_map(|root| match root.eligible() {
            Ok(true) => Some(Ok(root.path().to_path_buf())),
            Ok(false) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<Result<Vec<_>>>()?;
    collect_roots(
        &paths,
        outputs,
        true,
        if sources_only {
            FilePolicy::Sources
        } else {
            FilePolicy::BuildInputs
        },
    )
}

fn collect_roots(
    roots: &[PathBuf],
    outputs: &[PathBuf],
    include_missing: bool,
    policy: FilePolicy<'_>,
) -> Result<Vec<PathBuf>> {
    collect_roots_with_policy(
        roots,
        outputs,
        include_missing,
        TraversalPolicy {
            files: policy,
            observation: InputPolicy::ScopedV1,
        },
    )
}

pub(crate) fn observation_paths_with_policy(
    roots: &[PathBuf],
    outputs: &[PathBuf],
    cancel: &AtomicBool,
    policy: InputPolicy,
) -> Result<Vec<PathBuf>> {
    collect_roots_with_policy(
        roots,
        outputs,
        true,
        TraversalPolicy {
            files: FilePolicy::ObservedScopes(cancel),
            observation: policy,
        },
    )
}

pub fn observe_content_with_policy(
    path: &Path,
    excluded_children: &[String],
    policy: InputPolicy,
) -> Result<ContentObservation> {
    observe_content_controlled(path, excluded_children, None, policy)
}

pub(crate) fn observe_content_cancellable_with_policy(
    path: &Path,
    excluded_children: &[String],
    cancel: &AtomicBool,
    policy: InputPolicy,
) -> Result<ContentObservation> {
    observe_content_controlled(path, excluded_children, Some(cancel), policy)
}

#[derive(Clone, Copy)]
enum FilePolicy<'a> {
    Sources,
    BuildInputs,
    ObservedScopes(&'a AtomicBool),
}

impl FilePolicy<'_> {
    fn check(self) -> Result<()> {
        if let Self::ObservedScopes(cancel) = self {
            anyhow::ensure!(
                !cancel.load(Ordering::Acquire),
                "Freshness observation cancelled"
            );
        }
        Ok(())
    }

    fn accepts(self, path: &Path) -> bool {
        !matches!(self, Self::Sources) || relevant_file(path)
    }
}

struct TraversalPolicy<'a> {
    files: FilePolicy<'a>,
    observation: InputPolicy,
}

fn collect_roots_with_policy(
    roots: &[PathBuf],
    output_roots: &[PathBuf],
    include_missing: bool,
    policy: TraversalPolicy<'_>,
) -> Result<Vec<PathBuf>> {
    let _stage = crate::metrics::stage("fingerprints.discover");
    let mut collector = SourceCollector {
        paths: BTreeSet::new(),
        visited: BTreeSet::new(),
        policy: policy.files,
        observation_policy: policy.observation,
    };
    for root in roots {
        policy.files.check()?;
        if generated_graph_output(root) {
            continue;
        }
        let descendant_outputs: Vec<_> = output_roots
            .iter()
            .filter(|output| *output != root && output.starts_with(root))
            .cloned()
            .collect();
        if root.is_dir() {
            collector.visit(root, &descendant_outputs)?;
        } else if root.is_file() || include_missing {
            collector.paths.insert(root.clone());
        }
    }
    Ok(collector.paths.into_iter().collect())
}

struct SourceCollector<'a> {
    paths: BTreeSet<PathBuf>,
    visited: BTreeSet<(PathBuf, Vec<PathBuf>)>,
    policy: FilePolicy<'a>,
    observation_policy: InputPolicy,
}

impl SourceCollector<'_> {
    fn visit(&mut self, root: &Path, output_roots: &[PathBuf]) -> Result<()> {
        let exclusions: Vec<_> = output_roots
            .iter()
            .filter(|output| output.starts_with(root))
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        // Distinct exclusion policies must still visit explicit generated roots.
        // Keep lexical paths: aliases may carry distinct source ownership.
        if !self.visited.insert((root.into(), exclusions)) {
            crate::metrics::count("fingerprint.directory_scan_reuses", 1);
            return Ok(());
        }
        crate::metrics::count("fingerprint.directories_visited", 1);
        self.paths.insert(root.into());
        for entry in fs::read_dir(root).with_context(|| {
            format!(
                "Cannot enumerate analysis input directory {}",
                root.display()
            )
        })? {
            self.collect_entry(entry?, output_roots)?;
        }
        Ok(())
    }

    fn collect_entry(&mut self, entry: fs::DirEntry, output_roots: &[PathBuf]) -> Result<()> {
        self.policy.check()?;
        if self.excluded_entry(&entry, output_roots)? {
            return Ok(());
        }
        let path = entry.path();
        let kind = entry.file_type()?;
        if matches!(self.policy, FilePolicy::ObservedScopes(_)) && !kind.is_dir() {
            self.paths.insert(path);
        } else {
            self.collect_regular(path, kind, output_roots)?;
        }
        Ok(())
    }

    fn excluded_entry(&self, entry: &fs::DirEntry, output_roots: &[PathBuf]) -> Result<bool> {
        let path = entry.path();
        if self.observation_policy.excludes(entry)? {
            return Ok(true);
        }
        if generated_graph_output(&path)
            || output_roots.iter().any(|output| path.starts_with(output))
        {
            return Ok(true);
        }
        if matches!(self.policy, FilePolicy::ObservedScopes(_)) {
            return Ok(excluded_directory_entry(entry)?
                || generated_graph_directory(entry)?
                || generated_maven_marker(entry)?);
        }
        Ok(false)
    }

    fn collect_regular(
        &mut self,
        path: PathBuf,
        kind: fs::FileType,
        output_roots: &[PathBuf],
    ) -> Result<()> {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if kind.is_dir() && !excluded_directory(&name) {
            self.visit(&path, output_roots)?;
        } else if kind.is_file() && self.policy.accepts(&path) {
            self.paths.insert(path);
        }
        Ok(())
    }
}

fn excluded_directory_entry(entry: &fs::DirEntry) -> Result<bool> {
    if !excluded_directory(&entry.file_name().to_string_lossy()) {
        return Ok(false);
    }
    let kind = entry.file_type()?;
    // Directory aliases obey the same generated-output policy; ordinary files
    // with names such as build/target remain observable.
    Ok(kind.is_dir() || (kind.is_symlink() && entry.path().is_dir()))
}

fn relevant_file(path: &Path) -> bool {
    if path
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|name| name == ".codecallnavigator")
        && path.file_name().is_some_and(|name| name == "layers.yml")
    {
        return true;
    }
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

/// Content-only observation shared with portable artifact verification.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputKind {
    File,
    Directory,
    #[default]
    Missing,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ContentObservation {
    pub kind: InputKind,
    pub hash: String,
}

pub fn observe_content(path: &Path, excluded_children: &[String]) -> Result<ContentObservation> {
    observe_content_with_policy(path, excluded_children, InputPolicy::ScopedV1)
}

fn check_cancel(cancel: Option<&AtomicBool>) -> Result<()> {
    anyhow::ensure!(
        !cancel.is_some_and(|flag| flag.load(Ordering::Acquire)),
        "Freshness observation cancelled"
    );
    Ok(())
}

fn observe_content_controlled(
    path: &Path,
    excluded_children: &[String],
    cancel: Option<&AtomicBool>,
    policy: InputPolicy,
) -> Result<ContentObservation> {
    check_cancel(cancel)?;
    let metadata = match fs::metadata(path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            policy.bind_links(path, String::new())?;
            return Ok(ContentObservation {
                kind: InputKind::Missing,
                hash: String::new(),
            });
        }
        Err(error) => return Err(error).context("Cannot read analysis input"),
    };
    let (kind, hash) = if metadata.is_dir() {
        (
            InputKind::Directory,
            directory_hash_controlled(path, excluded_children, cancel, policy)?,
        )
    } else {
        anyhow::ensure!(
            metadata.is_file(),
            "Unsupported nonregular analysis input {}",
            path.display()
        );
        (InputKind::File, file_hash_controlled(path, cancel)?)
    };
    let hash = policy.bind_links(path, hash)?;
    Ok(ContentObservation { kind, hash })
}

fn fingerprint(
    path: &Path,
    excluded_children: &[String],
    allow_missing: bool,
    policy: InputPolicy,
) -> Result<FileFingerprint> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if allow_missing && error.kind() == std::io::ErrorKind::NotFound => {
            policy.bind_links(path, String::new())?;
            return Ok(FileFingerprint {
                path: path.to_string_lossy().into_owned(),
                missing: true,
                kind: InputKind::Missing,
                observation_policy: policy,
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
    let observation = observe_content_with_policy(path, excluded_children, policy)?;
    Ok(FileFingerprint {
        path: path.to_string_lossy().into_owned(),
        size: if metadata.is_dir() { 0 } else { metadata.len() },
        modified_ns,
        hash: observation.hash,
        kind: observation.kind,
        excluded_children: excluded_children.to_vec(),
        missing: false,
        observation_policy: policy,
    })
}

pub(crate) fn excluded_children(path: &Path, outputs: &[PathBuf]) -> Vec<String> {
    outputs
        .iter()
        .filter_map(|output| output.strip_prefix(path).ok()?.components().next())
        .map(|child| child.as_os_str().to_string_lossy().into_owned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn directory_hash_controlled(
    path: &Path,
    excluded_children: &[String],
    cancel: Option<&AtomicBool>,
    policy: InputPolicy,
) -> Result<String> {
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(path)? {
        check_cancel(cancel)?;
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if policy.excludes(&entry)? {
            continue;
        }
        if excluded_directory(&name) || excluded_children.contains(&name) {
            continue;
        }
        if generated_graph_output(&entry.path()) || generated_graph_directory(&entry)? {
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

fn generated_graph_directory(entry: &fs::DirEntry) -> Result<bool> {
    if entry.file_name() != ".codecallnavigator" || !entry.file_type()?.is_dir() {
        return Ok(false);
    }
    for child in fs::read_dir(entry.path())? {
        if !generated_graph_output(&child?.path()) {
            return Ok(false);
        }
    }
    Ok(true)
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

fn file_hash_controlled(path: &Path, cancel: Option<&AtomicBool>) -> Result<String> {
    check_cancel(cancel)?;
    let mut options = fs::OpenOptions::new();
    options.read(true);
    // A symlink can change between metadata and open. Nonblocking open plus
    // descriptor metadata prevents FIFO/device targets from blocking or streaming.
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let mut file = options.open(path)?;
    anyhow::ensure!(
        file.metadata()?.is_file(),
        "Unsupported nonregular analysis input {}",
        path.display()
    );
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 65536];
    let mut bytes = 0;
    loop {
        check_cancel(cancel)?;
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        bytes += count as u64;
    }
    crate::metrics::count("fingerprint.files_hashed", 1);
    crate::metrics::count("fingerprint.bytes_hashed", bytes);
    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maven_root_marker_creation_keeps_sources_current_but_configuration_stales_them() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("pom.xml"), "<project/>").unwrap();
        let mut inputs = source_paths(&[root.path().into()]).unwrap();
        inputs.push(root.path().join(".mvn/maven.config"));
        let before = capture_excluding(inputs, &[]).unwrap();
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
        content_validity_ignores_metadata(&file);
        let capture_now = || capture(source_paths(&[directory.path().into()]).unwrap()).unwrap();
        let before_addition = capture_now();
        fs::write(directory.path().join("New.java"), "class New {}").unwrap();
        assert!(is_stale(&before_addition).unwrap());
        let error = require_unchanged(&before_addition, "Analysis rejected").unwrap_err();
        assert!(
            error
                .to_string()
                .contains(directory.path().to_str().unwrap())
        );
        assert!(error.to_string().contains("directory membership"));
        let before_delete = capture_now();
        let file_before_delete = capture([file.clone()]).unwrap();
        fs::remove_file(&file).unwrap();
        assert!(is_stale(&before_delete).unwrap());
        let error = require_unchanged(&file_before_delete, "Analysis rejected").unwrap_err();
        assert!(error.to_string().contains(file.to_str().unwrap()));
        assert!(error.to_string().contains("removed"));

        empty_input_kind_replacements_are_rejected();
    }

    fn content_validity_ignores_metadata(file: &Path) {
        // Deterministic timestamps prove validity follows bytes, without sleeps
        // or depending on the filesystem clock resolution.
        let original_time = UNIX_EPOCH + std::time::Duration::from_secs(10);
        let touched_time = UNIX_EPOCH + std::time::Duration::from_secs(20);
        let rewritten_time = UNIX_EPOCH + std::time::Duration::from_secs(30);
        let set_modified = |time| {
            fs::File::options()
                .write(true)
                .open(file)
                .unwrap()
                .set_modified(time)
                .unwrap();
        };
        set_modified(original_time);
        let original = capture([file.to_path_buf()]).unwrap();
        assert!(!is_stale(&original).unwrap());
        set_modified(touched_time);
        assert!(
            !is_stale(&original).unwrap(),
            "touching an input preserves validity"
        );
        require_unchanged(&original, "Analysis rejected").unwrap();
        fs::write(file, "class App {}").unwrap();
        set_modified(rewritten_time);
        assert!(
            !is_stale(&original).unwrap(),
            "same-byte rewrites preserve validity"
        );
        require_unchanged(&original, "Analysis rejected").unwrap();
        // Same-length edits must still invalidate when metadata is restored.
        fs::write(file, "class New {}").unwrap();
        set_modified(original_time);
        assert!(is_stale(&original).unwrap());
        let error = require_unchanged(&original, "Analysis rejected").unwrap_err();
        assert!(error.is::<InputsChanged>());
        assert!(error.to_string().contains(file.to_str().unwrap()));
        assert!(error.to_string().contains("content or link target changed"));
    }

    fn empty_input_kind_replacements_are_rejected() {
        // Empty files and directories share the same content digest. Kind must
        // distinguish replacements even when metadata cannot distinguish them.
        let directory = tempfile::tempdir().unwrap();
        let empty = directory.path().join("empty");
        fs::write(&empty, "").unwrap();
        fs::File::options()
            .write(true)
            .open(&empty)
            .unwrap()
            .set_modified(UNIX_EPOCH)
            .unwrap();
        let empty_file = capture([empty.clone()]).unwrap();
        fs::remove_file(&empty).unwrap();
        fs::create_dir(&empty).unwrap();
        assert!(is_stale(&empty_file).unwrap(), "file replaced by directory");
        let error = require_unchanged(&empty_file, "Analysis rejected").unwrap_err();
        assert!(error.is::<InputsChanged>());
        assert!(error.to_string().contains(empty.to_str().unwrap()));
        let empty_directory = capture([empty.clone()]).unwrap();
        fs::remove_dir(&empty).unwrap();
        fs::write(&empty, "").unwrap();
        fs::File::options()
            .write(true)
            .open(&empty)
            .unwrap()
            .set_modified(UNIX_EPOCH)
            .unwrap();
        assert!(
            is_stale(&empty_directory).unwrap(),
            "directory replaced by file"
        );
        let error = require_unchanged(&empty_directory, "Analysis rejected").unwrap_err();
        assert!(error.is::<InputsChanged>());
        assert!(error.to_string().contains(empty.to_str().unwrap()));
    }

    #[test]
    fn creating_ignored_build_outputs_does_not_stale_sources() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("App.java"), "class App {}").unwrap();
        let before = capture(source_paths(&[directory.path().into()]).unwrap()).unwrap();
        fs::create_dir(directory.path().join("build")).unwrap();
        fs::write(directory.path().join("build/output.class"), "compiled").unwrap();
        assert!(!is_stale(&before).unwrap());
        // A fresh checkout has no declared frontend output until its first build.
        let frontend = directory.path().join("frontend");
        fs::create_dir(&frontend).unwrap();
        fs::write(frontend.join("pom.xml"), "<project/>").unwrap();
        let dist = frontend.join("dist");
        let outputs = vec![dist.clone()];
        let paths = source_paths_excluding(&[directory.path().into()], &outputs).unwrap();
        let before = capture_excluding(paths, &outputs).unwrap();
        fs::create_dir(&dist).unwrap();
        let generated = dist.join("Generated.java");
        fs::write(&generated, "class Generated {}").unwrap();
        require_unchanged(&before, "Analysis rejected").unwrap();
        assert!(
            !source_paths_excluding(&[directory.path().into()], &outputs)
                .unwrap()
                .contains(&generated)
        );
        // Explicit compiler-required roots retain coverage inside declared outputs.
        let explicit = source_paths_excluding(std::slice::from_ref(&dist), &outputs).unwrap();
        let required = capture_excluding(explicit, &outputs).unwrap();
        fs::write(&generated, "class Changed {}").unwrap();
        assert!(require_unchanged(&required, "Analysis rejected").is_err());
        fs::write(directory.path().join("App.java"), "class Edited {}").unwrap();
        assert!(require_unchanged(&before, "Analysis rejected").is_err());
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
    fn absent_explicit_inputs_detect_creation_and_incomplete_fingerprints_are_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let absent = directory.path().join("external/settings.xml");
        let paths = source_paths_excluding(std::slice::from_ref(&absent), &[]).unwrap();
        let before = capture_excluding(paths, &[]).unwrap();
        assert_eq!(before.len(), 1);
        assert!(!is_stale(&before).unwrap());
        fs::create_dir_all(absent.parent().unwrap()).unwrap();
        fs::write(&absent, "<settings/>").unwrap();
        assert!(is_stale(&before).unwrap());
        let error = require_unchanged(&before, "Analysis rejected").unwrap_err();
        assert!(error.to_string().contains(absent.to_str().unwrap()));
        assert!(error.to_string().contains("created"));
        let missing_kind = serde_json::from_str::<FileFingerprint>(
            r#"{"path":"old","size":0,"modified_ns":0,"hash":"x"}"#,
        );
        assert!(missing_kind.unwrap_err().to_string().contains("kind"));
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
        let paths =
            source_paths(&[directory.path().into(), directory.path().join(".mvn")]).unwrap();
        assert!(!paths.contains(&directory.path().join("irrelevant.xml")));
        let before = capture(paths).unwrap();
        fs::write(directory.path().join(".mvn/maven.config"), "-Pcustom").unwrap();
        assert!(is_stale(&before).unwrap());
    }
}
