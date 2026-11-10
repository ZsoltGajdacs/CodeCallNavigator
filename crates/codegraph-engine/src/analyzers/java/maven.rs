//! Maven build execution; evaluated ownership comes from the bundled extension.
use super::build::JavaBuildMetadata;
use crate::{
    analyzers::{AnalysisJob, choose_artifact, find_tool, process::run_process},
    discovery::BuildTarget,
};
use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(super) fn metadata(
    build: &BuildTarget,
    context: &AnalysisJob<'_>,
) -> Result<JavaBuildMetadata> {
    (context.progress)(format!(
        "Reading Maven reactor ownership for {}…",
        build.label
    ));
    task(build, context, "validate", "maven-metadata")?;
    read(context, "maven-initial.json")
}

pub(super) fn compile(build: &BuildTarget, context: &AnalysisJob<'_>) -> Result<JavaBuildMetadata> {
    // Never accept metadata left behind by an earlier, failed invocation.
    let final_path = context.path.join("maven-final.json");
    if final_path.exists() {
        fs::remove_file(&final_path)?;
    }
    task(build, context, "test-compile", "maven-compile")?;
    read(context, "maven-final.json")
}

fn read(context: &AnalysisJob<'_>, name: &str) -> Result<JavaBuildMetadata> {
    let path = context.path.join(name);
    serde_json::from_slice(
        &fs::read(&path).with_context(|| {
            format!("Maven did not export complete metadata: {}", path.display())
        })?,
    )
    .context("Invalid Maven ownership metadata")
}

fn task(build: &BuildTarget, context: &AnalysisJob<'_>, goal: &str, log: &str) -> Result<()> {
    let executable = maven_command(&build.path)?;
    let extension = choose_artifact(
        &context.runtime.bundle_root,
        &[
            "analyzers/maven-metadata.jar",
            "tools/maven-metadata/build/libs/codegraph-maven-metadata.jar",
        ],
    )?;
    let mut extensions = existing_extensions(&build.path)?;
    extensions.push(extension);
    let mut command = Command::new(executable);
    command
        .current_dir(&build.path)
        .env(
            "JAVA_HOME",
            context
                .runtime
                .java_home
                .as_ref()
                .context("Java analysis requires an installed JDK 25")?,
        )
        .arg("--batch-mode")
        .arg("--no-transfer-progress")
        .arg("-f")
        .arg(build.path.join("pom.xml"))
        .arg(format!(
            "-Dmaven.ext.class.path={}",
            std::env::join_paths(extensions)?.to_string_lossy()
        ))
        .arg(format!(
            "-Dcodegraph.metadata.initial={}",
            context.path.join("maven-initial.json").display()
        ))
        .arg(format!(
            "-Dcodegraph.metadata.final={}",
            context.path.join("maven-final.json").display()
        ))
        .args([
            "-DskipTests=true",
            "-DskipITs=true",
            "-Dmaven.test.skip=false",
            "-Dmaven.main.skip=false",
        ])
        .arg(goal);
    run_process(&mut command, context.path, log, context.cancel)?;
    Ok(())
}

fn maven_command(root: &Path) -> Result<PathBuf> {
    for directory in root.ancestors() {
        let wrapper = directory.join("mvnw");
        if wrapper.is_file() {
            return Ok(wrapper);
        }
    }
    find_tool("mvn")
        .context("No Maven Wrapper or installed Maven found; install Maven 3.9+ or Maven 4")
}

fn existing_extensions(root: &Path) -> Result<Vec<PathBuf>> {
    let project = project_arguments(root)?;
    let opts = std::env::var("MAVEN_OPTS").ok();
    let args = std::env::var("MAVEN_ARGS").ok();
    Ok(
        effective_extensions(project, opts.as_deref(), args.as_deref())?
            .into_iter()
            .map(|path| {
                if path.is_absolute() {
                    path
                } else {
                    root.join(path)
                }
            })
            .collect(),
    )
}

#[derive(Default)]
struct ProjectArguments {
    jvm: Vec<String>,
    maven: Vec<String>,
}

/// Maven user properties override JVM properties; the last definition wins
/// within each scope. Preserve that effective classpath before appending ours.
fn effective_extensions(
    mut project: ProjectArguments,
    opts: Option<&str>,
    args: Option<&str>,
) -> Result<Vec<PathBuf>> {
    project.jvm.extend(extension_arguments(opts)?);
    project.maven.extend(extension_arguments(args)?);
    Ok(extension_value(&project.maven)
        .or_else(|| extension_value(&project.jvm))
        .into_iter()
        .flat_map(std::env::split_paths)
        .filter(|path| !path.as_os_str().is_empty())
        .collect())
}

