use super::*;

const GRAPH_RESOURCE_NAMES: [&str; 4] = [
    "graph.json",
    "graph.manifest.schema.json",
    "graph.schema.json",
    "README.md",
];

#[test]
fn graph_publication_does_not_invalidate_sources_but_configuration_does() {
    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("App.java"), "class App {}").unwrap();
    let before = capture(source_paths(&[project.path().into()]).unwrap()).unwrap();
    let directory = project.path().join(".codecallnavigator");
    fs::create_dir(&directory).unwrap();
    for name in [
        "graph.json",
        "graph.manifest.schema.json",
        "graph.schema.json",
        "README.md",
        "graph.lock",
        "publication.lock",
        "previous.json",
        ".graph-publish-before",
        "inference.sqlite3",
        "inference.sqlite3-wal",
        "inference.sqlite3-shm",
        "inference.sqlite3-journal",
    ] {
        fs::write(directory.join(name), "generated").unwrap();
    }
    fs::create_dir(directory.join("objects")).unwrap();
    fs::write(directory.join("objects/part.flows.json.zst"), "compressed").unwrap();
    assert!(!is_stale(&before).unwrap());
    fs::write(directory.join("layers.yml"), "version: 1\n").unwrap();
    assert!(is_stale(&before).unwrap());
    let paths = source_paths(&[project.path().into()]).unwrap();
    assert!(paths.contains(&directory.join("layers.yml")));
    let before = capture(paths).unwrap();
    fs::write(directory.join("graph.json"), "replacement").unwrap();
    fs::remove_file(directory.join(".graph-publish-before")).unwrap();
    fs::write(directory.join(".graph-publish-after"), "temporary").unwrap();
    fs::write(
        directory.join("objects/another.structure.json.zst"),
        "compressed",
    )
    .unwrap();
    assert!(!is_stale(&before).unwrap());
    fs::write(directory.join("layers.yml"), "version: 2\n").unwrap();
    assert!(is_stale(&before).unwrap());
}

#[test]
fn resource_collection_excludes_only_project_graph_outputs() {
    let project = tempfile::tempdir().unwrap();
    let directory = project.path().join(".codecallnavigator");
    fs::create_dir(&directory).unwrap();
    for name in [
        "graph.json",
        "graph.manifest.schema.json",
        "graph.schema.json",
        "README.md",
        "graph.lock",
        "publication.lock",
        "previous.json",
        ".graph-publish-active",
        "layers.yml",
        "other.json",
        "inference.sqlite3",
        "inference.sqlite3-wal",
        "inference.sqlite3-shm",
        "inference.sqlite3-journal",
        "inference.sqlite3.authored",
    ] {
        fs::write(directory.join(name), "content").unwrap();
    }
    fs::create_dir(directory.join("objects")).unwrap();
    let object = directory.join("objects/part.flows.json.zst");
    fs::write(&object, "compressed").unwrap();
    for name in GRAPH_RESOURCE_NAMES {
        fs::write(project.path().join(name), "authored resource").unwrap();
    }
    let paths = input_paths_excluding(&[project.path().into()], &[]).unwrap();
    assert!(paths.contains(&directory.join("layers.yml")));
    assert!(paths.contains(&directory.join("other.json")));
    for name in GRAPH_RESOURCE_NAMES {
        assert!(paths.contains(&project.path().join(name)));
    }
    assert!(paths.contains(&directory.join("inference.sqlite3.authored")));
    assert!(!paths.contains(&object));
    assert!(capture([object.clone()]).unwrap().is_empty());
    assert!(input_paths_excluding(&[object], &[]).unwrap().is_empty());
    for name in [
        "graph.json",
        "graph.manifest.schema.json",
        "graph.schema.json",
        "README.md",
        "graph.lock",
        ".graph-publish-active",
        "inference.sqlite3",
        "inference.sqlite3-wal",
        "inference.sqlite3-shm",
        "inference.sqlite3-journal",
    ] {
        assert!(
            !paths.contains(&directory.join(name)),
            "Unexpected generated input {name}"
        );
    }
    let before = capture_excluding(paths, &[]).unwrap();
    fs::write(
        project.path().join("graph.json"),
        "changed authored resource",
    )
    .unwrap();
    assert!(is_stale(&before).unwrap());
}

#[test]
fn explicit_observations_of_generated_graph_files_are_excluded() {
    let project = tempfile::tempdir().unwrap();
    let directory = project.path().join(".codecallnavigator");
    fs::create_dir(&directory).unwrap();
    for name in [
        "graph.json",
        "graph.manifest.schema.json",
        "graph.schema.json",
        "README.md",
        "inference.sqlite3",
        "inference.sqlite3-wal",
        "inference.sqlite3-shm",
        "inference.sqlite3-journal",
    ] {
        let path = directory.join(name);
        fs::write(&path, "generated").unwrap();
        assert!(capture([path.clone()]).unwrap().is_empty());
        assert!(capture_excluding([path.clone()], &[]).unwrap().is_empty());
        assert!(input_paths_excluding(&[path], &[]).unwrap().is_empty());
    }
}
