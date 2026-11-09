//! Context planning and staging; original paths remain the source of ownership.
use super::{
    JavaInput,
    build::JavaSourceSet,
    compiled_paths,
    metadata::{read_tsv, unescape},
};
use crate::{
    analyzers::{AnalysisJob, process::run_process},
    fingerprint,
    model::FileFingerprint,
};
use anyhow::{Context, Result, ensure};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(super) struct JavaContext {
    pub owner: JavaSourceSet,
    pub classpath: Vec<PathBuf>,
}

pub(super) struct StagedContext {
    pub inputs: Vec<JavaInput>,
    pub diagnostics: Vec<String>,
}

pub(super) fn plan(sources: &[JavaSourceSet]) -> Result<Vec<JavaContext>> {
    let mut contexts = Vec::new();
    let mut identities = HashSet::new();
    for source in sources {
        ensure!(
            identities.insert((&source.project_dir, &source.source_set)),
            "Duplicate Java source-set identity"
        );
        if !source
            .class_dirs
            .iter()
            .map(|path| super::contains_class(path))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .any(|value| value)
        {
            continue;
        }
        let classpath = ordered_classpath(source)?;
        contexts.push(JavaContext {
            owner: source.clone(),
            classpath,
        });
    }
    ensure!(
        !contexts.is_empty(),
        "Build emitted no Java class directories"
    );
    Ok(contexts)
}

fn ordered_classpath(source: &JavaSourceSet) -> Result<Vec<PathBuf>> {
    let mut seen = HashSet::new();
    let mut classpath = Vec::new();
    for path in source.class_dirs.iter().chain(&source.compile_classpath) {
        let path = evaluated_path(path)?;
        if seen.insert(path.clone()) {
            classpath.push(path);
        }
    }
    Ok(classpath)
}

fn evaluated_path(path: &Path) -> Result<PathBuf> {
    ensure!(
        path.is_absolute(),
        "Relative Java classpath entry: {}",
        path.display()
    );
    if path.exists() {
        return Ok(path.canonicalize()?);
    }
    ensure!(
        !path
            .extension()
            .is_some_and(|extension| extension == "jar" || extension == "zip"),
        "Java compile classpath entry is missing: {}",
        path.display()
    );
    Ok(path.into())
}

pub(super) fn snapshot(contexts: &[JavaContext]) -> Result<Vec<FileFingerprint>> {
    let mut paths = Vec::new();
    for path in contexts
        .iter()
        .flat_map(|context| &context.classpath)
        .collect::<HashSet<_>>()
    {
        if path.is_dir() {
            compiled_paths(path, &mut paths)?;
        } else {
            paths.push(path.clone());
        }
    }
    fingerprint::capture_excluding(paths, &[])
}

pub(super) fn stage(
    context: &JavaContext,
    sources: &[JavaSourceSet],
    root: &Path,
    job: &AnalysisJob<'_>,
) -> Result<StagedContext> {
    fs::create_dir_all(root)?;
    let java = job
        .runtime
        .java_home
        .as_ref()
        .context("Java analysis requires JDK 25")?;
    let ownership = ownership(sources)?;
    let existing = context
        .classpath
        .iter()
        .filter(|path| path.exists())
        .collect::<Vec<_>>();
    let projects = existing
        .iter()
        .enumerate()
        .filter(|(_, path)| ownership.contains_key(**path))
        .map(|(index, _)| (index + 1).to_string())
        .collect::<Vec<_>>()
        .join(",");
    run_process(
        Command::new(java.join("bin/java"))
            .arg("-Xmx2g")
            .arg("-cp")
            .arg(job.runtime.bytecode_lib()?.join("*"))
            .arg("codegraph.bytecode.ContextStager")
            .arg(root)
            .arg("--project-inputs")
            .arg(projects)
            .args(&existing),
        root,
        "staging",
        job.cancel,
    )?;
    Ok(StagedContext {
        inputs: staged_inputs(root, context, &ownership)?,
        diagnostics: shadow_diagnostics(root, &context.owner)?,
    })
}

fn staged_inputs(
    root: &Path,
    context: &JavaContext,
    ownership: &BTreeMap<PathBuf, JavaSourceSet>,
) -> Result<Vec<JavaInput>> {
    let mut inputs = Vec::new();
    for row in read_tsv(&root.join("inputs.tsv"))?.iter().skip(1) {
        ensure!(row.len() == 2, "Malformed staging input manifest");
        let original = PathBuf::from(unescape(&row[0])).canonicalize()?;
        ensure!(
            context.classpath.contains(&original),
            "Staging emitted an unknown original input"
        );
        let path = PathBuf::from(unescape(&row[1]));
        ensure!(
            path.starts_with(root) && path.is_dir(),
            "Invalid staged input directory"
        );
        let source = ownership
            .get(&original)
            .context("Staging emitted bytecode outside the selected project source sets")?;
        inputs.push(JavaInput {
            path,
            source_set: Some(source.clone()),
        });
    }
    Ok(inputs)
}

fn shadow_diagnostics(root: &Path, owner: &JavaSourceSet) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    for row in read_tsv(&root.join("shadows.tsv"))?.iter().skip(1) {
        ensure!(row.len() == 4, "Malformed shadowing manifest");
        diagnostics.push(format!(
            "Java context {} / {}: {} selects {}; shadows {} ({}).",
            owner.project,
            owner.source_set,
            unescape(&row[0]),
            unescape(&row[1]),
            unescape(&row[2]),
            if row[3] == "true" {
                "identical bytes"
            } else {
                "differing bytes"
            }
        ));
    }
    Ok(diagnostics)
}

fn ownership(sources: &[JavaSourceSet]) -> Result<BTreeMap<PathBuf, JavaSourceSet>> {
    let mut owners = BTreeMap::new();
    for source in sources {
        for path in source.class_dirs.iter().chain(&source.classpath_aliases) {
            if !path.exists() {
                continue;
            }
            let path = path.canonicalize()?;
            if let Some(previous) = owners.insert(path.clone(), source.clone()) {
                ensure!(
                    previous.project_dir == source.project_dir
                        && previous.source_set == source.source_set,
                    "Ambiguous Java artifact ownership: {}",
                    path.display()
                );
            }
        }
    }
    Ok(owners)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_compile_directory_becoming_populated_invalidates_snapshot() {
        let root = tempfile::tempdir().unwrap();
        let classes = root.path().join("classes");
        fs::create_dir(&classes).unwrap();
        fs::write(classes.join("Example.class"), b"fixture").unwrap();
        let absent = root.path().join("dependency-output");
        let source = JavaSourceSet {
            project: "fixture".into(),
            source_set: "main".into(),
            project_dir: root.path().into(),
            class_dirs: vec![classes],
            compile_classpath: vec![absent.clone()],
            classpath_aliases: Vec::new(),
            source_roots: Vec::new(),
            generated_roots: Vec::new(),
            build_inputs: Vec::new(),
            is_test: false,
        };
        let contexts = plan(&[source]).unwrap();
        let before = snapshot(&contexts).unwrap();
        assert!(!fingerprint::is_stale(&before).unwrap());
        fs::create_dir(&absent).unwrap();
        fs::write(absent.join("New.class"), b"new definition").unwrap();
        assert!(fingerprint::is_stale(&before).unwrap());
    }
}
