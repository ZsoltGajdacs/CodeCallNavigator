//! Evaluated C# import inventories; MSBuildAllProjects is not an import list.
//!
//! SDK 11 preprocessing writes framed XML comments for actual imports, including
//! restore-generated obj files. Unknown/ambiguous framing fails closed. Missing
//! conditional imports are not evaluated inputs and are not resolution probes.
mod contexts;
use super::super::{ensure_unchanged, process::run_process, source_snapshot};
use crate::model::FileFingerprint;
use anyhow::{Context, Result, ensure};
use contexts::EvaluationContext;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
};

pub(super) struct Preprocessor<'a> {
    dotnet: &'a Path,
    job: &'a Path,
    cancel: &'a AtomicBool,
    solution_dir: Option<PathBuf>,
}

pub(super) struct Snapshot {
    project: PathBuf,
    sequence: usize,
    inventory: Evaluation,
    pub paths: Vec<PathBuf>,
    fingerprints: Vec<FileFingerprint>,
}

#[derive(PartialEq, Eq)]
struct Evaluation {
    contexts: Vec<EvaluationContext>,
    imports: BTreeMap<EvaluationContext, Vec<PathBuf>>,
}

impl Evaluation {
    fn input_paths(&self) -> Vec<PathBuf> {
        self.imports
            .values()
            .flatten()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

impl Snapshot {
    pub fn sequence(&self) -> usize {
        self.sequence
    }

    pub fn contexts(&self) -> &[EvaluationContext] {
        &self.inventory.contexts
    }

    pub fn verify(&self) -> Result<()> {
        ensure_unchanged(&self.fingerprints)
    }
}

impl<'a> Preprocessor<'a> {
    pub fn new(dotnet: &'a Path, job: &'a Path, cancel: &'a AtomicBool) -> Self {
        Self {
            dotnet,
            job,
            cancel,
            solution_dir: None,
        }
    }

    pub fn for_build(
        dotnet: &'a Path,
        job: &'a Path,
        cancel: &'a AtomicBool,
        build: &crate::discovery::BuildTarget,
    ) -> Self {
        let mut preprocessor = Self::new(dotnet, job, cancel);
        if build.kind == "solution" {
            preprocessor.solution_dir = build.path.parent().map(Path::to_path_buf);
        }
        preprocessor
    }

    pub fn capture(&self, project: &Path, sequence: usize) -> Result<Snapshot> {
        let inventory = self.inventory(project, &format!("{sequence}-discover"))?;
        let mut paths = inventory.input_paths();
        paths.push(
            project
                .parent()
                .context("C# project has no input scope")?
                .into(),
        );
        // Scope membership plus explicit file roots retain imported obj/SDK files.
        let fingerprints = source_snapshot(&paths)?;
        let repeated = self.inventory(project, &format!("{sequence}-verify"))?;
        ensure!(
            inventory == repeated,
            "C# evaluated import inventory changed during analysis"
        );
        ensure_unchanged(&fingerprints)?;
        Ok(Snapshot {
            project: project.into(),
            sequence,
            inventory,
            paths,
            fingerprints,
        })
    }

    pub fn verify(&self, snapshot: &Snapshot) -> Result<()> {
        snapshot.verify()?;
        let repeated =
            self.inventory(&snapshot.project, &format!("{}-recheck", snapshot.sequence))?;
        ensure!(
            snapshot.inventory == repeated,
            "C# evaluated import inventory changed during analysis"
        );
        snapshot.verify()
    }

    fn command(&self, project: &Path, context: &EvaluationContext) -> Command {
        let mut command = Command::new(self.dotnet);
        command.arg("msbuild").arg(project).arg("-nologo");
        context.apply(&mut command);
        if let Some(directory) = &self.solution_dir {
            contexts::literal_property(
                &mut command,
                "SolutionDir",
                &format!("{}/", directory.display()),
            );
        }
        command
    }

    pub fn metadata(
        &self,
        project: &Path,
        context: &EvaluationContext,
        phase: &str,
    ) -> Result<serde_json::Value> {
        let mut command = self.command(project, context);
        command.args([
            "-getItem:Compile,ProjectReference,AdditionalFiles",
            "-getProperty:MSBuildProjectFullPath,IsTestProject,AssemblyName",
        ]);
        self.query(command, phase)
    }

    fn query(&self, mut command: Command, phase: &str) -> Result<serde_json::Value> {
        let output = run_process(&mut command, self.job, phase, self.cancel)?;
        serde_json::from_slice(&fs::read(output)?).context("Invalid C# evaluated metadata")
    }

    fn evaluation_contexts(&self, project: &Path, phase: &str) -> Result<Vec<EvaluationContext>> {
        let mut contexts = Vec::new();
        for (index, context) in EvaluationContext::roots().into_iter().enumerate() {
            let mut command = self.command(project, &context);
            command.arg("-getProperty:TargetFramework,TargetFrameworks");
            let metadata = self.query(command, &format!("frameworks-{phase}-{index}"))?;
            let inner = context.inner_builds(&metadata)?;
            // Ownership authority is the configured normal root, evaluated first.
            contexts.push(context);
            contexts.extend(inner);
        }
        Ok(contexts)
    }

    fn inventory(&self, project: &Path, phase: &str) -> Result<Evaluation> {
        let contexts = self.evaluation_contexts(project, phase)?;
        let mut imports = BTreeMap::new();
        for (index, context) in contexts.iter().enumerate() {
            imports.insert(
                context.clone(),
                self.preprocess(project, context, &format!("{phase}-{index}"))?,
            );
        }
        Ok(Evaluation { contexts, imports })
    }

    fn preprocess(
        &self,
        project: &Path,
        context: &EvaluationContext,
        phase: &str,
    ) -> Result<Vec<PathBuf>> {
        let output = self.job.join(format!("imports-{phase}.xml"));
        // A successful invocation must create fresh output; never accept a prior pass.
        match fs::remove_file(&output) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("Cannot remove prior C# preprocess output"),
        }
        let mut command = self.command(project, context);
        command.arg(format!("-preprocess:{}", output.display()));
        run_process(
            &mut command,
            self.job,
            &format!("imports-{phase}"),
            self.cancel,
        )?;
        let text =
            fs::read_to_string(&output).context("Cannot read fresh C# preprocessed imports")?;
        inventory(&text, project)
            .with_context(|| format!("Unsupported C# import inventory for {}", project.display()))
    }
}

enum Frame<'a> {
    Root(&'a str),
    Begin(&'a str),
    End(Option<&'a str>),
}

fn separator(line: &str) -> bool {
    line.len() == 140 && line.bytes().all(|byte| byte == b'=')
}

fn frame(comment: &str) -> Result<Option<Frame<'_>>> {
    let lines = comment.trim().lines().collect::<Vec<_>>();
    let Some(first) = lines.first() else {
        return Ok(None);
    };
    if !separator(first.trim()) {
        return Ok(None);
    }
    ensure!(
        lines.len() >= 3 && separator(lines.last().unwrap().trim()),
        "Malformed MSBuild import frame"
    );
    if lines.len() == 3 {
        return Ok(Some(if lines[1].trim() == "</Import>" {
            Frame::End(None)
        } else {
            Frame::Root(lines[1])
        }));
    }
    Ok(Some(import_frame(&lines)?))
}

fn import_frame<'a>(lines: &[&'a str]) -> Result<Frame<'a>> {
    ensure!(
        lines[lines.len() - 3].trim().is_empty(),
        "MSBuild import frame lacks path boundary"
    );
    let path = lines[lines.len() - 2];
    let header = &lines[1..lines.len() - 3];
    if header.len() == 1 && header[0].trim() == "</Import>" {
        return Ok(Frame::End(Some(path)));
    }
    ensure!(
        header
            .first()
            .is_some_and(|line| line.trim().starts_with("<Import ") && line.trim().ends_with('>')),
        "Unknown MSBuild import frame header"
    );
    ensure!(
        header[1..].iter().all(|line| line
            .trim()
            .starts_with("This import was added implicitly because ")),
        "Unknown MSBuild import explanation"
    );
    Ok(Frame::Begin(path))
}

