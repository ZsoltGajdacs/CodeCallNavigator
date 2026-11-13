//! Real MSBuild restore/import provenance, including timestamp-only rewrites.
use serde_json::Value;
use std::{fs, path::Path, process::Command};

const PROJECT: &str = "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net11.0</TargetFramework></PropertyGroup></Project>";

fn command(root: &Path, operation: &str, imports: Option<&Path>) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"));
    command.args([operation, "--project"]).arg(root);
    if let Some(imports) = imports {
        command
            .arg("--root")
            .arg(format!("imports={}", imports.display()));
    }
    command
}

fn analyze(root: &Path) -> Value {
    analyze_with_imports(root, None)
}

fn analyze_with_imports(root: &Path, imports: Option<&Path>) -> Value {
    analyze_target(root, "Fixture.csproj", imports)
}

fn analyze_target(root: &Path, target: &str, imports: Option<&Path>) -> Value {
    let output = command(root, "analyze", imports)
        .arg(target)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&fs::read(root.join(".codecallnavigator/graph.json")).unwrap()).unwrap()
}

fn project(root: &Path) {
    fs::create_dir_all(root).unwrap();
    fs::write(root.join("Fixture.csproj"), PROJECT).unwrap();
    fs::write(
        root.join("App.cs"),
        "public class App { public static int Leaf() => 1; public static int Start() => Leaf(); }",
    )
    .unwrap();
}

fn input_paths(artifact: &Value) -> Vec<String> {
    artifact["graph"]["analysis"]["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|input| input["path"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET 11 SDK"]
fn cold_and_warm_import_coverage_survives_identical_project_rewrites() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    project(&root);
    let cold = analyze(&root);
    let generated = "@repo/obj/Fixture.csproj.nuget.g.targets";
    assert!(input_paths(&cold).iter().any(|path| path == generated));
    fs::write(root.join("Fixture.csproj"), PROJECT).unwrap();
    let warm = analyze(&root);
    assert!(
        input_paths(&warm).iter().any(|path| path == generated),
        "Generated import disappeared after identical project rewrite: {:?}",
        input_paths(&warm)
    );
    assert_eq!(cold, warm, "Canonical content must ignore input mtimes");
    retain_evidence(&cold, &warm);
    let independent = tempfile::tempdir().unwrap();
    project(independent.path());
    assert_eq!(
        warm,
        analyze(independent.path()),
        "Independent clean input coverage must match"
    );
}

fn retain_evidence(cold: &Value, warm: &Value) {
    if let Some(path) = std::env::var_os("CODECALLNAVIGATOR_CSHARP_INPUT_EVIDENCE") {
        let record = serde_json::json!({"cold_inputs": cold["graph"]["analysis"]["inputs"], "warm_inputs": warm["graph"]["analysis"]["inputs"], "engine_versions": warm["graph"]["analysis"]["engine_versions"], "limitations": warm["graph"]["analysis"]["limitations"], "full_artifact_equal": cold == warm});
        fs::write(path, serde_json::to_vec_pretty(&record).unwrap()).unwrap();
    }
}

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;")
}

