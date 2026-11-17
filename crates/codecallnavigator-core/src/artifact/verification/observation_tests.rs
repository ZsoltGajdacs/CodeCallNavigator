use super::*;
use crate::{artifact, fingerprint, model::GraphData};
use std::{path::PathBuf, sync::atomic::AtomicBool};

fn artifact_for(roots: &Roots, paths: Vec<PathBuf>, outputs: &[PathBuf]) -> Artifact {
    let captured = fingerprint::capture_excluding(paths, outputs).unwrap();
    let mut artifact = artifact::canonical(&GraphData::default(), roots).unwrap();
    artifact.graph["analysis"] =
        serde_json::to_value(provenance::build(&captured, &[], "{}", roots).unwrap()).unwrap();
    artifact
}

fn inspect(artifact: &Artifact, roots: &Roots) -> Observation {
    observe_artifact(artifact, "saved-generation", roots, &AtomicBool::new(false)).unwrap()
}

fn record_owned_class_directory(artifact: &mut Artifact, classes: &std::path::Path, roots: &Roots) {
    let mut analysis = provenance::read(&artifact.graph).unwrap().unwrap();
    let captured = artifact::refresh_inputs::capture(
        &analysis,
        &crate::model::CompilerOutputs {
            bytecode_directories: vec![classes.to_string_lossy().into_owned()],
            raw_bytecode_directories: Some(vec![classes.to_string_lossy().into_owned()]),
            ..Default::default()
        },
        true,
        roots,
    )
    .unwrap();
    analysis.refresh_inputs = Some(captured.value);
    artifact.graph["analysis"] = serde_json::to_value(analysis).unwrap();
}

#[test]
fn known_stale_and_new_file_repairs_change_content_revision_without_timestamps() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("app.ts");
    fs::write(&source, "export const value = 1;").unwrap();
    let roots = [("repo".into(), root.path().into())];
    let artifact = artifact_for(&roots, vec![root.path().into(), source.clone()], &[]);
    let initial = inspect(&artifact, &roots);
    assert_eq!(initial.report.input_status, InputStatus::Match);
    fs::write(&source, "export const value = 2;").unwrap();
    let stale = inspect(&artifact, &roots);
    fs::write(&source, "export const value = 3;").unwrap();
    let stale_again = inspect(&artifact, &roots);
    assert_ne!(stale.input_revision, stale_again.input_revision);
    fs::create_dir(root.path().join("added")).unwrap();
    let added = root.path().join("added/new.ts");
    fs::write(&added, "export const added = ").unwrap();
    let invalid = inspect(&artifact, &roots);
    fs::write(&added, "export const added = 4;").unwrap();
    let repaired = inspect(&artifact, &roots);
    assert_ne!(invalid.input_revision, repaired.input_revision);
    assert!(repaired.complete);
    assert_eq!(repaired.report.input_status, InputStatus::Stale);
    assert_eq!(repaired.report.digest, "saved-generation");
}

#[test]
fn generated_outputs_do_not_change_revision_but_explicit_generated_inputs_do() {
    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("custom-output");
    fs::create_dir(&output).unwrap();
    let imported = output.join("Imported.props");
    fs::write(&imported, "<Project/>").unwrap();
    let roots = [("repo".into(), root.path().into())];
    let artifact = artifact_for(
        &roots,
        vec![root.path().into(), imported.clone()],
        std::slice::from_ref(&output),
    );
    let initial = inspect(&artifact, &roots);
    fs::write(output.join("intermediate.bin"), "generated").unwrap();
    fs::create_dir(root.path().join("target")).unwrap();
    fs::write(root.path().join("target/object.bin"), "generated").unwrap();
    fs::create_dir(root.path().join(".codecallnavigator")).unwrap();
    fs::write(
        root.path().join(".codecallnavigator/graph.json"),
        "generated",
    )
    .unwrap();
    fs::write(
        root.path().join(".codecallnavigator/inference.sqlite3"),
        "generated",
    )
    .unwrap();
    let outputs = inspect(&artifact, &roots);
    assert_eq!(initial.input_revision, outputs.input_revision);
    assert_eq!(outputs.report.input_status, InputStatus::Match);
    fs::write(imported, "<Project><PropertyGroup/></Project>").unwrap();
    let changed = inspect(&artifact, &roots);
    assert_ne!(outputs.input_revision, changed.input_revision);
    assert_eq!(changed.report.input_status, InputStatus::Stale);
}

