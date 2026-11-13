//! Real build exporters preserve evaluated output spelling alongside canonical ownership.
use codecallnavigator_engine::{
    analyzers::{AnalyzerRuntime, analyze},
    discovery::BuildTarget,
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
};

struct Fixture {
    _directory: tempfile::TempDir,
    root: PathBuf,
    metadata: PathBuf,
}

impl Fixture {
    fn new(kind: &str, aliased: bool) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let base = directory.path().canonicalize().unwrap();
        let root = base.join("project");
        fs::create_dir_all(root.join("src/main/java/probe")).unwrap();
        fs::write(root.join("src/main/java/probe/App.java"), "package probe; public class App { public static void main(String[] args) { leaf(); } public static void leaf() {} }").unwrap();
        if kind == "maven" {
            write_maven(&root, aliased);
        } else {
            write_gradle(&root, aliased);
        }
        if aliased {
            let output = root.join(if kind == "maven" { "target" } else { "build" });
            fs::create_dir_all(output.join("actual-output")).unwrap();
            std::os::unix::fs::symlink("actual-output", output.join("output-alias")).unwrap();
        }
        Self {
            _directory: directory,
            root,
            metadata: base.join("metadata.json"),
        }
    }

    fn export(&self, kind: &str) -> Vec<Value> {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
        let java = AnalyzerRuntime::discover().unwrap().java_home.unwrap();
        let mut command = if kind == "maven" {
            let mut command = Command::new("mvn");
            command
                .args(["--batch-mode", "--no-transfer-progress", "-f"])
                .arg(self.root.join("pom.xml"))
                .arg(format!(
                    "-Dmaven.ext.class.path={}",
                    repository
                        .join(
                            "tools/maven-metadata/build/libs/codecallnavigator-maven-metadata.jar"
                        )
                        .display()
                ))
                .arg(format!(
                    "-Dcodecallnavigator.metadata.initial={}",
                    self.metadata.display()
                ))
                .arg("validate");
            command
        } else {
            let mut command = Command::new(repository.join("fixtures/gradle/gradlew"));
            command
                .args(["--no-daemon", "--console=plain", "-p"])
                .arg(&self.root)
                .arg("-I")
                .arg(repository.join("tools/gradle-metadata.gradle"))
                .arg(format!(
                    "-PcodecallnavigatorMetadataFile={}",
                    self.metadata.display()
                ))
                .arg("codecallnavigatorExportMetadata");
            command
        };
        let output = command
            .current_dir(&self.root)
            .env("JAVA_HOME", java)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let exported: Value = serde_json::from_slice(&fs::read(&self.metadata).unwrap()).unwrap();
        if kind == "maven" {
            exported["sourceSets"].as_array().unwrap().clone()
        } else {
            exported.as_array().unwrap().clone()
        }
    }

    fn manual_analysis(&self, kind: &str) {
        let graph = analyze(
            vec![BuildTarget {
                path: self.root.clone(),
                language: "java".into(),
                kind: kind.into(),
                label: "output alias acceptance".into(),
            }],
            &AnalyzerRuntime::discover().unwrap(),
            &AtomicBool::new(false),
            &|message| eprintln!("{message}"),
        )
        .unwrap();
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.signature.contains("probe.App:main("))
        );
    }
}

fn write_maven(root: &Path, aliased: bool) {
    let output = if aliased {
        "<outputDirectory>${project.build.directory}/output-alias</outputDirectory>"
    } else {
        ""
    };
    fs::write(root.join("pom.xml"), format!(r#"<project xmlns="http://maven.apache.org/POM/4.0.0"><modelVersion>4.0.0</modelVersion><groupId>probe</groupId><artifactId>output-metadata</artifactId><version>1</version><properties><maven.compiler.release>25</maven.compiler.release></properties><build>{output}<plugins><plugin><groupId>org.apache.maven.plugins</groupId><artifactId>maven-compiler-plugin</artifactId><version>3.14.1</version></plugin><plugin><groupId>org.apache.maven.plugins</groupId><artifactId>maven-resources-plugin</artifactId><version>3.3.1</version></plugin></plugins></build></project>"#)).unwrap();
}

fn write_gradle(root: &Path, aliased: bool) {
    let output = if aliased {
        "sourceSets.main.java.destinationDirectory.set(layout.buildDirectory.dir('output-alias'))"
    } else {
        ""
    };
    fs::write(
        root.join("settings.gradle"),
        "rootProject.name='output-metadata'\n",
    )
    .unwrap();
    fs::write(root.join("build.gradle"), format!("plugins {{ id 'java' }}\njava {{ toolchain {{ languageVersion=JavaLanguageVersion.of(25) }} }}\n{output}\n")).unwrap();
}

fn assert_alias(kind: &str) {
    let fixture = Fixture::new(kind, true);
    let sets = fixture.export(kind);
    for source in &sets {
        assert!(
            source["rawClassDirs"].is_array(),
            "Missing raw output spelling: {source}"
        );
    }
    let main = sets
        .iter()
        .find(|source| source["sourceSet"] == "main")
        .unwrap();
    let output = fixture
        .root
        .join(if kind == "maven" { "target" } else { "build" });
    assert_eq!(
        main["rawClassDirs"],
        serde_json::json!([output.join("output-alias")])
    );
    assert_eq!(
        main["classDirs"],
        serde_json::json!([output.join("actual-output")])
    );
    assert!(
        fs::symlink_metadata(output.join("output-alias"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    // Gradle may replace its output symlink during compilation. Manual analysis
    // retains that existing build-tool behavior; only automatic policy rejects it.
    fixture.manual_analysis(kind);
}

#[test]
#[ignore = "requires real Maven/Gradle exporters, bootstrapped Java resources and JDK25"]
fn real_gradle_output_alias_retains_raw_spelling_and_manual_analysis() {
    assert_alias("gradle");
}

#[test]
#[ignore = "requires real Maven/Gradle exporters, bootstrapped Java resources and JDK25"]
fn real_maven_output_alias_retains_raw_spelling_and_manual_analysis() {
    assert_alias("maven");
}

#[test]
#[ignore = "requires real Maven/Gradle exporters and JDK25"]
fn ordinary_source_sets_export_raw_outputs_too() {
    for kind in ["maven", "gradle"] {
        let fixture = Fixture::new(kind, false);
        for source in fixture.export(kind) {
            assert_eq!(
                source["rawClassDirs"], source["classDirs"],
                "Missing/default spelling changed: {source}"
            );
        }
    }
}