fn inventory(text: &str, project: &Path) -> Result<Vec<PathBuf>> {
    let document = roxmltree::Document::parse(text).context("Invalid MSBuild preprocessed XML")?;
    let project = project.canonicalize()?;
    ensure!(
        document.root_element().tag_name().name() == "Project",
        "Unknown MSBuild preprocessed root element"
    );
    let header = document
        .root()
        .children()
        .find(|node| node.is_comment())
        .context("Missing MSBuild root header")?;
    ensure!(
        matches!(frame(header.text().unwrap_or_default())?, Some(Frame::Root(path)) if path == escaped(&project)),
        "Missing or mismatched MSBuild root header"
    );
    let mut stack = Vec::new();
    let mut paths = BTreeSet::new();
    for node in document.descendants().filter(|node| node.is_comment()) {
        let Some(frame) = frame(node.text().unwrap_or_default())? else {
            continue;
        };
        apply_frame(frame, &project, &mut stack, &mut paths)?;
    }
    ensure!(
        stack == [project],
        "Missing root or unclosed MSBuild import frame"
    );
    Ok(paths.into_iter().collect())
}

fn apply_frame(
    frame: Frame<'_>,
    project: &Path,
    stack: &mut Vec<PathBuf>,
    paths: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    match frame {
        Frame::Root(path) => {
            ensure!(stack.is_empty(), "Repeated MSBuild root frame");
            ensure!(
                path == escaped(project),
                "MSBuild root frame does not identify selected project"
            );
            stack.push(project.into());
            paths.insert(project.into());
        }
        Frame::Begin(path) => {
            ensure!(!stack.is_empty(), "Import precedes MSBuild root frame");
            let path = resolve_path(path)?;
            paths.insert(path.clone());
            stack.push(path);
        }
        Frame::End(path) => {
            ensure!(stack.len() > 1, "Unmatched MSBuild import end frame");
            stack.pop();
            if let Some(path) = path {
                ensure!(
                    path == escaped(stack.last().unwrap()),
                    "MSBuild import parent frame mismatch"
                );
            }
        }
    }
    Ok(())
}

fn escaped(path: &Path) -> String {
    path.to_string_lossy().replace("--", "__")
}

fn resolve_path(value: &str) -> Result<PathBuf> {
    let path = Path::new(value);
    ensure!(
        path.is_absolute(),
        "MSBuild import frame path is not absolute"
    );
    let mut resolved = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(name) if name.to_string_lossy().contains("__") => {
                resolved = resolve_component(&resolved, &name.to_string_lossy())?;
            }
            component => resolved.push(component.as_os_str()),
        }
    }
    ensure!(
        resolved.is_file(),
        "MSBuild evaluated import is unavailable: {}",
        resolved.display()
    );
    // Retain the evaluated lexical alias: parent frames use that same spelling.
    Ok(resolved)
}

fn resolve_component(parent: &Path, name: &str) -> Result<PathBuf> {
    let candidates = fs::read_dir(parent)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .filter(|path| {
            path.file_name()
                .is_some_and(|value| value.to_string_lossy().replace("--", "__") == name)
        })
        .collect::<Vec<_>>();
    ensure!(
        candidates.len() == 1,
        "Ambiguous or missing escaped MSBuild path component {name}"
    );
    Ok(candidates.into_iter().next().unwrap())
}

#[cfg(test)]
mod tests;
