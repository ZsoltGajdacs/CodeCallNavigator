//! Real build producers and late mutations for compiler-consumption boundaries.
use std::{
    fs,
    path::{Path, PathBuf},
};

const PRODUCED_BEFORE: &str =
    "package fixture; public class Produced { public static int before() { return 1; } }";
const PRODUCED_AFTER: &str =
    "package fixture; public class Produced { public static int generated() { return 2; } }";
const GENERATED: &str =
    "package fixture; public class GeneratedThing { public static void generated() {} }";

pub fn maven_producer(root: &Path) -> PathBuf {
    let app = root.join("app");
    let inputs = app.join("producer-inputs");
    fs::create_dir(&inputs).unwrap();
    fs::write(inputs.join("Produced.template"), PRODUCED_AFTER).unwrap();
    fs::write(inputs.join("GeneratedThing.template"), GENERATED).unwrap();
    let produced = app.join("src/main-custom/java/fixture/Produced.java");
    fs::write(&produced, PRODUCED_BEFORE).unwrap();
    // Source-root walking must reconcile stable non-Java links as well as .java leaves.
    let note = root.join("producer-note.txt");
    fs::write(&note, "Owned non-Java source-root neighbor\n").unwrap();
    std::os::unix::fs::symlink(note, app.join("src/main-custom/java/fixture/README.txt")).unwrap();
    let pom = app.join("pom.xml");
    let before = fs::read_to_string(&pom).unwrap();
    let generated_echo = format!(
        "<echo file=\"${{project.build.directory}}/generated-sources/fixture/fixture/GeneratedThing.java\">{GENERATED}</echo>"
    );
    let copies = r#"<copy file="${project.basedir}/producer-inputs/GeneratedThing.template" tofile="${project.build.directory}/generated-sources/fixture/fixture/GeneratedThing.java" overwrite="true" preservelastmodified="true"/>
          <copy file="${project.basedir}/producer-inputs/Produced.template" tofile="${project.basedir}/src/main-custom/java/fixture/Produced.java" overwrite="true" preservelastmodified="true"/>"#;
    assert!(before.contains(&generated_echo));
    fs::write(
        pom,
        before.replace(&generated_echo, copies).replace(
            "<resources>",
            "<resources><resource><directory>producer-inputs</directory></resource>",
        ),
    )
    .unwrap();
    produced
}

pub fn gradle_producer_and_cache(root: &Path) -> PathBuf {
    let app = root.join("app");
    fs::create_dir(app.join("producer-inputs")).unwrap();
    fs::write(
        app.join("producer-inputs/Produced.template"),
        PRODUCED_AFTER,
    )
    .unwrap();
    let produced = app.join("src/main/java/fixture/Produced.java");
    fs::write(&produced, PRODUCED_BEFORE).unwrap();
    let script = root.join("build.gradle");
    let original = fs::read_to_string(&script).unwrap();
    fs::write(script, format!("{original}\n{GRADLE_OBSERVATIONS}")).unwrap();
    let settings = root.join("settings.gradle");
    let original = fs::read_to_string(&settings).unwrap();
    fs::write(settings, format!("{original}\nbuildCache {{ local {{ directory = new File(rootDir, 'build/native-cache') }} }}\n")).unwrap();
    fs::write(
        root.join("gradle.properties"),
        "org.gradle.caching=true\norg.gradle.configuration-cache=true\n",
    )
    .unwrap();
    produced
}

const GRADLE_OBSERVATIONS: &str = r#"
project(':app') {
    def produced = file('src/main/java/fixture/Produced.java')
    def template = file('producer-inputs/Produced.template')
    tasks.named('generateFixture') {
        inputs.file(template)
        outputs.file(produced)
        doLast { produced.bytes = template.bytes }
    }
}
allprojects {
    tasks.withType(org.gradle.api.tasks.compile.JavaCompile).configureEach {
        doLast {
            def marker = layout.buildDirectory.file('compile-actions.txt').get().asFile
            marker.parentFile.mkdirs()
            marker.append(path + '\n')
        }
    }
}
gradle.taskGraph.afterTask { task, state ->
    if (task instanceof org.gradle.api.tasks.compile.JavaCompile) {
        def marker = rootProject.layout.buildDirectory.file('native-outcomes.txt').get().asFile
        marker.parentFile.mkdirs()
        marker.append(task.path + '\t' + String.valueOf(state.skipMessage) + '\n')
    }
}
"#;

