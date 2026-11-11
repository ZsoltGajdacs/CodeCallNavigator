use codecallnavigator::{
    architecture::{self, ArchitectureFacts, FileFacts, Selection},
    model::GraphData,
};
use std::fs;
fn graph(language: &str) -> GraphData {
    GraphData {
        architecture: Some(ArchitectureFacts {
            version: architecture::FACTS_VERSION,
            files: vec![FileFacts {
                build: "/repo/app.csproj".into(),
                project: "app".into(),
                file: "/repo/src/Invoice.cs".into(),
                language: language.into(),
                namespaces: vec!["Billing.Services".into()],
                names: vec!["InvoiceService".into()],
                annotations: vec!["Web.ControllerAttribute".into()],
                method_annotations: vec!["Web.GetAttribute".into()],
                base_types: vec!["Core.IService".into()],
                annotated_interfaces: vec!["Api.EndpointAttribute".into()],
                kinds: vec!["class".into()],
                content: "invoice needle".into(),
                ..Default::default()
            }],
        }),
        ..Default::default()
    }
}
#[test]
fn native_semantic_rules_are_conjunctive_first_match_and_normalize_attributes() {
    let text = r#"version: 1
languages:
  csharp:
    layers:
      - name: Controller
        matchers:
          - path: '**/src/**'
            namespace: 'Billing.*'
            annotation: Controller
            method_annotation: Get
            base_type: IService
            annotated_interface: Endpoint
            kind: class
            content: needle
      - name: Service
        matchers: [{suffix: Service}]
"#;
    let config = architecture::parse(text).unwrap();
    let result = architecture::classify(&graph("csharp"), &config, "test".into()).unwrap();
    assert_eq!(result.files.values().next().unwrap().layer, "Controller");
    let config = architecture::parse(&text.replace("content: needle", "content: absent")).unwrap();
    assert_eq!(
        architecture::classify(&graph("csharp"), &config, "test".into())
            .unwrap()
            .files
            .values()
            .next()
            .unwrap()
            .layer,
        "Service"
    );
}
#[test]
fn tests_take_precedence_and_missing_facts_require_refresh() {
    let mut graph = graph("java");
    graph.architecture.as_mut().unwrap().files[0].is_test = true;
    let config = architecture::load(std::path::Path::new("/repo"), Selection::Spring).unwrap();
    let result = architecture::classify(&graph, &config.config, config.origin).unwrap();
    assert_eq!(result.files.values().next().unwrap().layer, "Tests");
    graph.architecture = None;
    assert!(
        architecture::classify(&graph, &config.config, "test".into())
            .unwrap_err()
            .to_string()
            .contains("Refresh")
    );
}
#[test]
fn import_resolves_presets_and_preserves_native_edits() {
    let temp = tempfile::tempdir().unwrap();
    let review = temp.path().join(".interactive-review");
    fs::create_dir(&review).unwrap();
    fs::write(review.join("layers.yml"), "extends: [spring, react]\n").unwrap();
    let loaded = architecture::load(temp.path(), Selection::Repository).unwrap();
    assert_eq!(loaded.config.languages.len(), 2);
    assert!(
        loaded.config.languages["java"]
            .layers
            .iter()
            .any(|l| l.tab.as_deref() == Some("Database"))
    );
    let native = temp.path().join(".codecallnavigator/layers.yml");
    fs::write(&native, "version: 1\nlanguages: {}\n").unwrap();
    fs::write(review.join("layers.yml"), "invalid: true").unwrap();
    assert!(
        architecture::load(temp.path(), Selection::Repository)
            .unwrap()
            .config
            .languages
            .is_empty()
    );
    assert_eq!(
        fs::read_to_string(native).unwrap(),
        "version: 1\nlanguages: {}\n"
    );
}
#[test]
fn invalid_imports_and_cycles_never_create_native_configuration() {
    let temp = tempfile::tempdir().unwrap();
    let review = temp.path().join(".interactive-review");
    fs::create_dir(&review).unwrap();
    for text in [
        "java:\n  layers:\n    - name: Bad\n      match: [{typo: x}]\n",
        "extends: [layers.yml]\n",
        "java:\n  layers:\n    - name: Bad\n      match: [{content: '['}]\n",
    ] {
        fs::write(review.join("layers.yml"), text).unwrap();
        assert!(architecture::load(temp.path(), Selection::Repository).is_err());
        assert!(!temp.path().join(".codecallnavigator/layers.yml").exists());
    }
}
#[test]
fn local_inheritance_replaces_language_sections_and_unknown_native_fields_fail() {
    let temp = tempfile::tempdir().unwrap();
    let review = temp.path().join(".interactive-review");
    fs::create_dir(&review).unwrap();
    fs::write(review.join("base.yml"), "extends: [spring]\n").unwrap();
    fs::write(review.join("layers.yml"),"extends: [base.yml]\njava:\n  layers:\n    - name: Domain\n      match: [{package: '*.domain.*'}]\n").unwrap();
    let config = architecture::load(temp.path(), Selection::Repository)
        .unwrap()
        .config;
    assert_eq!(config.languages["java"].layers.len(), 1);
    assert!(architecture::parse("version: 1\nunknown: true\n").is_err());
}

