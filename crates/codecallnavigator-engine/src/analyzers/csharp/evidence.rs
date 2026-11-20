//! Convert evaluated MSBuild inputs and Roslyn-consumed files to engine evidence.
use crate::{
    analyzers::{
        CleanSnapshotCheckout, ensure_unchanged,
        evidence::{EvidenceInput, EvidenceRole},
    },
    fingerprint::{self, InputKind, InputsChanged},
    model::FileFingerprint,
};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
pub(super) struct CompilerEvidence {
    version: String,
    inputs: Vec<CompilerInput>,
}

#[derive(Deserialize)]
struct CompilerInput {
    kind: CompilerInputKind,
    path: PathBuf,
    sha256: Option<String>,
    #[serde(rename = "contentChanged")]
    content_changed: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum CompilerInputKind {
    Source,
    Reference,
}

impl CompilerInputKind {
    fn description(&self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Reference => "assembly reference",
        }
    }
}

pub(super) struct Captured {
    pub(super) inputs: Vec<EvidenceInput>,
    pub(super) attribution_limit: Option<String>,
    compiler_files: Vec<FileFingerprint>,
}

impl Captured {
    pub(super) fn new(
        compiler: CompilerEvidence,
        evaluated: &[FileFingerprint],
        roles: Roles<'_>,
    ) -> Result<Self> {
        ensure!(
            compiler.version == "1",
            "CallAtlas compiler evidence version is unsupported; run tools/bootstrap.py"
        );
        let mut inputs = evaluated
            .iter()
            .map(|input| roles.classify(input.clone()))
            .collect::<Vec<_>>();
        let mut gaps = BTreeSet::new();
        let mut compiler_files = Vec::new();
        for input in compiler.inputs {
            if let Some(file) = input.capture()? {
                inputs.push(roles.classify(file.clone()));
                compiler_files.push(file);
            } else {
                gaps.insert(format!(
                    "CallAtlas could not establish the origin of compiler {} {}",
                    input.kind.description(),
                    input.path.display()
                ));
            }
        }
        let attribution_limit = gaps.into_iter().next();
        let captured = Self {
            inputs,
            attribution_limit,
            compiler_files,
        };
        captured.verify()?;
        Ok(captured)
    }

    pub(super) fn verify(&self) -> Result<()> {
        ensure_unchanged(&self.compiler_files)
    }
}

impl CompilerInput {
    fn capture(&self) -> Result<Option<FileFingerprint>> {
        if self.content_changed {
            return Err(self.changed().into());
        }
        let Some(expected) = self.sha256.as_deref() else {
            return Ok(None);
        };
        ensure!(
            self.path.is_absolute() && is_sha256(expected),
            "Invalid CallAtlas compiler input evidence for {}",
            self.path.display()
        );
        let fingerprints = fingerprint::capture_excluding([self.path.clone()], &[])?;
        let Some(input) = fingerprints.into_iter().next() else {
            return Ok(None);
        };
        self.verify_file(&input, expected)?;
        Ok(Some(input))
    }

    fn verify_file(&self, input: &FileFingerprint, expected: &str) -> Result<()> {
        if input.kind != InputKind::File {
            return Err(self.changed().into());
        }
        let bytes = fs::read(&self.path).with_context(|| {
            format!(
                "Cannot verify CallAtlas compiler input {}",
                self.path.display()
            )
        })?;
        if format!("{:x}", Sha256::digest(&bytes)) != expected {
            return Err(self.changed().into());
        }
        ensure_unchanged(std::slice::from_ref(input))
    }

    fn changed(&self) -> InputsChanged {
        InputsChanged::new(
            "C# compiler input changed before its evidence could be captured",
            self.path.to_string_lossy().into_owned(),
            "content changed",
        )
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) struct Roles<'a> {
    generated_sources: &'a BTreeSet<PathBuf>,
    restore_products: &'a BTreeSet<PathBuf>,
    checkout: Option<&'a CleanSnapshotCheckout>,
    managed: BTreeSet<PathBuf>,
}

