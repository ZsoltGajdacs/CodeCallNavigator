use super::*;
use crate::model::{Edge, EntryPoints, GraphData, Node};
use serde_json::json;
use std::{fs, process::Command, sync::atomic::AtomicBool};

fn fixture(root: &str) -> GraphData {
    let node = |name: &str| Node {
        id: format!("{root}/App.csproj|[App]::Program::{name}()"),
        project: "App".into(),
        build: format!("{root}/App.csproj"),
        language: "csharp".into(),
        kind: "method".into(),
        label: name.into(),
        signature: format!("[App]::Program::{name}()"),
        container: "Program".into(),
        file: format!("{root}/Program.cs"),
        ..Node::default()
    };
    let nodes = vec![node("Start"), node("Leaf")];
    let entries = EntryPoints {
        candidates: vec![nodes[0].id.clone()],
        ..EntryPoints::default()
    };
    GraphData {
        nodes,
        edges: vec![Edge {
            evidence: Vec::new(),
            from: 0,
            to: 1,
            kind: "call".into(),
            certainty: "direct".into(),
            provenance: "compiler".into(),
            count: 1,
        }],
        roots: vec![format!("{root}/App.csproj")],
        entry_points: Some(entries),
        ..GraphData::default()
    }
}

fn roots(root: &str) -> Vec<(String, PathBuf)> {
    vec![("repo".into(), root.into())]
}

#[test]
fn relocating_checkout_and_reordering_nodes_preserves_bytes() {
    let before = canonical(&fixture("/tmp/first"), &roots("/tmp/first")).unwrap();
    let mut graph = fixture("/tmp/second");
    graph.nodes.swap(0, 1);
    graph.edges[0].from = 1;
    graph.edges[0].to = 0;
    let after = canonical(&graph, &roots("/tmp/second")).unwrap();
    assert_eq!(before.bytes().unwrap(), after.bytes().unwrap());
    let hydrated = after.hydrate(&roots("/tmp/local")).unwrap();
    assert!(
        hydrated
            .nodes
            .iter()
            .all(|n| n.file == "/tmp/local/Program.cs")
    );
}

#[test]
fn semantic_edge_attributes_and_same_count_topology_changes_are_visible() {
    let graph = fixture("/tmp/a");
    let before = canonical(&graph, &roots("/tmp/a")).unwrap();
    let mut graph = graph;
    graph.edges[0].count = 3;
    graph.edges[0].certainty = "possible".into();
    let changed = canonical(&graph, &roots("/tmp/a")).unwrap();
    let result = compare(&before, &changed).unwrap();
    assert!(!result.equal);
    assert_eq!(result.edges.changed.len(), 1);
    graph.edges[0].to = 0;
    let moved = canonical(&graph, &roots("/tmp/a")).unwrap();
    let result = compare(&before, &moved).unwrap();
    assert_eq!(result.edges.added.len(), 1);
    assert_eq!(result.edges.removed.len(), 1);
}

#[test]
fn invalid_artifacts_and_unmapped_paths_are_rejected() {
    let artifact = canonical(&fixture("/tmp/a"), &roots("/tmp/a")).unwrap();
    let mut invalid = artifact.clone();
    invalid.schema = "future".into();
    assert!(invalid.bytes().is_err());
    let mut invalid = artifact.clone();
    invalid.graph["edges"][0]["to"] = "missing".into();
    assert!(invalid.validate().is_err());
    let mut invalid = artifact;
    let duplicate = invalid.graph["nodes"][0].clone();
    invalid.graph["nodes"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(invalid.validate().is_err());
    assert!(canonical(&fixture("/tmp/a"), &roots("/tmp/b")).is_err());
}

#[test]
fn supplied_typescript_semantic_keys_remain_distinct_across_files() {
    let mut graph = fixture("/tmp/a");
    graph.nodes[0].language = "typescript".into();
    graph.nodes[1].language = "typescript".into();
    graph.nodes[0].file = "/tmp/a/first.ts".into();
    graph.nodes[1].file = "/tmp/a/second.ts".into();
    let keys = graph
        .nodes
        .iter()
        .map(|node| (node.id.clone(), "function:leaf:():number".into()))
        .collect();
    let artifact = canonical_with_identities(&graph, &roots("/tmp/a"), &keys).unwrap();
    assert_ne!(
        artifact.graph["nodes"][0]["id"],
        artifact.graph["nodes"][1]["id"]
    );
    assert!(artifact.validate().is_ok());
}

#[test]
fn cancellation_preserves_the_only_project_artifact_and_configuration() {
    let project = tempfile::tempdir().unwrap();
    let artifact = canonical(&fixture("/tmp/a"), &roots("/tmp/a")).unwrap();
    fs::create_dir(project.path().join(".codegraph")).unwrap();
    fs::write(project.path().join(".codegraph/layers.yml"), "version: 1\n").unwrap();
    publish(project.path(), &artifact, &AtomicBool::new(false)).unwrap();
    let before = fs::read(project.path().join(GRAPH_PATH)).unwrap();
    let mut changed = artifact;
    changed.graph["nodes"][0]["label"] = "changed".into();
    assert!(publish(project.path(), &changed, &AtomicBool::new(true)).is_err());
    assert_eq!(before, fs::read(project.path().join(GRAPH_PATH)).unwrap());
    assert_eq!(
        fs::read_to_string(project.path().join(".codegraph/layers.yml")).unwrap(),
        "version: 1\n"
    );
    assert_eq!(
        fs::read_dir(project.path().join(".codegraph"))
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|e| e == "json"))
            .count(),
        1
    );
}

