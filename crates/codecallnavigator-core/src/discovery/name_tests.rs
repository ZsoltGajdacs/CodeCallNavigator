//! Filesystem discovery contract: compact labels never change target identity.
use super::{discover, target_from_file};
use std::fs;

#[test]
fn names_are_compact_and_match_explicit_file_discovery() {
    let directory = tempfile::tempdir().unwrap();
    let folder = directory.path().join("billing");
    fs::create_dir(&folder).unwrap();
    let cases = [
        (
            "pom.xml",
            "<project><parent><artifactId>wrong</artifactId></parent><artifactId>billing-api</artifactId><name>Recurring &amp; Billing</name></project>",
            "billing / Recurring & Billing (Maven)",
        ),
        (
            "App.csproj",
            "<Project><PropertyGroup><ProjectName>Billing API</ProjectName><AssemblyName>Billing.Assembly</AssemblyName></PropertyGroup></Project>",
            "billing / Billing API (C# project)",
        ),
        ("App.sln", "", "billing / App (Solution)"),
        ("App.slnx", "<Solution/>", "billing / App (Solution)"),
        ("tsconfig.app.json", "{}", "billing / tsconfig.app.json"),
    ];
    for (file, contents, _) in cases {
        fs::write(folder.join(file), contents).unwrap();
    }
    let targets = discover(&folder).unwrap();
    for (file, _, expected) in cases {
        let direct = target_from_file(&folder.join(file)).unwrap();
        assert_eq!(direct.label, expected);
        let found = targets
            .iter()
            .find(|t| t.identity() == direct.identity())
            .unwrap();
        assert_eq!(found, &direct);
    }
}

#[test]
fn metadata_falls_back_without_evaluating_expressions_or_conditions() {
    let directory = tempfile::tempdir().unwrap();
    let folder = directory.path().join("billing");
    fs::create_dir(&folder).unwrap();
    for (contents, expected) in [
        (
            "<project><parent><name>Wrong</name><artifactId>wrong</artifactId></parent><dependencies><dependency><name>Wrong</name></dependency></dependencies><artifactId>billing-api</artifactId></project>",
            "billing-api",
        ),
        (
            "<project><name>${computed}</name><artifactId>billing-api</artifactId></project>",
            "billing-api",
        ),
        ("<project><name>   </name></project>", "billing"),
        ("malformed", "billing"),
    ] {
        fs::write(folder.join("pom.xml"), contents).unwrap();
        assert_eq!(
            target_from_file(&folder.join("pom.xml")).unwrap().label,
            format!("billing / {expected} (Maven)")
        );
    }
    for (contents, expected) in [
        (
            "<Project><PropertyGroup Condition=\"'$(Configuration)' == 'Debug'\"><ProjectName>Wrong</ProjectName></PropertyGroup><PropertyGroup><AssemblyName>Billing.Assembly</AssemblyName></PropertyGroup></Project>",
            "Billing.Assembly",
        ),
        (
            "<Project><PropertyGroup><ProjectName Condition=\"true\">Wrong</ProjectName><AssemblyName>Billing.Assembly</AssemblyName></PropertyGroup></Project>",
            "Billing.Assembly",
        ),
        (
            "<Project><PropertyGroup><ProjectName>$(Computed)</ProjectName><AssemblyName>$(Computed)</AssemblyName></PropertyGroup></Project>",
            "App",
        ),
        ("malformed", "App"),
    ] {
        fs::write(folder.join("App.csproj"), contents).unwrap();
        assert_eq!(
            target_from_file(&folder.join("App.csproj")).unwrap().label,
            format!("billing / {expected} (C# project)")
        );
    }
}

#[test]
fn gradle_only_reads_literal_root_names_from_settings() {
    let directory = tempfile::tempdir().unwrap();
    let folder = directory.path().join("billing");
    fs::create_dir(&folder).unwrap();
    fs::write(folder.join("build.gradle"), "rootProject.name = 'Wrong'").unwrap();
    for (contents, expected) in [
        (
            "// rootProject.name = 'Wrong'\nrootProject.name = 'Billing API'",
            "Billing API",
        ),
        (
            "/* rootProject.name = 'Wrong' */\nrootProject.name = \"Billing API\"",
            "Billing API",
        ),
        ("rootProject.name = \"${computed}\"", "billing"),
        ("rootProject.name = 'Billing' + suffix", "billing"),
        ("rootProject.name = nameFromElsewhere", "billing"),
        (
            "rootProject.name = 'Earlier'\nrootProject.name = nameFromElsewhere",
            "billing",
        ),
        (
            "if (enabled) {\nrootProject.name = 'Conditional'\n}",
            "billing",
        ),
        (
            "rootProject.name = 'Billing // API'; // comment",
            "Billing // API",
        ),
    ] {
        fs::write(folder.join("settings.gradle.kts"), contents).unwrap();
        let targets = discover(&folder).unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].label, format!("billing / {expected} (Gradle)"));
        assert_eq!(targets[0].path, folder.canonicalize().unwrap());
    }
}

#[test]
fn xml_namespaces_and_repeated_properties_preserve_readable_names() {
    let directory = tempfile::tempdir().unwrap();
    let folder = directory.path().join("billing");
    fs::create_dir(&folder).unwrap();
    for (file, source, expected) in [
        (
            "pom.xml",
            r#"<project xmlns="http://maven.apache.org/POM/4.0.0"><name>Billing <!-- explanation -->API</name></project>"#,
            "billing / Billing API (Maven)",
        ),
        (
            "App.csproj",
            r#"<Project xmlns="http://schemas.microsoft.com/developer/msbuild/2003"><PropertyGroup><ProjectName>Earlier</ProjectName><ProjectName>Current</ProjectName></PropertyGroup></Project>"#,
            "billing / Current (C# project)",
        ),
    ] {
        fs::write(folder.join(file), source).unwrap();
        assert_eq!(
            target_from_file(&folder.join(file)).unwrap().label,
            expected
        );
    }
}

#[test]
fn gradle_multiline_string_examples_do_not_become_project_names() {
    let directory = tempfile::tempdir().unwrap();
    let folder = directory.path().join("billing");
    fs::create_dir(&folder).unwrap();
    for quote in ["\"\"\"", "'''"] {
        fs::write(
            folder.join("settings.gradle.kts"),
            format!("val example = {quote}\nrootProject.name = 'Wrong example'\n{quote}"),
        )
        .unwrap();
        assert_eq!(
            discover(&folder).unwrap()[0].label,
            "billing / billing (Gradle)"
        );
    }
}
