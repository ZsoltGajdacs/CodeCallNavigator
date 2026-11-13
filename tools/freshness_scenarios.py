"""Owned, disposable compiler projects and saved-file edit journeys."""
import json
import shutil

SCENARIOS = ("body", "signature", "cross_file", "addition", "deletion", "rename", "atomic_save", "burst", "configuration")


class Project:
    def __init__(self, root, language, modules, repository):
        self.root, self.language = root, language
        self.modules, self.repository = modules, repository
        root.mkdir(parents=True, exist_ok=True)
        self.roots = []
        source, extra = self.configure()
        self.source, self.extra = root / source, root / extra
        self.source.parent.mkdir(parents=True, exist_ok=True)
        sibling = {"typescript": "sibling.ts", "csharp": "Sibling.cs"}.get(language, "Sibling.java")
        self.sibling = self.source.with_name(sibling)
        self.renamed = self.extra.with_name("renamed" + self.extra.suffix)

    def configure(self):
        if self.language == "typescript":
            return self.configure_typescript()
        if self.language == "csharp":
            self.target = "Freshness.csproj"
            self.config = '<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net11.0</TargetFramework></PropertyGroup></Project>'
            return "App.cs", "Extra.cs"
        self.configure_java()
        return "src/main/java/freshness/App.java", "src/main/java/freshness/Extra.java"

    def configure_typescript(self):
        if self.modules is None or not (self.modules / "typescript/package.json").is_file():
            raise ValueError("TypeScript requires --typescript-modules containing typescript")
        (self.root / "node_modules").symlink_to(self.modules, target_is_directory=True)
        self.roots = [["deps", str(self.modules)]]
        self.target = "tsconfig.json"
        self.config = json.dumps({"compilerOptions": {"strict": True, "target": "ES2022", "module": "NodeNext", "moduleResolution": "NodeNext"}, "include": ["*.ts"]})
        return "app.ts", "extra.ts"

    def configure_java(self):
        if self.language == "maven":
            self.target = "pom.xml"
            self.config = '<project xmlns="http://maven.apache.org/POM/4.0.0"><modelVersion>4.0.0</modelVersion><groupId>freshness</groupId><artifactId>probe</artifactId><version>1</version><properties><maven.compiler.release>25</maven.compiler.release><maven.compiler.source>25</maven.compiler.source><maven.compiler.target>25</maven.compiler.target></properties></project>'
        elif self.language == "gradle":
            self.configure_gradle()
        else:
            raise ValueError(f"Unknown language: {self.language}")

    def configure_gradle(self):
        self.target = "build.gradle.kts"
        self.config = 'plugins { java }\njava { toolchain { languageVersion = JavaLanguageVersion.of(25) } }\n'
        for name in ("gradlew", "gradlew.bat", "gradle"):
            origin = self.repository / "fixtures/gradle" / name
            if origin.is_dir():
                shutil.copytree(origin, self.root / name)
            elif origin.exists():
                shutil.copy2(origin, self.root / name)
        (self.root / "settings.gradle.kts").write_text('rootProject.name = "freshness-probe"\n')

    def text(self, value=1, parameter=False, cross=False):
        argument, parameters = ("1", "int ignored") if parameter else ("", "")
        if self.language == "typescript":
            parameters = "ignored: number" if parameter else ""
            target = "sibling" if cross else "leaf"
            return f'import {{sibling}} from "./sibling.js"; export function leaf({parameters}){{return {value};}} export function start(){{return {target}({argument if not cross else ""});}}\n'
        target = "Sibling.sibling" if cross else "leaf"
        body = f'public static int leaf({parameters}){{return {value};}} public static int start(){{return {target}({argument if not cross else ""});}}'
        return f'public class App {{{body}}}\n' if self.language == "csharp" else f'package freshness; public class App {{{body}}}\n'

    def reset(self, scenario=None):
        for path in (self.extra, self.renamed):
            path.unlink(missing_ok=True)
        self.source.write_text(self.text())
        sibling = 'export function sibling(){return 3;}\n' if self.language == "typescript" else 'public class Sibling { public static int sibling(){return 3;} }\n' if self.language == "csharp" else 'package freshness; public class Sibling { public static int sibling(){return 3;} }\n'
        self.sibling.write_text(sibling)
        (self.root / self.target).write_text(self.config)
        if scenario in ("deletion", "rename"):
            self.add_extra()

    def add_extra(self):
        content = 'export function extra(){return 9;}\n' if self.language == "typescript" else 'class Extra { public static int extra(){return 9;} }\n' if self.language == "csharp" else 'package freshness; class Extra { public static int extra(){return 9;} }\n'
        self.extra.write_text(content)

    def edit(self, scenario, iteration):
        value = iteration + 2
        if scenario == "addition":
            self.add_extra()
        elif scenario == "deletion":
            self.extra.unlink()
        elif scenario == "rename":
            self.extra.rename(self.renamed)
        elif scenario == "configuration":
            self.edit_configuration(value)
        else:
            self.edit_content(scenario, value)

    def edit_content(self, scenario, value):
        if scenario == "atomic_save":
            temporary = self.source.with_suffix(".saving")
            temporary.write_text(self.text(value))
            temporary.replace(self.source)
        elif scenario == "burst":
            for offset in range(10):
                self.source.write_text(self.text(value + offset))
        else:
            self.source.write_text(self.text(value, scenario == "signature", scenario == "cross_file"))

    def edit_configuration(self, value):
        path = self.root / self.target
        if self.language == "typescript":
            config = json.loads(self.config)
            config["compilerOptions"]["strict"] = False
            path.write_text(json.dumps(config))
        else:
            suffix = f'\n// saved edit {value}\n' if self.language == "gradle" else f'\n<!-- saved edit {value} -->\n'
            path.write_text(self.config + suffix)


def assert_observable_edit(scenario, before, after):
    """Require the intended graph change, in addition to full-reference equality."""
    old_ids = {node["id"] for node in before["nodes"]}
    new_ids = {node["id"] for node in after["nodes"]}
    old_files = {node["file"] for node in before["nodes"]}
    new_files = {node["file"] for node in after["nodes"]}
    facts = {"input_manifest_changed": before["analysis"]["inputs"] != after["analysis"]["inputs"],
             "added_symbols": len(new_ids - old_ids), "removed_symbols": len(old_ids - new_ids),
             "edges_changed": before["edges"] != after["edges"], "source_paths_changed": old_files != new_files}
    if not facts["input_manifest_changed"]:
        raise RuntimeError(f"{scenario} did not change the observed input manifest")
    assert_membership_edit(scenario, facts)
    if scenario == "cross_file":
        assert_sibling_call(facts, after)
    return facts



def assert_membership_edit(scenario, facts):
    required = {"addition": facts["added_symbols"] > 0,
                "deletion": facts["removed_symbols"] > 0,
                "rename": facts["source_paths_changed"],
                "signature": min(facts["added_symbols"], facts["removed_symbols"]) > 0}
    if scenario in required and not required[scenario]:
        raise RuntimeError(f"{scenario} did not produce the expected symbol/source identity change")


def assert_sibling_call(facts, after):
    siblings = {node["id"] for node in after["nodes"] if "sibling" in node["label"].lower()}
    if not facts["edges_changed"] or not any(edge["to"] in siblings for edge in after["edges"]):
        raise RuntimeError("Cross-file call did not resolve to the sibling callable")