#[test]
fn whole_analysis_lock_excludes_other_writers_until_released() {
    let project = tempfile::tempdir().unwrap();
    let artifact = canonical(&fixture("/tmp/a"), &roots("/tmp/a")).unwrap();
    let owner = AnalysisLock::acquire(project.path()).unwrap();
    assert!(AnalysisLock::acquire(project.path()).is_err());
    owner.publish(&artifact, &AtomicBool::new(false)).unwrap();
    assert!(AnalysisLock::acquire(project.path()).is_err());
    drop(owner);
    assert!(AnalysisLock::acquire(project.path()).is_ok());
}

#[test]
fn source_text_is_preserved_and_control_flow_order_is_not_sorted() {
    let mut graph = fixture("/tmp/a");
    graph.flow_inventory = Some(
        serde_json::from_value(json!({"version":1,"functions":[{
            "symbol":graph.nodes[0].id, "file":"/tmp/a/Program.cs", "build":"/tmp/a/App.csproj",
            "body":[{"kind":"action","text":"z"},{"kind":"action","text":"a"}]
        }]}))
        .unwrap(),
    );
    graph.architecture = Some(serde_json::from_value(json!({"version":1,"files":[{
        "build":"/tmp/a/App.csproj", "file":"/tmp/a/Program.cs", "content":"literal /tmp/a/source",
        "names":["z","a"]
    }]})).unwrap());
    let artifact = canonical(&graph, &roots("/tmp/a")).unwrap();
    assert_eq!(
        artifact.graph["architecture"]["files"][0]["content"],
        "literal /tmp/a/source"
    );
    assert_eq!(
        artifact.graph["architecture"]["files"][0]["names"],
        json!(["a", "z"])
    );
    assert_eq!(
        artifact.graph["flow_inventory"]["functions"][0]["body"][0]["text"],
        "z"
    );
}

