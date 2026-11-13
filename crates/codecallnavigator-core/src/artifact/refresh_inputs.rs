//! Exact compiler-output ownership and saved directory membership for scheduling.
//! Full analysis input verification never excludes these outputs.
use super::{
    Roots,
    paths::Paths,
    provenance::{Analysis, Input},
};
use crate::{
    fingerprint::{self, InputKind},
    model::FileFingerprint,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

const BINDING_POLICY: &str = "direct.v1";

#[derive(Serialize, Deserialize)]
struct Metadata {
    version: u32,
    complete: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    binding_policy: Option<String>,
    owned_bytecode_directories: Vec<String>,
    directories: Vec<Input>,
}

pub struct Captured {
    pub value: Value,
    pub validation: Vec<FileFingerprint>,
}

/// Inputs already carry compiler fingerprints. Capture only directory overrides
/// whose direct child is an owned output, retaining all sibling input membership.
pub fn capture(
    analysis: &Analysis,
    outputs: &crate::model::CompilerOutputs,
    safe: bool,
    roots: &Roots,
) -> Result<Captured> {
    let paths = Paths::new(roots)?;
    let (owned, mapped) = portable_outputs(outputs, &paths);
    let mut complete = safe && mapped;
    complete &= unambiguous(outputs);
    complete &= certified_bindings(outputs, roots);
    complete &= only_bytecode_inputs(&analysis.inputs, &owned);
    let (directories, validation) = directory_overrides(&analysis.inputs, &owned, &paths)?;
    let metadata = Metadata {
        version: 1,
        complete,
        binding_policy: Some(BINDING_POLICY.into()),
        owned_bytecode_directories: owned,
        directories,
    };
    Ok(Captured {
        value: serde_json::to_value(metadata)?,
        validation,
    })
}

fn portable_outputs(outputs: &crate::model::CompilerOutputs, paths: &Paths) -> (Vec<String>, bool) {
    let mapped = outputs
        .bytecode_directories
        .iter()
        .map(|output| paths.portable(output))
        .collect::<Vec<_>>();
    let complete = mapped.iter().all(Result::is_ok);
    let owned = mapped
        .into_iter()
        .filter_map(Result::ok)
        .collect::<BTreeSet<_>>();
    (owned.into_iter().collect(), complete)
}

fn only_bytecode_inputs(inputs: &[Input], owned: &[String]) -> bool {
    inputs.iter().all(|input| {
        if input.kind == InputKind::Directory {
            return true;
        }
        if !owned.iter().any(|output| beneath(&input.path, output)) {
            return true;
        }
        Path::new(&input.path)
            .extension()
            .is_some_and(|extension| extension == "class")
    })
}

fn directory_overrides(
    inputs: &[Input],
    owned: &[String],
    paths: &Paths,
) -> Result<(Vec<Input>, Vec<FileFingerprint>)> {
    let mut directories = Vec::new();
    let mut validation = Vec::new();
    for input in inputs
        .iter()
        .filter(|input| input.kind == InputKind::Directory)
    {
        if owned.iter().any(|output| beneath(&input.path, output)) {
            continue;
        }
        if let Some((directory, captured)) = directory_override(input, owned, paths)? {
            directories.push(directory);
            validation.extend(captured);
        }
    }
    directories.sort_by(|a, b| {
        (&a.path, &a.excluded_children, &a.hash).cmp(&(&b.path, &b.excluded_children, &b.hash))
    });
    directories.dedup();
    Ok((directories, validation))
}

fn directory_override(
    input: &Input,
    owned: &[String],
    paths: &Paths,
) -> Result<Option<(Input, Vec<FileFingerprint>)>> {
    let excluded = exclusions(input, owned);
    if excluded == input.excluded_children {
        return Ok(None);
    }
    let local = PathBuf::from(paths.local(&input.path)?);
    let current = fingerprint::observe_content(&local, &input.excluded_children)?;
    ensure!(
        current.kind == input.kind && current.hash == input.hash,
        "Input membership changed before refresh policy capture"
    );
    let exclusions = excluded
        .iter()
        .map(|child| local.join(child))
        .collect::<Vec<_>>();
    let captured = fingerprint::capture_excluding(vec![local], &exclusions)?;
    let observed = captured
        .first()
        .context("Directory override was not captured")?;
    let directory = Input {
        path: input.path.clone(),
        kind: InputKind::Directory,
        hash: observed.hash.clone(),
        excluded_children: excluded,
    };
    let after = fingerprint::observe_content(
        Path::new(&paths.local(&input.path)?),
        &input.excluded_children,
    )?;
    ensure!(
        after.kind == input.kind && after.hash == input.hash,
        "Input membership changed during refresh policy capture"
    );
    Ok(Some((directory, captured)))
}

fn unambiguous(outputs: &crate::model::CompilerOutputs) -> bool {
    outputs.bytecode_directories.iter().all(|output| {
        let output = Path::new(output);
        let sources_separate = outputs
            .source_roots
            .iter()
            .all(|source| !output.starts_with(source) && !Path::new(source).starts_with(output));
        sources_separate
            && outputs
                .build_inputs
                .iter()
                .all(|input| !Path::new(input).starts_with(output))
            && foreign_classpath_separate(output, outputs)
    })
}

fn foreign_classpath_separate(output: &Path, outputs: &crate::model::CompilerOutputs) -> bool {
    outputs.compile_classpath.iter().all(|dependency| {
        if outputs.bytecode_directories.contains(dependency) {
            return true;
        }
        !output.starts_with(dependency) && !Path::new(dependency).starts_with(output)
    })
}

fn certified_bindings(outputs: &crate::model::CompilerOutputs, roots: &Roots) -> bool {
    let Some(raw) = &outputs.raw_bytecode_directories else {
        return false;
    };
    let raw_set = raw.iter().map(Path::new).collect::<BTreeSet<_>>();
    let canonical = outputs
        .bytecode_directories
        .iter()
        .map(Path::new)
        .collect::<BTreeSet<_>>();
    raw_set == canonical && raw_set.iter().all(|path| symlink_free(path, roots))
}

// The declared root itself may be a caller-selected mapping. No path component
// below that boundary may change an owned output into an alias of other inputs.
fn symlink_free(path: &Path, roots: &Roots) -> bool {
    let Some(root) = roots
        .iter()
        .map(|(_, root)| root)
        .filter(|root| path.starts_with(root))
        .max_by_key(|root| root.components().count())
    else {
        return false;
    };
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    let mut current = root.clone();
    for part in relative.components() {
        let std::path::Component::Normal(name) = part else {
            return false;
        };
        current.push(name);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_symlink() => return false,
            Ok(_) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => return false,
        }
    }
    true
}