#[test]
fn imported_paths_remain_relative_to_nested_build_roots() {
    let temp = tempfile::tempdir().unwrap();
    let review = temp.path().join(".interactive-review");
    fs::create_dir(&review).unwrap();
    fs::write(
        review.join("layers.yml"),
        "ts:\n  layers:\n    - name: Component\n      match: [{path: 'src/components/**'}]\n",
    )
    .unwrap();
    let config = architecture::load(temp.path(), Selection::Repository).unwrap();
    let mut graph = graph("typescript");
    let file = &mut graph.architecture.as_mut().unwrap().files[0];
    file.build = temp
        .path()
        .join("packages/web/tsconfig.json")
        .to_string_lossy()
        .into();
    file.file = temp
        .path()
        .join("packages/web/src/components/Widget.tsx")
        .to_string_lossy()
        .into();
    let assignments = architecture::classify(&graph, &config.config, config.origin).unwrap();
    assert_eq!(
        assignments.files.values().next().unwrap().layer,
        "Component"
    );
    let restored = architecture::load(temp.path(), Selection::Repository).unwrap();
    assert_eq!(
        architecture::classify(&graph, &restored.config, restored.origin)
            .unwrap()
            .files
            .values()
            .next()
            .unwrap()
            .layer,
        "Component"
    );
}

#[test]
fn build_relative_rules_keep_dotted_java_directory_names() {
    let config = architecture::parse("version: 1\nlanguages:\n  java:\n    layers:\n      - name: Service\n        matchers: [{path: 'src/**', path_base: build}]\n").unwrap();
    let mut graph = graph("java");
    let file = &mut graph.architecture.as_mut().unwrap().files[0];
    file.build = "/repo/billing.service".into();
    file.file = "/repo/billing.service/src/Invoice.java".into();
    let result = architecture::classify(&graph, &config, "test".into()).unwrap();
    assert_eq!(result.files.values().next().unwrap().layer, "Service");
}

#[test]
fn repository_rules_resolve_symlink_roots_to_analyzer_source_paths() {
    let temp = tempfile::tempdir().unwrap();
    let checkout = temp.path().join("checkout");
    fs::create_dir_all(checkout.join(".codecallnavigator")).unwrap();
    fs::write(checkout.join(".codecallnavigator/layers.yml"), "version: 1\nlanguages:\n  csharp:\n    layers:\n      - name: Controller\n        matchers: [{path: 'src/**'}]\n").unwrap();
    let alias = temp.path().join("workspace");
    std::os::unix::fs::symlink(&checkout, &alias).unwrap();
    let loaded = architecture::load(&alias, Selection::Repository).unwrap();
    let mut graph = graph("csharp");
    graph.architecture.as_mut().unwrap().files[0].file = checkout
        .canonicalize()
        .unwrap()
        .join("src/Invoice.cs")
        .to_string_lossy()
        .into();
    let assignments = architecture::classify(&graph, &loaded.config, loaded.origin).unwrap();
    assert_eq!(
        assignments.files.values().next().unwrap().layer,
        "Controller"
    );
}
