"""Fail early with all discoverable full-verification prerequisite problems."""
import argparse
import json
import importlib.metadata
import re
import os
from pathlib import Path
import platform
import shutil
import sqlite3
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from verification.profile import validate


def checked(check):
    try:
        return check() or []
    except (OSError, ValueError, KeyError, sqlite3.Error, subprocess.SubprocessError) as error:
        return [str(error)]


def require_version(command, matches, hint):
    if not shutil.which(command[0]):
        return []  # Reported by executable preflight.
    result = subprocess.run(command, capture_output=True, text=True, timeout=30, check=True)
    if not matches(result.stdout + result.stderr):
        return [f"Required toolchain: {hint}"]
    return []


def package_version(modules, package, expected):
    value = json.loads((modules / package / "package.json").read_text())["version"]
    if not value.startswith(expected):
        return [f"{package}: expected {expected}x, found {value}"]
    return []


def typescript_problems(environment):
    modules = environment.get("CODECALLNAVIGATOR_TS_NODE_MODULES")
    if not modules:
        return ["Set CODECALLNAVIGATOR_TS_NODE_MODULES to installed TypeScript/React/Next node_modules"]
    modules = Path(modules).resolve()
    errors = []
    for package, expected in [("typescript", "6.0."), ("react", "19."), ("next", "16.")]:
        errors.extend(checked(lambda: package_version(modules, package, expected)))
    typescript = environment.get("CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT")
    if typescript and Path(typescript).resolve() != modules / "typescript":
        errors.append("CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT must match CODECALLNAVIGATOR_TS_NODE_MODULES/typescript")
    return errors


def dependencies(root, environment):
    errors = typescript_problems(environment)
    private = environment.get("CODECALLNAVIGATOR_RECURRING_BILLING")
    if not private or not (Path(private) / "pom.xml").is_file():
        errors.append("Set CODECALLNAVIGATOR_RECURRING_BILLING to the private Maven checkout (private repository access is also required)")
    if not (root / "fixtures/gradle/gradlew").is_file():
        errors.append("Missing fixtures/gradle/gradlew")
    return errors


def maven_supported(output):
    match = re.search(r"Apache Maven (\d+)\.(\d+)", output)
    if match is None:
        return False
    major, minor = map(int, match.groups())
    return major == 4 or (major == 3 and minor >= 9)


def dotnet_supported(output):
    return any(line.startswith("11.") for line in output.splitlines())


def graphviz_supported(output, expected):
    match = re.search(r"graphviz version (\d+\.\d+\.\d+)(?:\s|$)", output)
    return match is not None and match.group(1) == expected


def version_problems(root):
    graphviz = json.loads((root / "analyzers.lock.json").read_text())["graphviz"]["version"]
    specifications = [
        (["node", "--version"], lambda s: int(s.strip().lstrip("v").split(".")[0]) >= 24, "Node 24+"),
        (["dotnet", "--list-sdks"], dotnet_supported, ".NET SDK 11"),
        (["java", "-version"], lambda s: 'version "25' in s, "JDK 25"),
        (["mvn", "--version"], maven_supported, "Maven 3.9+/4"),
        (["dot", "-V"], lambda s: graphviz_supported(s, graphviz), f"Graphviz {graphviz}"),
    ]
    errors = []
    for command, matches, hint in specifications:
        errors.extend(checked(lambda: require_version(command, matches, hint)))
    return errors


def profile_problems(source):
    if source is None:
        return ["Set --profile-source or CODECALLNAVIGATOR_VERIFY_PROFILE_SOURCE to a saved application data directory; benchmarks run on a snapshot, never this source"]
    return checked(lambda: validate(source))


def python_problems(root):
    errors = []
    for name, expected in re.findall(r"^([\w-]+)==([^\s\\]+)", (root / "requirements-verification.txt").read_text(), re.MULTILINE):
        try:
            actual = importlib.metadata.version(name)
        except importlib.metadata.PackageNotFoundError:
            actual = "not installed"
        if actual != expected:
            errors.append(f"Python dependency {name}=={expected}: {actual}; install requirements-verification.txt in the selected interpreter")
    return errors


def problems(root, source, codescene, full=True):
    errors = python_problems(root)
    if not full:
        tools = ["git", "cargo", "rustfmt", "/usr/sbin/lsof", codescene]
        errors.extend(f"Missing executable: {tool}" for tool in tools if not shutil.which(tool))
        return errors
    return errors + full_problems(root, source, codescene)


def full_problems(root, source, codescene):
    errors = []
    if platform.system() != "Darwin" or platform.machine() != "arm64":
        errors.append("Full verification requires Apple Silicon macOS with native Metal/process-table access")
    tools = ["git", "cargo", "rustfmt", "node", "dotnet", "java", "mvn", "dot", "codesign", "otool", "install_name_tool", "/usr/sbin/lsof", codescene]
    errors.extend(f"Missing executable: {tool}" for tool in tools if not shutil.which(tool))
    errors.extend(checked(lambda: version_problems(root)))
    errors.extend(checked(lambda: dependencies(root, os.environ)))
    errors.extend(profile_problems(source))
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--light", action="store_true")
    parser.add_argument("--profile-source", type=Path)
    parser.add_argument("--codescene-command", default="cs-mcp")
    args = parser.parse_args()
    source = args.profile_source.resolve() if args.profile_source else None
    errors = problems(args.root, source, args.codescene_command, not args.light)
    if errors:
        print("Verification prerequisites are incomplete:\n- " + "\n- ".join(errors))
        print("See docs/reference.md#validation. No required acceptance checks were skipped or marked passed.")
        return 1
    print("Required verification tools and inputs are available.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