#[test]
fn missing_inputs_and_linked_roots_have_explicit_revision_coverage() {
    let base = tempfile::tempdir().unwrap();
    let repo = base.path().join("repo");
    let shared = base.path().join("shared");
    fs::create_dir(&repo).unwrap();
    fs::create_dir(&shared).unwrap();
    let missing = repo.join("missing.ts");
    let source = shared.join("linked.ts");
    fs::write(&source, "export const linked = 1;").unwrap();
    let roots = [
        ("repo".into(), repo.clone()),
        ("shared".into(), shared.clone()),
    ];
    let artifact = artifact_for(
        &roots,
        vec![
            repo.clone(),
            shared.clone(),
            missing.clone(),
            source.clone(),
        ],
        &[],
    );
    let initial = inspect(&artifact, &roots);
    assert!(initial.complete);
    assert_unmapped_environment_preserves_observed_scope_coverage(&artifact, &roots);
    fs::write(&missing, "export const created = 1;").unwrap();
    assert_ne!(
        initial.input_revision,
        inspect(&artifact, &roots).input_revision
    );
    fs::remove_file(missing).unwrap();
    let renamed = shared.join("renamed.ts");
    fs::rename(&source, &renamed).unwrap();
    let renamed_observation = inspect(&artifact, &roots);
    assert!(renamed_observation.complete);
    assert_eq!(renamed_observation.report.input_status, InputStatus::Stale);
    assert_ne!(initial.input_revision, renamed_observation.input_revision);
    fs::remove_file(&renamed).unwrap();
    let removed = inspect(&artifact, &roots);
    assert!(removed.complete);
    assert_eq!(removed.report.input_status, InputStatus::Stale);
    assert_ne!(renamed_observation.input_revision, removed.input_revision);
    assert!(
        removed
            .report
            .reasons
            .iter()
            .any(|reason| reason.code == "input_missing" && !reason.is_blocker())
    );
    fs::rename(&shared, base.path().join("unavailable")).unwrap();
    let unavailable = inspect(&artifact, &roots);
    assert!(!unavailable.complete);
    assert_eq!(unavailable.report.input_status, InputStatus::Unverifiable);
    assert!(
        unavailable
            .report
            .reasons
            .iter()
            .any(|reason| reason.code == "unavailable_root")
    );
}

fn assert_unmapped_environment_preserves_observed_scope_coverage(
    artifact: &Artifact,
    roots: &Roots,
) {
    let mut unmapped = artifact.clone();
    unmapped.graph["analysis"]["limitations"] =
        serde_json::json!([{"code":"unmapped_input","count":3}]);
    let observed = inspect(&unmapped, roots);
    assert!(observed.complete);
    assert_eq!(observed.report.input_status, InputStatus::Unverifiable);
    assert!(
        observed
            .report
            .reasons
            .iter()
            .any(|reason| reason.describe().contains("3 required"))
    );
}