fn extension_arguments(value: Option<&str>) -> Result<Vec<String>> {
    match value.filter(|value| value.contains("maven.ext.class.path")) {
        Some(value) => arguments(value),
        None => Ok(Vec::new()),
    }
}

fn extension_value(arguments: &[String]) -> Option<&str> {
    arguments
        .iter()
        .enumerate()
        .filter_map(|(index, argument)| {
            let definition = match argument.as_str() {
                "-D" | "--define" => arguments.get(index + 1)?.as_str(),
                argument => argument
                    .strip_prefix("-D")
                    .or_else(|| argument.strip_prefix("--define="))?,
            };
            let (name, value) = definition.split_once('=').unwrap_or((definition, "true"));
            (name == "maven.ext.class.path").then_some(value)
        })
        .next_back()
}

fn project_arguments(root: &Path) -> Result<ProjectArguments> {
    let Some(directory) = root.ancestors().find(|path| path.join(".mvn").is_dir()) else {
        return Ok(ProjectArguments::default());
    };
    let configuration = directory.join(".mvn");
    let mut result = ProjectArguments::default();
    let jvm = configuration.join("jvm.config");
    if jvm.is_file() {
        let value = fs::read_to_string(jvm)?;
        result.jvm = extension_arguments(Some(&value))?;
    }
    let maven = configuration.join("maven.config");
    if maven.is_file() {
        let value = fs::read_to_string(maven)?;
        result.maven.extend(
            value
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(str::to_owned),
        );
    }
    Ok(result)
}

fn arguments(input: &str) -> Result<Vec<String>> {
    shlex::split(input).context("Malformed Maven argument quoting")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extension_classpath_preserves_quoted_paths() {
        assert_eq!(
            arguments("-Xmx2g '-Dmaven.ext.class.path=/path with spaces/ext.jar'\n-Dother=true")
                .unwrap(),
            vec![
                "-Xmx2g",
                "-Dmaven.ext.class.path=/path with spaces/ext.jar",
                "-Dother=true"
            ]
        );
        assert!(arguments("'unterminated").is_err());
    }
    #[test]
    fn native_maven_config_uses_one_literal_argument_per_line() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join(".mvn")).unwrap();
        fs::write(
            root.path().join(".mvn/maven.config"),
            "# comment\n-Dmessage=don't\n-Dmaven.ext.class.path=/path with spaces/ext.jar\n",
        )
        .unwrap();
        assert_eq!(
            project_arguments(root.path()).unwrap().maven,
            vec![
                "-Dmessage=don't",
                "-Dmaven.ext.class.path=/path with spaces/ext.jar"
            ]
        );
    }

    #[test]
    fn effective_extension_definitions_override_instead_of_accumulating() {
        let project = || ProjectArguments {
            jvm: vec!["-Dmaven.ext.class.path=/jvm.jar".into()],
            maven: Vec::new(),
        };
        assert_eq!(
            effective_extensions(project(), Some("-Dmaven.ext.class.path=/opts.jar"), None)
                .unwrap(),
            vec![PathBuf::from("/opts.jar")]
        );
        let configured = || ProjectArguments {
            maven: vec!["-Dmaven.ext.class.path=/config.jar".into()],
            ..project()
        };
        assert_eq!(
            effective_extensions(configured(), Some("-Dmaven.ext.class.path=/opts.jar"), None)
                .unwrap(),
            vec![PathBuf::from("/config.jar")]
        );
        assert_eq!(
            effective_extensions(
                configured(),
                Some("-Dmaven.ext.class.path=/opts.jar"),
                Some("-Dmaven.ext.class.path=/superseded.jar -Dmaven.ext.class.path=/one.jar:/two.jar")
            )
            .unwrap(),
            vec![PathBuf::from("/one.jar"), PathBuf::from("/two.jar")]
        );
        assert!(
            effective_extensions(configured(), None, Some("-Dmaven.ext.class.path="))
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            effective_extensions(
                configured(),
                None,
                Some("-D maven.ext.class.path=/separate.jar")
            )
            .unwrap(),
            vec![PathBuf::from("/separate.jar")]
        );
    }

    #[test]
    fn nearest_ancestor_wrapper_wins() {
        let root = tempfile::tempdir().unwrap();
        let child = root.path().join("module/nested");
        fs::create_dir_all(&child).unwrap();
        fs::write(root.path().join("mvnw"), "outer").unwrap();
        fs::write(root.path().join("module/mvnw"), "inner").unwrap();
        assert_eq!(
            maven_command(&child).unwrap(),
            root.path().join("module/mvnw")
        );
    }
}
