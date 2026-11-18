//! The native SHA-256 observation is checked before core fingerprints take over.
use crate::fingerprint::{self, InputKind};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{ErrorKind, Read},
    path::{Path, PathBuf},
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Snapshot {
    pub scopes: Vec<Scope>,
    pub inputs: Vec<Input>,
    pub links: Vec<Link>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Scope {
    pub path: PathBuf,
    pub selection: Selection,
    pub members: Vec<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Selection {
    Java,
    Class,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Input {
    pub path: PathBuf,
    pub kind: InputKind,
    pub sha256: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Link {
    pub path: PathBuf,
    pub target: PathBuf,
}

impl Snapshot {
    pub fn validate(&self) -> Result<()> {
        let mut inputs = BTreeMap::new();
        for input in &self.inputs {
            absolute(&input.path)?;
            ensure!(
                inputs.insert(&input.path, input).is_none(),
                "Duplicate compiler input: {}",
                input.path.display()
            );
            match (&input.kind, &input.sha256) {
                (InputKind::File, Some(hash)) => ensure!(
                    hash.len() == 64
                        && hash
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                    "Invalid compiler SHA-256 for {}",
                    input.path.display()
                ),
                (InputKind::Directory | InputKind::Missing, None) => (),
                _ => anyhow::bail!(
                    "Compiler input kind and digest disagree: {}",
                    input.path.display()
                ),
            }
        }
        self.validate_scopes(&inputs)?;
        let mut links = BTreeSet::new();
        for link in &self.links {
            absolute(&link.path)?;
            ensure!(
                !link.target.as_os_str().is_empty() && links.insert(&link.path),
                "Invalid or duplicate compiler link: {}",
                link.path.display()
            );
        }
        Ok(())
    }

    fn validate_scopes(&self, inputs: &BTreeMap<&PathBuf, &Input>) -> Result<()> {
        let mut scopes = BTreeSet::new();
        for scope in &self.scopes {
            absolute(&scope.path)?;
            ensure!(
                scopes.insert((&scope.path, scope.selection)),
                "Duplicate compiler input scope: {}",
                scope.path.display()
            );
            ensure!(
                inputs.contains_key(&scope.path),
                "Compiler scope lacks a kind observation: {}",
                scope.path.display()
            );
            let mut members = BTreeSet::new();
            for member in &scope.members {
                absolute(member)?;
                ensure!(
                    member.starts_with(&scope.path)
                        && selected(member, scope.selection)
                        && members.insert(member),
                    "Invalid compiler scope member: {}",
                    member.display()
                );
                ensure!(
                    inputs
                        .get(member)
                        .is_some_and(|input| input.kind == InputKind::File),
                    "Compiler scope member lacks file evidence: {}",
                    member.display()
                );
            }
        }
        Ok(())
    }

    pub fn paths(&self) -> impl Iterator<Item = &PathBuf> {
        self.input_paths()
            .chain(self.links.iter().map(|link| &link.path))
    }

    /// A no-source operation observes source membership and aliases, but does
    /// not consume classpath entries just because the build supplied them.
    pub fn source_paths(&self) -> impl Iterator<Item = &PathBuf> {
        self.scopes
            .iter()
            .filter(|scope| scope.selection == Selection::Java)
            .flat_map(|scope| {
                std::iter::once(&scope.path).chain(&scope.members).chain(
                    self.links.iter().filter_map(|link| {
                        (scope.path.starts_with(&link.path) || link.path.starts_with(&scope.path))
                            .then_some(&link.path)
                    }),
                )
            })
    }

    fn input_paths(&self) -> impl Iterator<Item = &PathBuf> {
        self.inputs
            .iter()
            .map(|input| &input.path)
            .chain(self.scopes.iter().map(|scope| &scope.path))
    }

    pub fn verify(&self, own_classes: &[PathBuf]) -> Result<()> {
        let outputs = ClassOutputs::new(own_classes)?;
        let mut current = self.current_links()?;
        for input in &self.inputs {
            verify_input(input)?;
        }
        for scope in &self.scopes {
            current.extend(scope.verify(&outputs)?);
        }
        let expected = self
            .links
            .iter()
            .map(|link| (link.path.clone(), link.target.clone()))
            .collect();
        same_map(&expected, &current, "required link target changed")
    }

    fn current_links(&self) -> Result<BTreeMap<PathBuf, PathBuf>> {
        let mut current = BTreeMap::new();
        for path in self.paths() {
            add_links(&mut current, path)?;
        }
        Ok(current)
    }

    pub fn require_same_consumed(&self, after: &Self) -> Result<()> {
        let before = self.consumed();
        let after = after.consumed();
        same_map(
            &before.inputs,
            &after.inputs,
            "compiler input content or kind changed",
        )?;
        same_map(&before.links, &after.links, "required link target changed")?;
        for (key, members) in &before.scopes {
            let Some(current) = after.scopes.get(key) else {
                return Err(changed(&key.0, "compiler input scope changed"));
            };
            if let Some(path) = members.symmetric_difference(current).next() {
                return Err(changed(path, "compiler input membership changed"));
            }
        }
        for key in after.scopes.keys() {
            if !before.scopes.contains_key(key) {
                return Err(changed(&key.0, "compiler input scope changed"));
            }
        }
        Ok(())
    }

    pub fn consumed_key(&self) -> String {
        blake3::hash(format!("{:?}", self.consumed()).as_bytes())
            .to_hex()
            .to_string()
    }

    fn consumed(&self) -> Consumed {
        let inputs: BTreeMap<_, _> = self
            .inputs
            .iter()
            .map(|input| (input.path.clone(), input.clone()))
            .collect();
        let scopes = self
            .scopes
            .iter()
            .map(|scope| {
                (
                    (scope.path.clone(), scope.selection),
                    scope.members.iter().cloned().collect(),
                )
            })
            .collect();
        let available = self
            .links
            .iter()
            .map(|link| (link.path.clone(), link.target.clone()))
            .collect::<BTreeMap<_, _>>();
        let links = consumed_links(self.input_paths(), &available);
        Consumed {
            inputs,
            scopes,
            links,
        }
    }
}

#[derive(Debug)]
struct Consumed {
    inputs: BTreeMap<PathBuf, Input>,
    scopes: BTreeMap<(PathBuf, Selection), BTreeSet<PathBuf>>,
    links: BTreeMap<PathBuf, PathBuf>,
}

/// Compare the links that resolved consumed paths. A producer can legitimately
/// change which empty nested directories are excluded from a compiler's walk.
/// Full postcompile validation separately preserves the entire observed topology.
fn consumed_links<'a>(
    paths: impl Iterator<Item = &'a PathBuf>,
    available: &BTreeMap<PathBuf, PathBuf>,
) -> BTreeMap<PathBuf, PathBuf> {
    let mut pending = paths
        .flat_map(|path| path.ancestors().map(Path::to_path_buf))
        .collect::<Vec<_>>();
    let mut visited = BTreeSet::new();
    let mut links = BTreeMap::new();
    while let Some(path) = pending.pop() {
        if !visited.insert(path.clone()) {
            continue;
        }
        let Some(target) = available.get(&path) else {
            continue;
        };
        links.insert(path.clone(), target.clone());
        let resolved = if target.is_absolute() {
            target.clone()
        } else {
            path.parent().unwrap_or(Path::new("/")).join(target)
        };
        pending.extend(resolved.ancestors().map(Path::to_path_buf));
    }
    links
}

fn verify_input(input: &Input) -> Result<()> {
    if input_kind(&input.path)? != input.kind {
        return Err(changed(&input.path, "compiler input kind changed"));
    }
    match &input.sha256 {
        Some(expected) if &sha256(&input.path)? != expected => {
            Err(changed(&input.path, "compiler input content changed"))
        }
        _ => Ok(()),
    }
}

fn input_kind(path: &Path) -> Result<InputKind> {
    Ok(match metadata(path)? {
        None => InputKind::Missing,
        Some(ref metadata) if metadata.is_file() => InputKind::File,
        Some(ref metadata) if metadata.is_dir() => InputKind::Directory,
        Some(_) => anyhow::bail!("Unsupported nonregular compiler input: {}", path.display()),
    })
}

fn metadata(path: &Path) -> Result<Option<fs::Metadata>> {
    Ok(match fs::metadata(path) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => {
            return Err(error)
                .with_context(|| format!("Cannot inspect compiler input {}", path.display()));
        }
    })
}

fn sha256(path: &Path) -> Result<String> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let mut file = options.open(path).map_err(|error| {
        if error.kind() == ErrorKind::NotFound {
            changed(path, "compiler input disappeared")
        } else {
            anyhow::Error::new(error)
                .context(format!("Cannot open compiler input {}", path.display()))
        }
    })?;
    ensure!(
        file.metadata()?.is_file(),
        "Compiler input became nonregular: {}",
        path.display()
    );
    let mut digest = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let count = file
            .read(&mut buffer)
            .with_context(|| format!("Cannot hash compiler input {}", path.display()))?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

impl Scope {
    fn verify(&self, outputs: &ClassOutputs<'_>) -> Result<BTreeMap<PathBuf, PathBuf>> {
        let mut walk = ScopeWalk {
            selection: self.selection,
            outputs,
            ancestors: BTreeSet::new(),
            members: BTreeSet::new(),
            links: BTreeMap::new(),
        };
        walk.visit(&self.path)?;
        let expected = self.members.iter().cloned().collect::<BTreeSet<_>>();
        if let Some(path) = expected.symmetric_difference(&walk.members).next() {
            return Err(changed(path, "compiler input membership changed"));
        }
        Ok(walk.links)
    }
}

struct ScopeWalk<'a, 'b> {
    selection: Selection,
    outputs: &'a ClassOutputs<'b>,
    ancestors: BTreeSet<PathBuf>,
    members: BTreeSet<PathBuf>,
    links: BTreeMap<PathBuf, PathBuf>,
}

impl ScopeWalk<'_, '_> {
    fn visit(&mut self, path: &Path) -> Result<()> {
        if self.selection == Selection::Class && self.outputs.excludes(path)? {
            return Ok(());
        }
        match metadata(path)? {
            None => Ok(()),
            Some(kind) if kind.is_dir() => self.directory(path),
            Some(kind) => self.file(path, &kind),
        }
    }

    fn file(&mut self, path: &Path, metadata: &fs::Metadata) -> Result<()> {
        if !selected(path, self.selection) {
            return Ok(());
        }
        ensure!(
            metadata.is_file(),
            "Unsupported nonregular compiler input: {}",
            path.display()
        );
        add_links(&mut self.links, path)?;
        self.members.insert(path.to_path_buf());
        Ok(())
    }

    fn directory(&mut self, path: &Path) -> Result<()> {
        add_links(&mut self.links, path)?;
        let resolved = path
            .canonicalize()
            .with_context(|| format!("Cannot resolve compiler scope {}", path.display()))?;
        ensure!(
            self.ancestors.insert(resolved.clone()),
            "Cyclic compiler scope: {}",
            path.display()
        );
        let result = self.children(path);
        self.ancestors.remove(&resolved);
        result
    }

    fn children(&mut self, path: &Path) -> Result<()> {
        for entry in fs::read_dir(path)
            .with_context(|| format!("Cannot read compiler scope {}", path.display()))?
        {
            self.visit(&entry?.path())?;
        }
        Ok(())
    }
}

fn add_links(current: &mut BTreeMap<PathBuf, PathBuf>, path: &Path) -> Result<()> {
    for link in fingerprint::required_link_bindings(path)? {
        current.insert(link.path, link.target);
    }
    Ok(())
}

struct ClassOutputs<'a> {
    lexical: &'a [PathBuf],
    physical: Vec<PathBuf>,
}

impl<'a> ClassOutputs<'a> {
    fn new(lexical: &'a [PathBuf]) -> Result<Self> {
        let physical = lexical
            .iter()
            .map(|path| resolved_or_missing(path))
            .collect::<Result<_>>()?;
        Ok(Self { lexical, physical })
    }

    fn excludes(&self, path: &Path) -> Result<bool> {
        if self.lexical.iter().any(|root| path.starts_with(root)) {
            return Ok(true);
        }
        let physical = resolved_or_missing(path)?;
        Ok(self.physical.iter().any(|root| physical.starts_with(root)))
    }
}

/// Match the JDK's canonical exclusion paths while retaining missing probes.
/// Resolve the existing prefix before folding `..`, so links keep OS semantics.
fn resolved_or_missing(path: &Path) -> Result<PathBuf> {
    let mut existing = path.to_path_buf();
    let mut tail = Vec::new();
    loop {
        match existing.canonicalize() {
            Ok(mut resolved) => {
                for part in tail.into_iter().rev() {
                    append_component(&mut resolved, part);
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {
                let part = existing
                    .components()
                    .next_back()
                    .context("Missing compiler path has no existing ancestor")?
                    .as_os_str()
                    .to_os_string();
                ensure!(
                    existing.pop(),
                    "Cannot resolve compiler path {}",
                    path.display()
                );
                tail.push(part);
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("Cannot resolve compiler exclusion {}", path.display())
                });
            }
        }
    }
}

fn append_component(path: &mut PathBuf, part: std::ffi::OsString) {
    if part == ".." {
        path.pop();
    } else if part != "." {
        path.push(part);
    }
}

fn selected(path: &Path, selection: Selection) -> bool {
    path.extension().is_some_and(|extension| {
        extension
            == match selection {
                Selection::Java => "java",
                Selection::Class => "class",
            }
    })
}

fn same_map<T: PartialEq>(
    before: &BTreeMap<PathBuf, T>,
    after: &BTreeMap<PathBuf, T>,
    reason: &'static str,
) -> Result<()> {
    for path in before.keys().chain(after.keys()) {
        if before.get(path) != after.get(path) {
            return Err(changed(path, reason));
        }
    }
    Ok(())
}

pub(super) fn absolute(path: &Path) -> Result<()> {
    ensure!(
        path.is_absolute(),
        "Compiler observation path must be absolute: {}",
        path.display()
    );
    Ok(())
}

pub(super) fn changed(path: &Path, reason: &'static str) -> anyhow::Error {
    fingerprint::InputsChanged::new(
        "Project inputs changed during Java compilation.",
        path.to_string_lossy().into_owned(),
        reason,
    )
    .into()
}