#[cfg(unix)]
#[test]
fn incidental_symlinks_are_ignored_and_required_link_bytes_are_observed() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let target = external.path().join("target.ts");
    fs::write(&target, "export const value = 1;").unwrap();
    let roots = [("repo".into(), root.path().into())];
    let artifact = artifact_for(&roots, vec![root.path().into()], &[]);
    let initial = inspect(&artifact, &roots);
    let alias = root.path().join("linked.ts");
    symlink(&target, &alias).unwrap();
    let ignored = inspect(&artifact, &roots);
    assert_eq!(initial.input_revision, ignored.input_revision);
    let required = artifact_for(&roots, vec![root.path().into(), alias], &[]);
    let before = inspect(&required, &roots);
    fs::write(&target, "export const value = 2;").unwrap();
    let after = inspect(&required, &roots);
    assert_ne!(before.input_revision, after.input_revision);
    assert!(after.complete);
    symlink(external.path(), root.path().join("directory-link")).unwrap();
    let ignored = inspect(&artifact, &roots);
    assert!(ignored.complete);
    assert_eq!(initial.input_revision, ignored.input_revision);
    fs::remove_file(&target).unwrap();
    let broken_required = inspect(&required, &roots);
    assert!(!broken_required.complete);
    assert!(
        broken_required
            .report
            .reasons
            .iter()
            .any(|reason| reason.code == "unreadable_input" && reason.is_blocker())
    );
}

#[cfg(unix)]
#[test]
fn legacy_broken_link_retargets_change_revision_but_plain_missing_inputs_remain_complete() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let alias = root.path().join("alias.ts");
    let missing = root.path().join("plain-missing.ts");
    symlink("absent-one.ts", &alias).unwrap();
    let roots = [("repo".into(), root.path().into())];
    // Persist a genuine legacy inventory: those graphs retain their previous
    // link semantics even though a new analysis rejects broken required links.
    let captured = [root.path().to_path_buf(), alias.clone(), missing]
        .into_iter()
        .map(|path| {
            let current = fingerprint::observe_content_with_policy(
                &path,
                &[],
                fingerprint::InputPolicy::Legacy,
            )
            .unwrap();
            crate::model::FileFingerprint {
                path: path.to_string_lossy().into_owned(),
                hash: current.hash,
                kind: current.kind,
                missing: current.kind == fingerprint::InputKind::Missing,
                observation_policy: fingerprint::InputPolicy::Legacy,
                ..Default::default()
            }
        })
        .collect::<Vec<_>>();
    let mut artifact = artifact::canonical(&GraphData::default(), &roots).unwrap();
    artifact.graph["analysis"] =
        serde_json::to_value(provenance::build(&captured, &[], "{}", &roots).unwrap()).unwrap();
    let before = inspect(&artifact, &roots);
    assert!(before.complete);
    assert!(
        before
            .report
            .reasons
            .iter()
            .any(|reason| reason.code == "legacy_input_policy")
    );
    assert_eq!(before.report.input_status, InputStatus::Unverifiable);
    fs::remove_file(&alias).unwrap();
    symlink("absent-two.ts", &alias).unwrap();
    let after = inspect(&artifact, &roots);
    assert_ne!(before.input_revision, after.input_revision);
    assert!(after.complete);
    assert_eq!(after.report.input_status, InputStatus::Unverifiable);
}

#[test]
fn each_directory_scope_retains_its_own_exclusions() {
    let root = tempfile::tempdir().unwrap();
    let roots = [("repo".into(), root.path().into())];
    let mut captured = fingerprint::capture_excluding([root.path().into()], &[]).unwrap();
    captured.extend(
        fingerprint::capture_excluding([root.path().into()], &[root.path().join("custom-output")])
            .unwrap(),
    );
    let mut artifact = artifact::canonical(&GraphData::default(), &roots).unwrap();
    artifact.graph["analysis"] =
        serde_json::to_value(provenance::build(&captured, &[], "{}", &roots).unwrap()).unwrap();
    fs::create_dir(root.path().join("custom-output")).unwrap();
    let added = root.path().join("custom-output/new.ts");
    fs::write(&added, "invalid").unwrap();
    let before = inspect(&artifact, &roots);
    fs::write(added, "export const repaired = 1;").unwrap();
    let after = inspect(&artifact, &roots);
    assert_ne!(before.input_revision, after.input_revision);
    assert_eq!(after.report.input_status, InputStatus::Stale);
}

