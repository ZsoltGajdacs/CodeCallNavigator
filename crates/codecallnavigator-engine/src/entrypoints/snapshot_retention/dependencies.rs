//! Bind observed compiler and managed package inputs to the analyzer identity.
use anyhow::Result;
use codecallnavigator_core::{
    discovery::BuildTarget, fingerprint::InputKind, model::FileFingerprint,
};
use codecallnavigator_engine::analyzers::AnalyzerRuntime;
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    path::{Path, PathBuf},
};

#[derive(Default)]
pub(super) struct Dependencies {
    files: BTreeSet<PathBuf>,
    roots: BTreeSet<PathBuf>,
    inputs: BTreeMap<PathBuf, String>,
    typescript: bool,
    csharp: bool,
}

impl Dependencies {
    pub(super) fn new(targets: &[BuildTarget]) -> Result<Self> {
        let has = |language| targets.iter().any(|target| target.language == language);
        let mut dependencies = Self {
            typescript: has("typescript"),
            csharp: has("csharp"),
            ..Self::default()
        };
        if dependencies.typescript {
            dependencies.typescript_policy()?;
        }
        if has("java") {
            dependencies.java_paths();
        }
        if dependencies.csharp {
            dependencies.dotnet_paths();
        }
        Ok(dependencies)
    }

    fn typescript_policy(&mut self) -> Result<()> {
        let resources = AnalyzerRuntime::discover()?.bundle_root;
        for location in [
            "analyzers/typescript/compatibility.json",
            "tools/typescript/compatibility.json",
        ] {
            register(&mut self.files, resources.join(location));
        }
        Ok(())
    }

    fn java_paths(&mut self) {
        self.maven_paths();
        if let Some(home) = user_directory("GRADLE_USER_HOME", ".gradle") {
            for name in ["gradle.properties", "init.gradle", "init.gradle.kts"] {
                register(&mut self.files, home.join(name));
            }
            for name in ["caches", "wrapper/dists"] {
                register(&mut self.roots, home.join(name));
            }
        }
    }

    fn maven_paths(&mut self) {
        let installation = env::var_os("MAVEN_HOME").map(PathBuf::from).or_else(|| {
            command_directory("mvn").and_then(|bin| bin.parent().map(Path::to_path_buf))
        });
        let home = user_directory("MAVEN_USER_HOME", ".m2");
        let system_configs = installation
            .iter()
            .flat_map(|root| [root.join("conf"), root.join("libexec/conf")]);
        let configs = home.iter().cloned().chain(system_configs);
        for path in configs.flat_map(|dir| [dir.join("settings.xml"), dir.join("toolchains.xml")]) {
            register(&mut self.files, path);
        }
        if let Some(home) = home {
            register(&mut self.roots, home.join("repository"));
            register(&mut self.roots, home.join("wrapper/dists"));
        }
    }

    fn dotnet_paths(&mut self) {
        if let Some(packages) = user_directory("NUGET_PACKAGES", ".nuget/packages") {
            register(&mut self.roots, packages);
        }
        if let Some(root) = env::var_os("DOTNET_ROOT")
            .map(PathBuf::from)
            .or_else(|| command_directory("dotnet"))
            && root.join("sdk").is_dir()
        {
            register(&mut self.roots, root);
        }
        if let Some(path) = msbuild_user_extensions() {
            register(&mut self.roots, path);
        }
    }

    pub(super) fn observe(&mut self, input: &FileFingerprint, relative: Option<&Path>) -> bool {
        let path = Path::new(&input.path);
        if !self.managed(path, input.kind, relative) {
            return false;
        }
        self.record(relative.unwrap_or(path), input);
        true
    }

    pub(super) fn record(&mut self, path: &Path, input: &FileFingerprint) {
        if input.kind != InputKind::Missing {
            self.inputs.insert(path.into(), input.hash.clone());
        }
    }

    fn managed(&self, path: &Path, kind: InputKind, relative: Option<&Path>) -> bool {
        self.files.contains(path)
            || self.roots.iter().any(|root| path.starts_with(root))
            || (self.typescript && typescript_package(path, kind, relative))
            || (self.csharp && relative.is_some_and(nuget_generated_import))
    }

    pub(super) fn digest(&self) -> Result<String> {
        Ok(blake3::hash(&serde_json::to_vec(&self.inputs)?)
            .to_hex()
            .to_string())
    }
}

fn msbuild_user_extensions() -> Option<PathBuf> {
    if let Some(path) = env::var_os("MSBuildUserExtensionsPath") {
        return Some(path.into());
    }
    let home = PathBuf::from(env::var_os("HOME")?);
    let data = if cfg!(target_os = "macos") {
        home.join("Library/Application Support")
    } else {
        env::var_os("XDG_DATA_HOME").map_or_else(|| home.join(".local/share"), PathBuf::from)
    };
    Some(data.join("Microsoft/MSBuild"))
}

fn register(paths: &mut BTreeSet<PathBuf>, path: PathBuf) {
    if !path.is_absolute() {
        return;
    }
    if let Ok(canonical) = path.canonicalize() {
        paths.insert(canonical);
    }
    paths.insert(path);
}

fn user_directory(variable: &str, default: &str) -> Option<PathBuf> {
    env::var_os(variable)
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(default)))
}

fn command_directory(command: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|directory| directory.join(command))
        .find_map(|binary| binary.canonicalize().ok().filter(|path| path.is_file()))?
        .parent()
        .map(Path::to_path_buf)
}

fn typescript_package(path: &Path, kind: InputKind, relative: Option<&Path>) -> bool {
    // Missing ancestor probes cannot supply code and their absolute paths can
    // vary by worktree. Existing observations must come from installed packages.
    (kind == InputKind::Missing && relative.is_none())
        || path.components().any(|part| {
            matches!(
                part.as_os_str().to_str(),
                Some("node_modules" | ".node_modules")
            )
        })
}

fn nuget_generated_import(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    path.parent()
        .and_then(Path::file_name)
        .is_some_and(|name| name == "obj")
        && (name.ends_with(".nuget.g.props")
            || name.ends_with(".nuget.g.targets")
            || name == "project.assets.json")
}
