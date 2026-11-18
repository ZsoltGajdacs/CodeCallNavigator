//! A real JSR269 processor exercises physical output ownership through aliases.
use codecallnavigator_engine::analyzers::AnalyzerRuntime;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub fn maven_generated_output_alias(root: &Path) -> PathBuf {
    let work = root.parent().unwrap().join("processor-setup");
    let classes = work.join("classes");
    fs::create_dir_all(&classes).unwrap();
    let source = work.join("Generate.java");
    fs::write(&source, PROCESSOR).unwrap();
    let java = AnalyzerRuntime::discover().unwrap().java_home.unwrap();
    run_tool(
        Command::new(java.join("bin/javac"))
            .args(["-proc:none", "--release", "17", "-d"])
            .arg(&classes)
            .arg(source),
    );
    run_tool(
        Command::new(java.join("bin/jar"))
            .args(["--create", "--file"])
            .arg(root.join("processor.jar"))
            .arg("-C")
            .arg(classes)
            .arg("."),
    );
    let physical = root.join("target/ap-actual");
    let declared = root.join("target/ap-alias");
    fs::create_dir_all(&physical).unwrap();
    std::os::unix::fs::symlink(&physical, &declared).unwrap();
    // Maven may replace the output symlink while cleaning annotation output.
    // Keep both its former location and the active configured output reachable.
    std::os::unix::fs::symlink(
        &physical,
        root.join("src/main/java/old-generated-location-link"),
    )
    .unwrap();
    std::os::unix::fs::symlink(&declared, root.join("src/main/java/generated-source-link"))
        .unwrap();
    configure_maven(root);
    declared.join("probe/Generated.java")
}

fn run_tool(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn configure_maven(root: &Path) {
    let pom = root.join("pom.xml");
    let original = fs::read_to_string(&pom).unwrap();
    let compiler = "<artifactId>maven-compiler-plugin</artifactId><version>3.14.1</version>";
    let options = r#"<configuration><generatedSourcesDirectory>${project.build.directory}/ap-alias</generatedSourcesDirectory><annotationProcessors><annotationProcessor>fixture.processor.Generate</annotationProcessor></annotationProcessors></configuration>"#;
    let dependency = r#"<dependencies><dependency><groupId>codecallnavigator.fixture</groupId><artifactId>processor</artifactId><version>1</version><scope>system</scope><systemPath>${project.basedir}/processor.jar</systemPath></dependency></dependencies>"#;
    assert!(original.contains(compiler));
    let configured = original.replace(compiler, &format!("{compiler}{options}"));
    fs::write(
        pom,
        configured.replace("<build>", &format!("{dependency}<build>")),
    )
    .unwrap();
}

const PROCESSOR: &str = r#"
package fixture.processor;
import java.io.IOException;
import java.io.Writer;
import java.util.Set;
import javax.annotation.processing.*;
import javax.lang.model.SourceVersion;
import javax.lang.model.element.TypeElement;
@SupportedAnnotationTypes("*")
public final class Generate extends AbstractProcessor {
    private boolean emitted;
    @Override public SourceVersion getSupportedSourceVersion() { return SourceVersion.latestSupported(); }
    @Override public boolean process(Set<? extends TypeElement> annotations, RoundEnvironment round) {
        if (!emitted && !round.processingOver()) {
            emitted = true;
            try (Writer output = processingEnv.getFiler().createSourceFile("probe.Generated").openWriter()) {
                output.write("package probe; public class Generated { public static void generated() {} }");
            } catch (IOException error) { throw new IllegalStateException(error); }
        }
        return false;
    }
}
"#;