#[test]
fn maven_generated_marker_does_not_change_observed_revision() {
    let root = tempfile::tempdir().unwrap();
    let roots = [("repo".into(), root.path().into())];
    let artifact = artifact_for(&roots, vec![root.path().into()], &[]);
    let initial = inspect(&artifact, &roots);
    fs::create_dir_all(root.path().join(".mvn/target/project-local-repo")).unwrap();
    fs::write(
        root.path().join(".mvn/target/project-local-repo/file"),
        "generated",
    )
    .unwrap();
    let after = inspect(&artifact, &roots);
    assert_eq!(after.report.input_status, InputStatus::Match);
    assert_eq!(initial.input_revision, after.input_revision);
}

#[test]
fn directory_exclusion_names_do_not_hide_ordinary_file_contents() {
    let root = tempfile::tempdir().unwrap();
    let roots = [("repo".into(), root.path().into())];
    let artifact = artifact_for(&roots, vec![root.path().into()], &[]);
    for name in ["target", "build"] {
        let file = root.path().join(name);
        fs::write(&file, "before").unwrap();
        let before = inspect(&artifact, &roots);
        fs::write(&file, "after").unwrap();
        assert_ne!(
            before.input_revision,
            inspect(&artifact, &roots).input_revision
        );
    }
}

#[cfg(unix)]
#[test]
fn special_file_targets_return_incomplete_without_blocking() {
    use std::{os::unix::fs::symlink, sync::mpsc, time::Duration};
    let root = tempfile::tempdir().unwrap();
    let roots = [("repo".into(), root.path().into())];
    let artifact = artifact_for(&roots, vec![root.path().into()], &[]);
    symlink("/dev/zero", root.path().join("zero")).unwrap();
    let fifo = root.path().join("fifo");
    let name = std::ffi::CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
    // Real FIFO with no writer: opening without O_NONBLOCK would hang.
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    symlink(&fifo, root.path().join("fifo-link")).unwrap();
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        let value = inspect(&artifact, &roots);
        let _ = send.send(value);
    });
    let result = receive
        .recv_timeout(Duration::from_secs(2))
        .expect("special input must never block reconciliation");
    assert!(!result.complete);
    assert!(
        result
            .report
            .reasons
            .iter()
            .any(|reason| reason.code == "unreadable_input")
    );
}

#[cfg(unix)]
#[test]
fn excluded_directory_symlinks_do_not_hide_explicit_dependency_inputs() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let dependencies = tempfile::tempdir().unwrap();
    let imported = dependencies.path().join("package.json");
    fs::write(&imported, "{}").unwrap();
    symlink(dependencies.path(), root.path().join("node_modules")).unwrap();
    let roots = [
        ("repo".into(), root.path().into()),
        ("deps".into(), dependencies.path().into()),
    ];
    let artifact = artifact_for(&roots, vec![root.path().into(), imported.clone()], &[]);
    let initial = inspect(&artifact, &roots);
    assert!(
        initial.complete,
        "Excluded dependency link is not an unobserved source scope"
    );
    fs::write(imported, "{\"changed\":true}").unwrap();
    let changed = inspect(&artifact, &roots);
    assert!(changed.complete);
    assert_ne!(initial.input_revision, changed.input_revision);
    assert_eq!(changed.report.input_status, InputStatus::Stale);
}

