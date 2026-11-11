//! Discovery behavior shared by the repository drawer and headless CLI.
use codecallnavigator::discovery::{default_selection, discover, target_from_file};
use std::fs;

#[test]
fn finds_config_variants_but_defaults_to_main_config_in_each_directory() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    for name in [
        "tsconfig.json",
        "tsconfig.test.json",
        "nested/tsconfig.app.json",
        "nested/tsconfig.test.json",
        "node_modules/dependency/tsconfig.json",
        ".next/tsconfig.json",
        "dist/tsconfig.json",
        "coverage/tsconfig.json",
    ] {
        let file = root.join(name);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, "{}").unwrap();
    }
    let targets = discover(root).unwrap();
    assert_eq!(targets.len(), 4);
    assert!(targets.iter().all(|target| target.language == "typescript"));
    let selected = default_selection(&targets);
    assert_eq!(selected.len(), 3);
    let main = target_from_file(&root.join("tsconfig.json")).unwrap();
    let tests = target_from_file(&root.join("tsconfig.test.json")).unwrap();
    assert_eq!(main.kind, "typescript");
    assert!(selected.contains(&main.identity()));
    assert!(!selected.contains(&tests.identity()));
    assert!(
        selected
            .iter()
            .any(|identity| identity.path.ends_with("nested/tsconfig.app.json"))
    );
    assert!(
        selected
            .iter()
            .any(|identity| identity.path.ends_with("nested/tsconfig.test.json"))
    );
}

#[test]
fn typescript_output_conventions_do_not_hide_java_source_packages() {
    let temporary = tempfile::tempdir().unwrap();
    for directory in ["src/main/java/coverage", "src/main/java/dist"] {
        fs::create_dir_all(temporary.path().join(directory)).unwrap();
        fs::write(
            temporary.path().join(directory).join("Worker.java"),
            "class Worker {}\n",
        )
        .unwrap();
    }
    let paths = codecallnavigator::fingerprint::source_paths(&[temporary.path().to_path_buf()]).unwrap();
    assert!(
        paths
            .iter()
            .any(|path| path.ends_with("java/coverage/Worker.java"))
    );
    assert!(
        paths
            .iter()
            .any(|path| path.ends_with("java/dist/Worker.java"))
    );
}