pub(super) fn validate(value: &Value, analysis: &Analysis) -> Result<()> {
    if value.get("version").and_then(Value::as_u64) != Some(1) {
        return Ok(());
    }
    let metadata: Metadata =
        serde_json::from_value(value.clone()).context("Invalid refresh input metadata")?;
    validate_metadata(&metadata, analysis)
}

fn validate_metadata(metadata: &Metadata, analysis: &Analysis) -> Result<()> {
    for path in &metadata.owned_bytecode_directories {
        validate_path(path, &analysis.roots)?;
    }
    let mut overridden = BTreeSet::new();
    for directory in &metadata.directories {
        directory.validate()?;
        validate_path(&directory.path, &analysis.roots)?;
        ensure!(
            directory.kind == InputKind::Directory,
            "Refresh override must be directory membership"
        );
        ensure!(
            overridden.insert((&directory.path, &directory.excluded_children)),
            "Duplicate refresh directory override"
        );
        let original = analysis
            .inputs
            .iter()
            .find(|input| {
                input.path == directory.path
                    && input.kind == InputKind::Directory
                    && exclusions(input, &metadata.owned_bytecode_directories)
                        == directory.excluded_children
            })
            .context("Refresh directory override has no recorded input")?;
        let expected = exclusions(original, &metadata.owned_bytecode_directories);
        ensure!(
            directory.excluded_children == expected,
            "Refresh directory override excludes unowned inputs"
        );
    }
    Ok(())
}

fn validate_path(path: &str, roots: &[String]) -> Result<()> {
    super::paths::validate_field("file", &Value::String(path.into()))?;
    let root = path
        .strip_prefix('@')
        .and_then(|path| path.split_once('/'))
        .context("Invalid refresh input path")?
        .0;
    ensure!(
        roots.iter().any(|known| known == root),
        "Refresh input references undeclared root"
    );
    Ok(())
}