#[test]
fn git_comparison_reads_history_without_writing_snapshots() {
    let project = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .arg("-C")
            .arg(project.path())
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["config", "user.email", "test@example.invalid"]);
    git(&["config", "user.name", "Artifact test"]);
    let artifact = canonical(&fixture("/tmp/a"), &roots("/tmp/a")).unwrap();
    publish(project.path(), &artifact, &AtomicBool::new(false)).unwrap();
    git(&["add", GRAPH_PATH]);
    git(&["commit", "-qm", "baseline"]);
    assert!(
        compare_git(project.path(), "HEAD", "WORKTREE")
            .unwrap()
            .equal
    );
    let mut changed = artifact;
    changed.graph["nodes"][0]["label"] = "changed".into();
    publish(project.path(), &changed, &AtomicBool::new(false)).unwrap();
    assert_eq!(
        compare_git(project.path(), "HEAD", "WORKTREE")
            .unwrap()
            .nodes
            .changed
            .len(),
        1
    );
    assert!(compare_git(project.path(), "missing-revision", "WORKTREE").is_err());
    let files: Vec<_> = fs::read_dir(project.path().join(".codegraph"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(files.len(), 2); // graph.json and the persistent advisory lock
}

#[test]
fn java_and_csharp_portable_ids_survive_repeated_hydration_and_cache_roundtrips() {
    for language in ["java", "csharp"] {
        let mut graph = fixture("/tmp/original");
        for node in &mut graph.nodes {
            node.language = language.into();
        }
        let original = canonical(&graph, &roots("/tmp/original")).unwrap();
        let first = original.hydrate(&roots("/tmp/relocated")).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let store = crate::store::SqliteStore::open(directory.path()).unwrap();
        store.publish(&first, &AtomicBool::new(false)).unwrap();
        let restored = store.load_active().unwrap().unwrap();
        let after = canonical(&restored, &roots("/tmp/relocated")).unwrap();
        assert_eq!(
            original.bytes().unwrap(),
            after.bytes().unwrap(),
            "{language}"
        );
        let second = after.hydrate(&roots("/tmp/another-checkout")).unwrap();
        assert_eq!(
            after.bytes().unwrap(),
            canonical(&second, &roots("/tmp/another-checkout"))
                .unwrap()
                .bytes()
                .unwrap()
        );
    }
}

#[test]
fn only_exact_owner_matched_portable_ids_reuse_their_semantic_component() {
    let variants = [
        json!(["java", "@repo/App.csproj", "@repo/Program.cs", "raw-key"]).to_string(),
        json!([
            "csharp",
            "/tmp/a/App.csproj",
            "/tmp/a/Program.cs",
            "raw-key"
        ])
        .to_string(),
        json!([
            "csharp",
            "@repo/Other.csproj",
            "@repo/Program.cs",
            "raw-key"
        ])
        .to_string(),
        json!(["csharp", "@repo/App.csproj", "@repo/Other.cs", "raw-key"]).to_string(),
        "[\"csharp\", \"@repo/App.csproj\", \"@repo/Program.cs\", \"raw-key\"]".into(),
    ];
    for native in variants {
        let mut graph = fixture("/tmp/a");
        graph.nodes.truncate(1);
        graph.edges.clear();
        graph.entry_points = None;
        graph.nodes[0].id = native;
        let artifact = canonical(&graph, &roots("/tmp/a")).unwrap();
        let fields: Vec<String> =
            serde_json::from_str(artifact.graph["nodes"][0]["id"].as_str().unwrap()).unwrap();
        assert_ne!(fields[3], "raw-key");
        assert!(fields[3].contains("raw-key"));
    }
}

#[test]
fn recorded_real_language_graph_can_be_recanonicalized_after_relocation() {
    let original =
        Artifact::from_bytes(include_bytes!("../../../../fixtures/.codegraph/graph.json")).unwrap();
    let graph = original.hydrate(&roots("/tmp/mixed-relocated")).unwrap();
    let repeated = canonical(&graph, &roots("/tmp/mixed-relocated")).unwrap();
    let moved_again = repeated.hydrate(&roots("/tmp/mixed-again")).unwrap();
    let stable = canonical(&moved_again, &roots("/tmp/mixed-again")).unwrap();
    assert_eq!(
        blake3::hash(&repeated.bytes().unwrap()),
        blake3::hash(&stable.bytes().unwrap())
    );
    let ids = |artifact: &Artifact| {
        artifact.graph["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|node| node["id"].clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        ids(&original),
        ids(&repeated),
        "Display-label normalization must preserve every recorded portable ID"
    );
}

#[test]
fn compiler_project_labels_stay_stable_when_they_match_the_checkout_basename() {
    for language in ["java", "csharp"] {
        let make = |root: &str| {
            let mut graph = fixture(root);
            for node in &mut graph.nodes {
                node.language = language.into();
                node.project = "App".into();
            }
            graph.architecture=Some(serde_json::from_value(json!({"version":1,"files":[{"build":format!("{root}/App.csproj"),"file":format!("{root}/Program.cs"),"project":"App:module","language":language}]})).unwrap());
            graph.flow_inventory=Some(serde_json::from_value(json!({"version":1,"functions":[{"symbol":graph.nodes[0].id,"file":format!("{root}/Program.cs"),"build":format!("{root}/App.csproj"),"project":"App:module","language":language}]})).unwrap());
            graph
        };
        let first = canonical(&make("/tmp/App"), &roots("/tmp/App")).unwrap();
        let relocated = canonical(&make("/tmp/Renamed"), &roots("/tmp/Renamed")).unwrap();
        assert_eq!(
            blake3::hash(&first.bytes().unwrap()),
            blake3::hash(&relocated.bytes().unwrap()),
            "{language}"
        );
        let hydrated = first.hydrate(&roots("/tmp/Another")).unwrap();
        assert_eq!(hydrated.nodes[0].project, "App");
    }
}

#[test]
fn typescript_folder_derived_project_labels_still_relocate() {
    let make = |root: &str| {
        let mut graph = fixture(root);
        for node in &mut graph.nodes {
            node.language = "typescript".into();
            node.project = root.rsplit('/').next().unwrap().into();
        }
        graph
    };
    let first = canonical(&make("/tmp/first"), &roots("/tmp/first")).unwrap();
    let relocated = canonical(&make("/tmp/second"), &roots("/tmp/second")).unwrap();
    assert_eq!(first.bytes().unwrap(), relocated.bytes().unwrap());
    let hydrated = first.hydrate(&roots("/tmp/local")).unwrap();
    assert_eq!(hydrated.nodes[0].project, "local");
}

#[test]
fn typescript_semantic_string_literal_types_are_source_values_not_checkout_paths() {
    let make = |root: &str| {
        let mut graph = fixture(root);
        for node in &mut graph.nodes {
            node.language = "typescript".into();
        }
        let keys = graph
            .nodes
            .iter()
            .map(|node| {
                (
                    node.id.clone(),
                    json!([
                        "<module>",
                        "FunctionDeclaration",
                        node.label,
                        "(value: '/tmp/first/literal'): void"
                    ])
                    .to_string(),
                )
            })
            .collect();
        (graph, keys)
    };
    let (first, first_keys) = make("/tmp/first");
    let (second, second_keys) = make("/tmp/second");
    let first = canonical_with_identities(&first, &roots("/tmp/first"), &first_keys).unwrap();
    let second = canonical_with_identities(&second, &roots("/tmp/second"), &second_keys).unwrap();
    assert_eq!(
        blake3::hash(&first.bytes().unwrap()),
        blake3::hash(&second.bytes().unwrap())
    );
    assert!(
        first.graph["nodes"][0]["id"]
            .as_str()
            .unwrap()
            .contains("/tmp/first/literal")
    );
}

#[test]
fn malformed_portable_paths_are_invalid_before_comparison_or_hydration() {
    let original = canonical(&fixture("/tmp/a"), &roots("/tmp/a")).unwrap();
    for path in [
        "/absolute/source.cs",
        "relative.cs",
        "@repo/../escape.cs",
        "@repo/sub/../../escape.cs",
        "@../source.cs",
    ] {
        let mut invalid = original.clone();
        invalid.graph["nodes"][0]["file"] = path.into();
        assert!(invalid.validate().is_err(), "{path}");
        assert!(compare(&invalid, &invalid).is_err());
        assert!(Artifact::from_bytes(&serde_json::to_vec(&invalid).unwrap()).is_err());
    }
    let mut invalid = original;
    invalid.graph["roots"] = json!(["@repo/../outside"]);
    assert!(invalid.validate().is_err());
}

#[test]
fn nested_typescript_project_label_uses_build_owner_not_owning_checkout_basename() {
    let make = |root: &str| {
        let mut graph = fixture(root);
        for node in &mut graph.nodes {
            node.language = "typescript".into();
            node.build = format!("{root}/App/tsconfig.json");
            node.file = format!("{root}/App/src/Program.ts");
            node.project = "App".into();
        }
        graph
    };
    let first = canonical(&make("/tmp/App"), &roots("/tmp/App")).unwrap();
    let moved = canonical(&make("/tmp/Renamed"), &roots("/tmp/Renamed")).unwrap();
    assert_eq!(
        blake3::hash(&first.bytes().unwrap()),
        blake3::hash(&moved.bytes().unwrap())
    );
    assert_eq!(first.graph["nodes"][0]["project"], "@repo/App");
    assert_eq!(
        first.hydrate(&roots("/tmp/Another")).unwrap().nodes[0].project,
        "App"
    );
}

#[test]
fn explicit_gradle_project_origins_relocate_without_rewriting_compiler_labels() {
    let make = |root: &str| {
        let mut graph = fixture(root);
        graph.folder_project_builds = vec![root.into()];
        graph.identity_roots = vec![root.into()];
        for node in &mut graph.nodes {
            node.language = "java".into();
            node.build = root.into();
            node.project = format!("{}:sub", root.rsplit('/').next().unwrap());
        }
        graph.boundaries = Some(crate::boundary::BoundaryInventory {
            version: 1,
            uses: vec![],
            routes: vec![crate::boundary::RouteFact {
                symbol: Some(graph.nodes[0].id.clone()),
                build: root.into(),
                application: ":sub".into(),
                framework: "spring".into(),
                kind: "http".into(),
                path: Some("/hello".into()),
                methods: vec!["GET".into()],
                site: crate::model::SourceSite {
                    file: graph.nodes[0].file.clone(),
                    line: 1,
                    column: None,
                    span: None,
                },
                restrictions: vec![],
                order: Some(0),
            }],
        });
        graph
    };
    let first = canonical(&make("/tmp/first"), &roots("/tmp/first")).unwrap();
    let moved = canonical(&make("/tmp/second"), &roots("/tmp/second")).unwrap();
    assert_eq!(first.bytes().unwrap(), moved.bytes().unwrap());
    assert_eq!(first.graph["nodes"][0]["project"], "@repo:sub");
    assert_eq!(
        first.graph["boundaries"]["routes"][0]["application"],
        ":sub"
    );
    let hydrated = first.hydrate(&roots("/tmp/local")).unwrap();
    assert_eq!(hydrated.nodes[0].project, "local:sub");
    assert_eq!(hydrated.folder_project_builds, vec!["/tmp/local/"]);
    assert_eq!(hydrated.identity_roots, vec!["/tmp/local/"]);
    let directory = tempfile::tempdir().unwrap();
    let store = crate::store::SqliteStore::open(directory.path()).unwrap();
    store.publish(&hydrated, &AtomicBool::new(false)).unwrap();
    let restored = store.load_active().unwrap().unwrap();
    assert_eq!(
        first.bytes().unwrap(),
        canonical(&restored, &roots("/tmp/local"))
            .unwrap()
            .bytes()
            .unwrap()
    );
}

#[test]
fn typescript_file_display_paths_follow_named_logical_roots() {
    let make = |external: &str| {
        let mut graph = fixture("/checkout");
        for node in &mut graph.nodes {
            node.language = "typescript".into();
            node.build = "/checkout/tsconfig.json".into();
            node.file = format!("/{external}/Program.ts");
            node.container = format!("../{external}/Program.ts");
            node.namespace = format!("../{external}");
        }
        let keys = graph
            .nodes
            .iter()
            .map(|node| (node.id.clone(), node.label.clone()))
            .collect();
        (graph, keys)
    };
    let (first, keys) = make("externalOne");
    let first_roots = vec![
        ("repo".into(), "/checkout".into()),
        ("ref".into(), "/externalOne".into()),
    ];
    let before = canonical_with_identities(&first, &first_roots, &keys).unwrap();
    let (second, keys) = make("externalTwo");
    let second_roots = vec![
        ("repo".into(), "/checkout".into()),
        ("ref".into(), "/externalTwo".into()),
    ];
    let after = canonical_with_identities(&second, &second_roots, &keys).unwrap();
    assert_eq!(before.bytes().unwrap(), after.bytes().unwrap());
    assert_eq!(before.graph["nodes"][0]["container"], "@ref/Program.ts");
    assert_eq!(before.graph["nodes"][0]["namespace"], "@ref");
    let hydrated = before.hydrate(&second_roots).unwrap();
    assert_eq!(hydrated.nodes[0].container, "../externalTwo/Program.ts");
}

#[test]
fn compact_output_and_opaque_extensions_preserve_contract() {
    let mut artifact = canonical(&fixture("/tmp/a"), &roots("/tmp/a")).unwrap();
    let opaque =
        serde_json::json!({"file":"/not/a/known/path", "symbol":"unknown", "targets":["z","a"]});
    artifact.graph["extension"] = opaque.clone();
    artifact.graph["nodes"][0]["extension"] = opaque.clone();
    artifact
        .extensions
        .insert("extension".into(), opaque.clone());
    let bytes = artifact.bytes().unwrap();
    assert_eq!(bytes.iter().filter(|&&b| b == b'\n').count(), 1);
    assert_eq!(bytes.last(), Some(&b'\n'));
    let restored = Artifact::from_bytes(&bytes).unwrap();
    assert_eq!(restored.graph["extension"], opaque);
    assert_eq!(restored.graph["nodes"][0]["extension"], opaque);
    assert_eq!(restored.extensions["extension"], opaque);
    restored.hydrate(&roots("/tmp/relocated")).unwrap();
    let pretty = serde_json::to_vec_pretty(&artifact).unwrap();
    assert!(
        compare(&restored, &Artifact::from_bytes(&pretty).unwrap())
            .unwrap()
            .equal
    );
}

#[test]
fn reserved_extension_names_cannot_override_validated_artifact_fields() {
    for name in ["schema", "graph"] {
        let mut artifact = canonical(&fixture("/tmp/a"), &roots("/tmp/a")).unwrap();
        artifact
            .extensions
            .insert(name.into(), serde_json::json!("invalid"));
        assert!(artifact.bytes().is_err());
        assert!(artifact.encode().is_err());
    }
}