pub fn maven_execution(root: &Path, phase: &str, actions: &str) -> String {
    let pom = root.join("pom.xml");
    let original = fs::read_to_string(&pom).unwrap();
    let build = format!(
        r#"<build><plugins><plugin><groupId>org.apache.maven.plugins</groupId><artifactId>maven-antrun-plugin</artifactId><version>3.2.0</version><executions><execution><phase>{phase}</phase><goals><goal>run</goal></goals><configuration><target>{actions}</target></configuration></execution></executions></plugin></plugins></build>"#
    );
    fs::write(
        pom,
        original.replace("</project>", &format!("{build}</project>")),
    )
    .unwrap();
    original
}

pub fn maven_late_source_mutation(root: &Path, source: &Path) -> String {
    let actions = mutation_actions(root, source, "disconnected", "disConnected");
    maven_execution(root, "process-test-resources", &actions)
}

fn mutation_actions(root: &Path, source: &Path, before: &str, after: &str) -> String {
    fs::write(root.join("SourceMutation.java"), MUTATOR).unwrap();
    let relative = source.strip_prefix(root).unwrap().to_string_lossy();
    format!(
        r#"<exec executable="${{java.home}}/bin/java" failonerror="true"><arg value="${{project.basedir}}/SourceMutation.java"/><arg value="${{project.basedir}}/{relative}"/><arg value="{before}"/><arg value="{after}"/></exec>"#
    )
}

pub fn maven_late_generated_mutation(root: &Path) -> String {
    let app = root.join("app");
    let source = app.join("target/generated-sources/fixture/fixture/GeneratedThing.java");
    let actions = mutation_actions(&app, &source, "generated()", "lateValue()");
    let pom = app.join("pom.xml");
    let original = fs::read_to_string(&pom).unwrap();
    let late = format!(
        "<execution><id>late-consumed-input</id><phase>process-test-resources</phase><goals><goal>run</goal></goals><configuration><target>{actions}</target></configuration></execution>"
    );
    assert!(original.contains("<executions>"));
    fs::write(
        pom,
        original.replacen("<executions>", &format!("<executions>{late}"), 1),
    )
    .unwrap();
    original
}

const MUTATOR: &str = r#"
import java.nio.file.*;
import java.nio.file.attribute.FileTime;
import java.nio.charset.StandardCharsets;
class SourceMutation {
    public static void main(String[] args) throws Exception {
        Path path = Path.of(args[0]);
        FileTime modified = Files.getLastModifiedTime(path);
        String before = Files.readString(path);
        String after = before.replace(args[1], args[2]);
        if (before.equals(after) || before.length() != after.length()) throw new IllegalStateException("Mutation was not equal-sized");
        Files.write(path, after.getBytes(StandardCharsets.UTF_8));
        Files.setLastModifiedTime(path, modified);
    }
}
"#;

pub fn gradle_late_source_mutation(root: &Path) -> String {
    let script = root.join("build.gradle");
    let original = fs::read_to_string(&script).unwrap();
    let mutation = r#"
project(':app') {
    tasks.named('compileJava') {
        doLast {
            def source = file('src/main/java/fixture/App.java')
            def modified = java.nio.file.Files.getLastModifiedTime(source.toPath())
            String before = source.getText('UTF-8')
            String after = before.replace('disconnected', 'disConnected')
            assert before != after && before.length() == after.length()
            source.setText(after, 'UTF-8')
            java.nio.file.Files.setLastModifiedTime(source.toPath(), modified)
        }
    }
}
"#;
    fs::write(script, format!("{original}\n{mutation}")).unwrap();
    original
}

pub fn maven_json_producer(root: &Path) -> PathBuf {
    let frontend = root.join("frontend");
    fs::create_dir(&frontend).unwrap();
    fs::write(frontend.join("value.json"), r#"{"kind":"before"}"#).unwrap();
    fs::write(
        frontend.join("main.ts"),
        "import value from './value.json'; export function producedKind() { return value.kind; }\n",
    )
    .unwrap();
    let config = frontend.join("tsconfig.json");
    fs::write(&config, r#"{"compilerOptions":{"strict":true,"target":"ES2022","module":"ESNext","moduleResolution":"Bundler","resolveJsonModule":true,"allowSyntheticDefaultImports":true,"skipLibCheck":true,"noEmit":true,"types":[]},"files":["main.ts"]}"#).unwrap();
    let compiler = std::env::var_os("CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../PacificFateMapEditor/node_modules/typescript")
        })
        .canonicalize()
        .expect(
            "Set CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT to a supported real TypeScript compiler",
        );
    fs::create_dir(frontend.join("node_modules")).unwrap();
    std::os::unix::fs::symlink(compiler, frontend.join("node_modules/typescript")).unwrap();
    maven_execution(
        root,
        "generate-resources",
        r#"<echo file="${project.basedir}/frontend/value.json">{"kind":1}</echo>"#,
    );
    config
}