fn beneath(path: &str, root: &str) -> bool {
    Path::new(path).starts_with(root)
}

fn exclusions(input: &Input, outputs: &[String]) -> Vec<String> {
    let mut excluded = input.excluded_children.clone();
    for output in outputs {
        if Path::new(output).parent() == Some(Path::new(&input.path)) {
            excluded.push(
                Path::new(output)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    excluded.sort();
    excluded.dedup();
    excluded
}

/// Return a small filtered manifest; never mutate the canonical artifact or its
/// complete freshness report. Unknown policy versions require explicit refresh.
pub fn filtered(analysis: &Analysis) -> Result<Analysis> {
    let value = analysis.refresh_inputs.as_ref().context(
        "Java refresh requires recorded compiler-output ownership; run a manual refresh",
    )?;
    let metadata: Metadata =
        serde_json::from_value(value.clone()).context("Unsupported refresh input metadata")?;
    ensure!(
        metadata.version == 1
            && metadata.complete
            && metadata.binding_policy.as_deref() == Some(BINDING_POLICY),
        "Compiler-output ownership is incomplete or unsupported; use manual refresh"
    );
    validate_metadata(&metadata, analysis)?;
    let mut filtered = analysis.clone();
    filtered.inputs.retain(|input| {
        !metadata
            .owned_bytecode_directories
            .iter()
            .any(|output| beneath(&input.path, output))
    });
    for input in &mut filtered.inputs {
        let expected = exclusions(input, &metadata.owned_bytecode_directories);
        if expected == input.excluded_children {
            continue;
        }
        *input = metadata
            .directories
            .iter()
            .find(|directory| {
                directory.path == input.path && directory.excluded_children == expected
            })
            .context("Missing saved refresh directory membership")?
            .clone();
    }
    Ok(filtered)
}

pub(super) fn normalize(value: &mut Value) {
    if value.get("version").and_then(Value::as_u64) != Some(1) {
        return;
    }
    for field in ["owned_bytecode_directories", "directories"] {
        if let Some(items) = value.get_mut(field).and_then(Value::as_array_mut) {
            items.sort_by_cached_key(Value::to_string);
        }
    }
}

pub fn owned_outputs(analysis: &Analysis, roots: &Roots) -> Result<Vec<PathBuf>> {
    let value = analysis
        .refresh_inputs
        .as_ref()
        .context("Missing compiler-output ownership")?;
    let metadata: Metadata = serde_json::from_value(value.clone())?;
    ensure!(
        metadata.version == 1
            && metadata.complete
            && metadata.binding_policy.as_deref() == Some(BINDING_POLICY),
        "Unsupported compiler-output ownership"
    );
    let paths = Paths::new(roots)?;
    let outputs = metadata
        .owned_bytecode_directories
        .iter()
        .map(|path| paths.local(path).map(PathBuf::from))
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        outputs.iter().all(|path| symlink_free(path, roots)),
        "Compiler-output binding became a symlink or is unreadable; use manual refresh"
    );
    Ok(outputs)
}

pub(crate) fn direct_children(path: &Path, outputs: &[PathBuf]) -> Vec<String> {
    outputs
        .iter()
        .filter(|output| output.parent() == Some(path))
        .filter_map(|output| output.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CompilerOutputs;
    fn ownership(root: &Path) -> CompilerOutputs {
        CompilerOutputs {
            bytecode_directories: vec![root.join("classes").to_string_lossy().into_owned()],
            raw_bytecode_directories: Some(vec![
                root.join("classes").to_string_lossy().into_owned(),
            ]),
            compile_classpath: Vec::new(),
            source_roots: vec![root.join("src").to_string_lossy().into_owned()],
            build_inputs: vec![root.join("pom.xml").to_string_lossy().into_owned()],
        }
    }
    #[test]
    fn output_ownership_never_excludes_source_config_or_other_consumed_inputs() {
        let root = tempfile::tempdir().unwrap();
        let roots = [("repo".into(), root.path().into())];
        let paths = fingerprint::capture_excluding(vec![root.path().into()], &[]).unwrap();
        let analysis = super::super::provenance::build(&paths, &[], "{}", &roots).unwrap();
        let mut outputs = ownership(root.path());
        assert!(
            capture(&analysis, &outputs, true, &roots).unwrap().value["complete"]
                .as_bool()
                .unwrap()
        );
        outputs
            .source_roots
            .push(outputs.bytecode_directories[0].clone());
        assert_eq!(
            capture(&analysis, &outputs, true, &roots).unwrap().value["complete"],
            false
        );
        outputs = ownership(root.path());
        outputs.bytecode_directories = vec![root.path().to_string_lossy().into_owned()];
        assert_eq!(
            capture(&analysis, &outputs, true, &roots).unwrap().value["complete"],
            false
        );
        outputs = ownership(root.path());
        std::fs::create_dir(root.path().join("classes")).unwrap();
        let resource = root.path().join("classes/authored.properties");
        std::fs::write(&resource, "authored=true").unwrap();
        let captured = fingerprint::capture_excluding(vec![resource], &[]).unwrap();
        let analysis = super::super::provenance::build(&captured, &[], "{}", &roots).unwrap();
        assert_eq!(
            capture(&analysis, &outputs, true, &roots).unwrap().value["complete"],
            false
        );
    }
    #[test]
    fn foreign_classpath_and_unverified_output_bindings_disable_refresh() {
        let root = tempfile::tempdir().unwrap();
        let dependency = root.path().join("classes/dependency");
        std::fs::create_dir_all(&dependency).unwrap();
        std::fs::write(dependency.join("Dep.class"), "foreign bytecode").unwrap();
        let roots = [("repo".into(), root.path().into())];
        let snapshots = fingerprint::capture_excluding(vec![root.path().into()], &[]).unwrap();
        let analysis = super::super::provenance::build(&snapshots, &[], "{}", &roots).unwrap();
        let complete = |outputs: &CompilerOutputs| {
            capture(&analysis, outputs, true, &roots).unwrap().value["complete"]
                .as_bool()
                .unwrap()
        };
        let mut outputs = ownership(root.path());
        assert!(complete(&outputs));
        outputs.compile_classpath = outputs.bytecode_directories.clone();
        assert!(
            complete(&outputs),
            "Exact owned cross-module outputs remain valid"
        );
        outputs.compile_classpath = vec![
            root.path()
                .join("classes/dependency")
                .to_string_lossy()
                .into_owned(),
        ];
        assert!(
            !complete(&outputs),
            "Missing nested foreign dependency directory is not an owned output"
        );
        outputs.compile_classpath = vec![root.path().to_string_lossy().into_owned()];
        assert!(
            !complete(&outputs),
            "Foreign classpath ancestors are ambiguous too"
        );
        outputs = ownership(root.path());
        outputs.raw_bytecode_directories = None;
        assert!(
            !complete(&outputs),
            "Old metadata cannot prove raw output bindings"
        );
    }

    #[test]
    fn raw_symlink_output_and_missing_nonclass_inputs_are_not_certified() {
        let root = tempfile::tempdir().unwrap();
        let roots = [("repo".into(), root.path().into())];
        let actual = root.path().join("classes");
        std::fs::create_dir(&actual).unwrap();
        let alias = root.path().join("alias");
        std::os::unix::fs::symlink(&actual, &alias).unwrap();
        let snapshots = fingerprint::capture_excluding(vec![root.path().into()], &[]).unwrap();
        let mut analysis = super::super::provenance::build(&snapshots, &[], "{}", &roots).unwrap();
        let mut outputs = ownership(root.path());
        outputs.raw_bytecode_directories = Some(vec![alias.to_string_lossy().into_owned()]);
        assert_eq!(
            capture(&analysis, &outputs, true, &roots).unwrap().value["complete"],
            false
        );
        outputs = ownership(root.path());
        let missing = actual.join("external-dependency");
        let observed = fingerprint::capture_excluding(vec![missing], &[]).unwrap();
        analysis = super::super::provenance::build(&observed, &[], "{}", &roots).unwrap();
        assert_eq!(
            capture(&analysis, &outputs, true, &roots).unwrap().value["complete"],
            false
        );
    }

    #[test]
    fn uncertified_prototype_policy_stays_readable_but_requires_manual_refresh() {
        let root = tempfile::tempdir().unwrap();
        let roots = [("repo".into(), root.path().into())];
        let snapshots = fingerprint::capture_excluding(vec![root.path().into()], &[]).unwrap();
        let mut analysis = super::super::provenance::build(&snapshots, &[], "{}", &roots).unwrap();
        let mut policy = capture(&analysis, &ownership(root.path()), true, &roots)
            .unwrap()
            .value;
        assert_eq!(policy["complete"], true);
        policy.as_object_mut().unwrap().remove("binding_policy");
        analysis.refresh_inputs = Some(policy.clone());
        analysis.validate().unwrap();
        assert!(filtered(&analysis).is_err());
        assert!(owned_outputs(&analysis, &roots).is_err());
        policy["binding_policy"] = "future-policy".into();
        analysis.refresh_inputs = Some(policy);
        analysis.validate().unwrap();
        assert!(filtered(&analysis).is_err());
    }

    #[test]
    fn distinct_input_policies_can_converge_on_one_saved_override() {
        let root = tempfile::tempdir().unwrap();
        let roots = [("repo".into(), root.path().into())];
        let a = root.path().join("a");
        let b = root.path().join("b");
        std::fs::create_dir(&a).unwrap();
        std::fs::create_dir(&b).unwrap();
        let mut snapshots =
            fingerprint::capture_excluding(vec![root.path().into()], std::slice::from_ref(&a))
                .unwrap();
        snapshots.extend(
            fingerprint::capture_excluding(vec![root.path().into()], std::slice::from_ref(&b))
                .unwrap(),
        );
        let mut analysis = super::super::provenance::build(&snapshots, &[], "{}", &roots).unwrap();
        let outputs = CompilerOutputs {
            bytecode_directories: vec![
                a.to_string_lossy().into_owned(),
                b.to_string_lossy().into_owned(),
            ],
            raw_bytecode_directories: Some(vec![
                a.to_string_lossy().into_owned(),
                b.to_string_lossy().into_owned(),
            ]),
            ..Default::default()
        };
        let captured = capture(&analysis, &outputs, true, &roots).unwrap();
        assert_eq!(captured.value["directories"].as_array().unwrap().len(), 1);
        analysis.refresh_inputs = Some(captured.value);
        analysis.validate().unwrap();
        assert!(
            filtered(&analysis)
                .unwrap()
                .inputs
                .iter()
                .all(|input| input.excluded_children == ["a", "b"])
        );
    }

    #[test]
    fn directory_overrides_keep_each_recorded_exclusion_policy_and_future_payloads_opaque() {
        let root = tempfile::tempdir().unwrap();
        let roots = [("repo".into(), root.path().into())];
        std::fs::create_dir(root.path().join("classes")).unwrap();
        std::fs::create_dir(root.path().join("custom-output")).unwrap();
        let mut snapshots = fingerprint::capture_excluding(vec![root.path().into()], &[]).unwrap();
        snapshots.extend(
            fingerprint::capture_excluding(
                vec![root.path().into()],
                &[root.path().join("custom-output")],
            )
            .unwrap(),
        );
        let mut analysis = super::super::provenance::build(&snapshots, &[], "{}", &roots).unwrap();
        analysis.refresh_inputs = Some(
            capture(&analysis, &ownership(root.path()), true, &roots)
                .unwrap()
                .value,
        );
        analysis.validate().unwrap();
        let filtered = filtered(&analysis).unwrap();
        assert_eq!(filtered.inputs.len(), 2);
        assert_eq!(filtered.inputs[0].excluded_children, vec!["classes"]);
        assert_eq!(
            filtered.inputs[1].excluded_children,
            vec!["classes", "custom-output"]
        );
        analysis.refresh_inputs =
            Some(serde_json::json!({"version":2,"directories":[3,1,2],"opaque":{"anything":true}}));
        analysis.validate().unwrap();
        let before = analysis.refresh_inputs.clone();
        let mut value = before.clone().unwrap();
        normalize(&mut value);
        assert_eq!(Some(value), before);
        assert!(super::filtered(&analysis).is_err());
    }
}