impl<'a> Roles<'a> {
    pub(super) fn new(
        dotnet: &Path,
        checkout: Option<&'a CleanSnapshotCheckout>,
        generated_sources: &'a BTreeSet<PathBuf>,
        restore_products: &'a BTreeSet<PathBuf>,
    ) -> Self {
        Self {
            generated_sources,
            restore_products,
            checkout,
            managed: managed_roots(dotnet),
        }
    }

    fn classify(&self, fingerprint: FileFingerprint) -> EvidenceInput {
        let path = Path::new(&fingerprint.path);
        let restore_product = self.restore_products.contains(path);
        let generated_here = fingerprint.kind == InputKind::File
            && (restore_product || self.generated_sources.contains(path))
            && self
                .checkout
                .is_some_and(|checkout| checkout.owns_generated_root(path));
        let role = if generated_here {
            EvidenceRole::Generated
        } else if restore_product || self.managed.iter().any(|root| path.starts_with(root)) {
            EvidenceRole::Dependency
        } else {
            EvidenceRole::Revision
        };
        EvidenceInput { fingerprint, role }
    }
}

pub(super) fn restore_products(project: &Path, metadata: &Value) -> impl Iterator<Item = PathBuf> {
    let project_name = project.file_name().unwrap_or_default().to_string_lossy();
    let property = |name| evaluated_path(project, metadata, name);
    let extensions = property("MSBuildProjectExtensionsPath");
    let imports = [".nuget.g.props", ".nuget.g.targets"]
        .into_iter()
        .filter_map(move |suffix| {
            Some(extensions.as_ref()?.join(format!("{project_name}{suffix}")))
        });
    property("ProjectAssetsFile").into_iter().chain(imports)
}

pub(super) fn generated_sources(project: &Path, metadata: &Value) -> impl Iterator<Item = PathBuf> {
    [
        "GeneratedAssemblyInfoFile",
        "GeneratedGlobalUsingsFile",
        "TargetFrameworkMonikerAssemblyAttributesPath",
    ]
    .into_iter()
    .filter_map(|name| evaluated_path(project, metadata, name))
}

fn evaluated_path(project: &Path, metadata: &Value, name: &str) -> Option<PathBuf> {
    let value = metadata["Properties"][name]
        .as_str()
        .filter(|path| !path.is_empty())?;
    // MSBuild can retain Windows separators in otherwise native evaluated paths.
    Some(project.parent()?.join(value.replace('\\', "/")))
}

fn managed_roots(dotnet: &Path) -> BTreeSet<PathBuf> {
    let home = env::var_os("HOME").map(PathBuf::from);
    let packages = env::var_os("NUGET_PACKAGES")
        .map(PathBuf::from)
        .or_else(|| home.as_ref().map(|home| home.join(".nuget/packages")));
    let dotnet_path = dotnet
        .canonicalize()
        .ok()
        .and_then(|tool| tool.parent().map(Path::to_path_buf));
    let sdk = env::var_os("DOTNET_ROOT")
        .map(PathBuf::from)
        .into_iter()
        .chain(dotnet_path)
        .filter(|path| path.join("sdk").is_dir());
    let msbuild = env::var_os("MSBUILD_USER_EXTENSIONS_PATH")
        .or_else(|| env::var_os("MSBuildUserExtensionsPath"))
        .map(PathBuf::from)
        .or_else(|| default_msbuild_user_extensions(home.as_deref()));
    packages
        .into_iter()
        .chain(sdk)
        .chain(msbuild)
        .filter(|path| path.is_absolute())
        .flat_map(|path| path.canonicalize().ok().into_iter().chain([path]))
        .collect()
}

fn default_msbuild_user_extensions(home: Option<&Path>) -> Option<PathBuf> {
    let data = if cfg!(target_os = "macos") {
        home?.join("Library/Application Support")
    } else if let Some(data) = env::var_os("XDG_DATA_HOME") {
        data.into()
    } else {
        home?.join(".local/share")
    };
    Some(data.join("Microsoft/MSBuild"))
}