#[test]
fn refresh_trigger_keeps_full_freshness_but_excludes_only_owned_bytecode() {
    let root = tempfile::tempdir().unwrap();
    let classes = root.path().join("custom/classes");
    let generated = root.path().join("custom/generated");
    let dependency = root.path().join("dependency/classes");
    for path in [&classes, &generated, &dependency] {
        fs::create_dir_all(path).unwrap();
    }
    let class = classes.join("App.class");
    let source = root.path().join("App.java");
    fs::write(&class, "class-v1").unwrap();
    fs::write(&source, "class App {}").unwrap();
    let roots = [("repo".into(), root.path().into())];
    let observed =
        fingerprint::observation_paths(&[root.path().into()], &[], &AtomicBool::new(false))
            .unwrap();
    let mut artifact = artifact_for(&roots, observed, &[]);
    record_owned_class_directory(&mut artifact, &classes, &roots);
    let trigger = |artifact: &Artifact| {
        observe_refresh_trigger(artifact, "generation", &roots, &AtomicBool::new(false)).unwrap()
    };
    let initial = trigger(&artifact);
    let full_initial = inspect(&artifact, &roots);
    fs::write(&class, "class-v2").unwrap();
    let output_change = trigger(&artifact);
    assert_ne!(
        inspect(&artifact, &roots).input_revision,
        full_initial.input_revision
    );
    assert_eq!(output_change.input_revision, initial.input_revision);
    assert_eq!(output_change.report.input_status, InputStatus::Match);
    fs::remove_file(&class).unwrap();
    let fifo = std::ffi::CString::new(class.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    assert!(
        !inspect(&artifact, &roots).complete,
        "Full report retains unreadable owned-output coverage"
    );
    let during_compile = trigger(&artifact);
    assert!(
        during_compile.complete,
        "Owned-output FIFO cannot cancel a valid filtered refresh"
    );
    assert_eq!(during_compile.input_revision, initial.input_revision);
    fs::remove_file(&class).unwrap();
    fs::remove_dir_all(&classes).unwrap();
    assert_eq!(
        trigger(&artifact).input_revision,
        initial.input_revision,
        "Owned output directory membership is excluded too"
    );
    fs::write(generated.join("Generated.java"), "class Generated {}").unwrap();
    let generated_change = trigger(&artifact);
    assert_ne!(generated_change.input_revision, initial.input_revision);
    fs::write(dependency.join("Dependency.class"), "dependency-v2").unwrap();
    assert_ne!(
        trigger(&artifact).input_revision,
        generated_change.input_revision
    );
    fs::write(&source, "class App { void changed(){} }").unwrap();
    assert_eq!(trigger(&artifact).report.input_status, InputStatus::Stale);
    std::os::unix::fs::symlink(&dependency, &classes).unwrap();
    assert!(
        observe_refresh_trigger(&artifact, "generation", &roots, &AtomicBool::new(false)).is_err(),
        "Replacing an owned output with a symlink invalidates scheduling ownership"
    );
}

// Real directory traversal counters prove that overlapping scopes share I/O;
// pure grouping tests cannot establish filesystem walk cost or content coverage.
#[test]
fn overlapping_scopes_reconcile_each_directory_once_and_keep_live_content() {
    let root = tempfile::tempdir().unwrap();
    let roots = [("repo".into(), root.path().into())];
    let mut directories = vec![root.path().to_path_buf()];
    for index in 0..16 {
        let child = directories.last().unwrap().join(format!("level-{index}"));
        fs::create_dir(&child).unwrap();
        directories.push(child);
    }
    let source = directories.last().unwrap().join("Input.java");
    fs::write(&source, "class Input {}").unwrap();
    let artifact = artifact_for(&roots, directories.clone(), &[]);
    let metrics = crate::metrics::Metrics::default();
    let initial = {
        let _scope = metrics.enter();
        inspect(&artifact, &roots)
    };
    assert!(initial.complete);
    assert_eq!(
        metrics.report().counters["fingerprint.directories_visited"],
        directories.len() as u64
    );
    assert_eq!(metrics.report().counters["fingerprint.files_hashed"], 1);
    fs::write(&source, "class Input { void changed() {} }").unwrap();
    let changed = inspect(&artifact, &roots);
    assert_ne!(initial.input_revision, changed.input_revision);
    let added = directories.last().unwrap().join("Added.java");
    fs::write(&added, "class Added {}").unwrap();
    let addition = inspect(&artifact, &roots);
    assert_ne!(changed.input_revision, addition.input_revision);
    fs::write(&added, "class Added { void repaired() {} }").unwrap();
    assert_ne!(
        addition.input_revision,
        inspect(&artifact, &roots).input_revision
    );
}

// Generated directories and directory aliases require actual filesystem entries:
// an explicit scope must remain traversable even when its parent walk skips it.
#[cfg(unix)]
#[test]
fn shared_traversal_keeps_explicit_generated_and_symlink_directory_roots() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let generated = root.path().join("target/generated");
    fs::create_dir_all(&generated).unwrap();
    let generated_source = generated.join("Generated.java");
    let linked_source = external.path().join("Linked.java");
    fs::write(&generated_source, "class Generated {}").unwrap();
    fs::write(&linked_source, "class Linked {}").unwrap();
    let alias = root.path().join("alias");
    std::os::unix::fs::symlink(external.path(), &alias).unwrap();
    let roots = [("repo".into(), root.path().into())];
    let artifact = artifact_for(&roots, vec![root.path().into(), generated, alias], &[]);
    let initial = inspect(&artifact, &roots);
    assert!(
        initial.complete,
        "Explicit directory aliases are covered scopes"
    );
    fs::write(&generated_source, "class Generated { void changed() {} }").unwrap();
    let generated_change = inspect(&artifact, &roots);
    assert!(generated_change.complete);
    assert_ne!(initial.input_revision, generated_change.input_revision);
    fs::write(&linked_source, "class Linked { void changed() {} }").unwrap();
    let alias_change = inspect(&artifact, &roots);
    assert!(alias_change.complete);
    assert_ne!(generated_change.input_revision, alias_change.input_revision);
}