fn imported_project(base: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let root = base.join("app--&");
    let imports = base.join("external--&");
    project(&root);
    fs::create_dir(&imports).unwrap();
    fs::write(base.join("Noise.props"), "<Project />").unwrap();
    fs::write(imports.join("Never.props"), "<Project />").unwrap();
    fs::write(imports.join("Inner--&.props"), "<Project><PropertyGroup><DefineConstants>$(DefineConstants);IMPORTED</DefineConstants></PropertyGroup></Project>").unwrap();
    fs::write(imports.join("Outer--&.props"), format!("<Project><!-- {} --><Import Project=\"$(MSBuildThisFileDirectory)Inner--&amp;.props\"/><Import Project=\"Never.props\" Condition=\"false\"/></Project>", base.join("Noise.props").display())).unwrap();
    let contents = PROJECT.replace(
        "</Project>",
        &format!(
            "<Import Project=\"{}\"/></Project>",
            xml("../external--&/Outer--&.props")
        ),
    );
    fs::write(root.join("Fixture.csproj"), contents).unwrap();
    fs::write(root.join("App.cs"), "public class App { public static int Start() => ImportedLeaf();\n#if IMPORTED\n public static int ImportedLeaf() => 1;\n#endif\n}").unwrap();
    (root, imports)
}

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET 11 SDK"]
fn nested_external_property_imports_are_observed_and_failed_refresh_retains_artifact() {
    let base = tempfile::tempdir().unwrap();
    let (root, imports) = imported_project(base.path());
    let artifact = analyze_with_imports(&root, Some(&imports));
    let paths = input_paths(&artifact);
    assert!(paths.iter().any(|path| path == "@imports/Outer--&.props"));
    assert!(paths.iter().any(|path| path == "@imports/Inner--&.props"));
    assert!(
        !paths
            .iter()
            .any(|path| path.contains("Never.props") || path.contains("Noise.props"))
    );
    assert!(
        artifact["graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["label"] == "ImportedLeaf")
    );
    fs::write(imports.join("Inner--&.props"), "<Project><PropertyGroup><DefineConstants>$(DefineConstants);IMPORTED;CHANGED</DefineConstants></PropertyGroup></Project>").unwrap();
    let verified = command(&root, "verify", Some(&imports)).output().unwrap();
    let report: Value = serde_json::from_slice(&verified.stdout).unwrap();
    assert_eq!(report["input_status"], "stale");
    assert!(
        report["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason["path"] == "@imports/Inner--&.props")
    );
    let saved = fs::read(root.join(".codecallnavigator/graph.json")).unwrap();
    fs::write(imports.join("Inner--&.props"), "<Project>").unwrap();
    let failed = command(&root, "analyze", Some(&imports))
        .arg("Fixture.csproj")
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert_eq!(
        fs::read(root.join(".codecallnavigator/graph.json")).unwrap(),
        saved
    );
}

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET 11 SDK"]
fn empty_external_referenced_project_scope_detects_new_sources_after_publication() {
    let base = tempfile::tempdir().unwrap();
    let root = base.path().join("app");
    let empty = base.path().join("empty");
    project(&root);
    fs::create_dir(&empty).unwrap();
    fs::write(empty.join("Empty.csproj"), PROJECT).unwrap();
    fs::write(root.join("Fixture.csproj"), PROJECT.replace("</Project>", "<ItemGroup><ProjectReference Include=\"../empty/Empty.csproj\" /></ItemGroup></Project>")).unwrap();
    let artifact = analyze_with_imports(&root, Some(&empty));
    assert!(
        input_paths(&artifact)
            .iter()
            .any(|path| path == "@imports/")
    );
    fs::write(
        empty.join("New.cs"),
        "public class New { public static int Leaf() => 2; }",
    )
    .unwrap();
    let output = command(&root, "verify", Some(&empty)).output().unwrap();
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["input_status"], "stale");
    assert!(
        report["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason["path"] == "@imports/")
    );
}

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET 11 SDK"]
fn design_time_imports_and_external_source_globs_are_observed() {
    let base = tempfile::tempdir().unwrap();
    let root = base.path().join("app");
    let external = base.path().join("external");
    project(&root);
    fs::create_dir_all(external.join("linked")).unwrap();
    fs::write(
        external.join("linked/Linked.cs"),
        "public class Linked { public static int DesignLinked() => 3; }",
    )
    .unwrap();
    fs::write(external.join("Design.props"), "<Project><PropertyGroup><DefineConstants>$(DefineConstants);DESIGN_ONLY</DefineConstants></PropertyGroup><ItemGroup><Compile Include=\"$(MSBuildThisFileDirectory)linked/**/*.cs\" /></ItemGroup></Project>").unwrap();
    fs::write(root.join("Fixture.csproj"), PROJECT.replace("</Project>", "<Import Project=\"../external/Design.props\" Condition=\"'$(DesignTimeBuild)' == 'true'\" /></Project>")).unwrap();
    fs::write(root.join("App.cs"), "public class App {\n#if DESIGN_ONLY\npublic static int DesignOnly() => Linked.DesignLinked();\n#endif\n}").unwrap();
    let artifact = analyze_with_imports(&root, Some(&external));
    let nodes = artifact["graph"]["nodes"].as_array().unwrap();
    assert!(
        nodes.iter().any(|node| node["label"] == "DesignOnly"),
        "Real Roslyn must consume the design-time import"
    );
    let paths = input_paths(&artifact);
    assert!(
        paths.iter().any(|path| path == "@imports/Design.props"),
        "Consumed design import missing from inputs: {paths:?}"
    );
    assert!(
        paths
            .iter()
            .any(|path| path.trim_end_matches('/') == "@imports/linked"),
        "Design-only source scope missing: {paths:?}"
    );
    fs::write(
        external.join("linked/New.cs"),
        "public class New { public static int Added() => 4; }",
    )
    .unwrap();
    let output = command(&root, "verify", Some(&external)).output().unwrap();
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["input_status"], "stale");
}

#[test]
#[ignore = "requires pinned CallAtlas, .NET 11 SDK and cached .NET 10 targeting pack"]
fn every_roslyn_target_framework_import_is_observed() {
    let base = tempfile::tempdir().unwrap();
    let root = base.path().join("app");
    let external = base.path().join("external");
    project(&root);
    fs::create_dir(&external).unwrap();
    fs::write(external.join("Ten.props"), "<Project><PropertyGroup><DefineConstants>$(DefineConstants);FRAMEWORK_TEN</DefineConstants></PropertyGroup></Project>").unwrap();
    fs::write(external.join("Eleven.props"), "<Project><PropertyGroup><DefineConstants>$(DefineConstants);FRAMEWORK_ELEVEN</DefineConstants></PropertyGroup></Project>").unwrap();
    fs::write(root.join("Fixture.csproj"), "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFrameworks>net10.0;net11.0</TargetFrameworks></PropertyGroup><ItemGroup><KnownFrameworkReference Update=\"Microsoft.NETCore.App\" Condition=\"'$(TargetFramework)' == 'net10.0'\" TargetingPackVersion=\"10.0.9\" /></ItemGroup><Import Project=\"../external/Ten.props\" Condition=\"'$(TargetFramework)' == 'net10.0'\" /><Import Project=\"../external/Eleven.props\" Condition=\"'$(TargetFramework)' == 'net11.0'\" /></Project>").unwrap();
    fs::write(root.join("App.cs"), "public class App {\n#if FRAMEWORK_TEN\npublic static int Ten() => 10;\n#endif\n#if FRAMEWORK_ELEVEN\npublic static int Eleven() => 11;\n#endif\n}").unwrap();
    let artifact = analyze_with_imports(&root, Some(&external));
    let paths = input_paths(&artifact);
    assert!(
        paths.iter().any(|path| path == "@imports/Ten.props"),
        "First inner build import missing: {paths:?}"
    );
    assert!(
        paths.iter().any(|path| path == "@imports/Eleven.props"),
        "Second inner build import missing: {paths:?}"
    );
}

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET 11 SDK"]
fn solution_directory_global_is_shared_with_roslyn_import_evaluation() {
    let base = tempfile::tempdir().unwrap();
    let (solution, external) = solution_project(base.path(), "solution");
    let artifact = analyze_target(&solution, "Fixture.slnx", Some(&external));
    assert!(
        artifact["graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["label"] == "FromSolution")
    );
    assert!(
        input_paths(&artifact)
            .iter()
            .any(|path| path == "@imports/Solution.props")
    );
}

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET 11 SDK"]
fn normal_configured_ownership_is_preserved_while_design_dependencies_are_collected() {
    let base = tempfile::tempdir().unwrap();
    let root = base.path().join("app");
    let external = base.path().join("external");
    project(&root);
    fs::create_dir_all(external.join("linked")).unwrap();
    fs::write(
        external.join("linked/Design.cs"),
        "public class Design { public static int DesignFallback() => 2; }",
    )
    .unwrap();
    fs::write(external.join("Design.props"), "<Project><PropertyGroup><DefineConstants>$(DefineConstants);DESIGN_IMPORT</DefineConstants></PropertyGroup><ItemGroup><Compile Include=\"$(MSBuildThisFileDirectory)linked/**/*.cs\" /></ItemGroup></Project>").unwrap();
    fs::write(root.join("Fixture.csproj"), PROJECT.replace("</Project>", "<PropertyGroup><IsTestProject>false</IsTestProject><IsTestProject Condition=\"'$(DesignTimeBuild)' == 'true'\">true</IsTestProject></PropertyGroup><ItemGroup><Compile Update=\"App.cs\" AutoGen=\"false\" /><Compile Update=\"App.cs\" AutoGen=\"true\" Condition=\"'$(DesignTimeBuild)' == 'true'\" /></ItemGroup><Import Project=\"../external/Design.props\" Condition=\"'$(DesignTimeBuild)' == 'true'\" /></Project>")).unwrap();
    let artifact = analyze_with_imports(&root, Some(&external));
    let nodes = artifact["graph"]["nodes"].as_array().unwrap();
    let leaf = nodes.iter().find(|node| node["label"] == "Leaf").unwrap();
    assert_eq!(
        leaf["is_test"], false,
        "Design metadata overwrote configured normal ownership"
    );
    assert_eq!(
        leaf["is_generated"], false,
        "Design metadata overwrote configured normal generated classification"
    );
    assert!(nodes.iter().any(|node| node["label"] == "DesignFallback"));
    assert!(
        input_paths(&artifact)
            .iter()
            .any(|path| path == "@imports/Design.props")
    );
    assert!(
        input_paths(&artifact)
            .iter()
            .any(|path| path.trim_end_matches('/') == "@imports/linked")
    );
}

fn solution_project(base: &Path, name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let solution = base.join(name);
    let root = solution.join("app");
    let external = base.join("shared");
    project(&root);
    fs::create_dir(&external).unwrap();
    fs::write(external.join("Solution.props"), "<Project><PropertyGroup><DefineConstants>$(DefineConstants);SOLUTION_IMPORT</DefineConstants></PropertyGroup></Project>").unwrap();
    fs::write(
        solution.join("Fixture.slnx"),
        "<Solution><Project Path=\"app/Fixture.csproj\" /></Solution>",
    )
    .unwrap();
    fs::write(root.join("Fixture.csproj"), PROJECT.replace("</Project>", "<Import Project=\"$(SolutionDir)../shared/Solution.props\" Condition=\"Exists('$(SolutionDir)../shared/Solution.props')\" /></Project>")).unwrap();
    fs::write(root.join("App.cs"), "public class App {\n#if SOLUTION_IMPORT\npublic static int FromSolution() => 1;\n#endif\n}").unwrap();
    (solution, external)
}

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET 11 SDK"]
fn solution_paths_with_property_separators_and_percent_retain_real_compiler_imports() {
    let base = tempfile::tempdir().unwrap();
    let (solution, external) =
        solution_project(base.path(), "solution;DesignTimeBuild=FALSE,part%literal");
    // Test actual design-time consumption independently of Roslyn's upstream
    // raw SolutionDir interpolation behavior for delimiter-containing paths.
    fs::write(solution.join("app/Fixture.csproj"), PROJECT.replace("</Project>", "<Import Project=\"../../shared/Solution.props\" Condition=\"'$(DesignTimeBuild)' == 'True'\" /></Project>")).unwrap();
    let artifact = analyze_target(&solution, "Fixture.slnx", Some(&external));
    assert!(
        artifact["graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["label"] == "FromSolution")
    );
    assert!(
        input_paths(&artifact)
            .iter()
            .any(|path| path == "@imports/Solution.props")
    );
}