// Permission failures require the operating system boundary; a fake read error
// would not prove that real source observation preserves the path and OS cause.
#[cfg(unix)]
#[test]
fn permission_denied_required_source_reports_actionable_incomplete_coverage() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("Required.java");
    fs::write(&source, "class Required {}").unwrap();
    let roots = [("repo".into(), root.path().into())];
    let artifact = artifact_for(&roots, vec![source.clone()], &[]);
    let original_permissions = fs::metadata(&source).unwrap().permissions();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o0)).unwrap();
    let permission_denied = fs::File::open(&source)
        .is_err_and(|error| error.kind() == std::io::ErrorKind::PermissionDenied);
    let observed = observe_artifact(
        &artifact,
        "saved-generation",
        &roots,
        &AtomicBool::new(false),
    );
    // Always restore before evaluating either the fixture prerequisite or results.
    fs::set_permissions(&source, original_permissions).unwrap();
    assert!(
        permission_denied,
        "Unavailable fixture prerequisite: this user bypasses chmod-denied file reads; run permission coverage as an unprivileged user"
    );
    let observed = observed.unwrap();
    assert!(!observed.complete);
    assert_eq!(observed.report.input_status, InputStatus::Unverifiable);
    let reason = observed
        .report
        .reasons
        .iter()
        .find(|reason| {
            reason.code == "unreadable_input"
                && reason.path.as_deref() == Some("@repo/Required.java")
        })
        .expect("Read failure retains the exact portable source path");
    assert!(
        reason
            .operation
            .as_deref()
            .is_some_and(|operation| operation.contains("Read"))
    );
    let detail = reason
        .detail
        .as_deref()
        .expect("Read failure retains OS detail");
    assert!(
        detail.contains("Permission denied") || detail.contains("os error 13"),
        "Unexpected OS cause: {detail}"
    );
    assert!(reason.recovery.as_deref().is_some_and(
        |recovery| recovery.contains("permissions") && recovery.contains("Retry freshness")
    ));
}
