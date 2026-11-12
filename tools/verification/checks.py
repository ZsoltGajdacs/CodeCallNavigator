"""Change selection and deterministic verification stage construction."""
import subprocess
import sys
from pathlib import Path
from verification.runner import Step


def changed_files(root, base):
    def git(*arguments):
        return subprocess.check_output(["git", *arguments], cwd=root).decode().split("\0")
    paths = git("diff", "--name-only", "-z", base, "--")
    paths += git("ls-files", "--others", "--exclude-standard", "-z")
    return sorted({p for p in paths if p and not p.startswith("vendor/")})


def docs_only(paths):
    return all(not p.startswith("fixtures/") and (
        Path(p).suffix == ".md" or p.startswith(".backlog/")
        or Path(p).name in {"LICENSE", "NOTICE", "COPYING"}) for p in paths)


def static_steps(root, paths, base, python):
    steps = [Step("whitespace", ["git", "diff", "--check", base, "--"])]
    for index, path in enumerate(paths):
        if not (root / path).is_file():
            continue
        suffix = Path(path).suffix
        if suffix == ".rs":
            command = ["rustfmt", "--check", "--edition", "2024", "--config", "skip_children=true", path]
        elif suffix == ".py":
            command = [python, "-c", "import ast, pathlib, sys; ast.parse(pathlib.Path(sys.argv[1]).read_bytes(), filename=sys.argv[1])", path]
        elif suffix == ".sh":
            command = ["bash", "-n", path]
        elif suffix in {".js", ".cjs", ".mjs"}:
            command = ["node", "--check", path]
        else:
            continue
        steps.append(Step(f"syntax-format-{index:03d}", command))
    return steps


def runtime_steps(root, python, timeout):
    return [
        Step("build-debug-engine", ["cargo", "build", "--locked", "-p", "codecallnavigator-engine", "--bin", "codecallnavigator-engine"], timeout),
        Step("cargo-check", ["cargo", "check", "--locked", "--workspace", "--all-targets"], timeout),
        Step("clippy", ["cargo", "clippy", "--locked", "--workspace", "--all-targets", "--no-deps", "--", "-D", "warnings"], timeout),
        Step("rust-tests", ["cargo", "test", "--locked", "--workspace", "--all-targets", "--", "--test-threads=1"], timeout),
        Step("python-tests", [python, "-m", "unittest", "discover", "-s", "tests", "-v"], timeout),
        Step("python-tool-tests", [python, "-m", "unittest", "discover", "-s", "tools", "-p", "test_*.py", "-v"], timeout),
    ]


def full_steps(root, args, output):
    python, timeout = sys.executable, args.timeout
    profile_source = args.profile_source or output / "MISSING_PROFILE_SOURCE"
    release_engine = str(root / "target/release/codecallnavigator-engine")
    environment = {"CODECALLNAVIGATOR_ENGINE": release_engine,
                   "CODECALLNAVIGATOR_TEST_ENGINE": release_engine,
                   "CODECALLNAVIGATOR_DATA_DIR": str(output / "profile")}
    return [
        Step("typescript-tests", ["node", "--test", *[str(p.relative_to(root)) for p in sorted((root / "tools/typescript").glob("*.test.js"))]], timeout),
        Step("build-release-engine", ["cargo", "build", "--locked", "--release", "-p", "codecallnavigator-engine", "--bin", "codecallnavigator-engine"], timeout),
        Step("profile-snapshot", [python, "tools/verification/profile.py", str(profile_source), str(output / "profile")]),
        Step("release-acceptance-and-benchmarks", ["cargo", "test", "--locked", "--release", "--workspace", "--all-targets", "--no-fail-fast", "--", "--include-ignored", "--test-threads=1", "--nocapture"], timeout, environment),
        Step("storage-benchmark", ["cargo", "run", "--locked", "--release", "--bin", "benchmark", "--", str(output / "scale-cache")], timeout),
        Step("package", [python, "tools/package.py"], timeout),
        Step("package-signature", ["codesign", "--verify", "--deep", "--strict", "dist/CodeCallNavigator.app"]),
        Step("packaged-engine", [str(root / "dist/CodeCallNavigatorEngine/bin/codecallnavigator-engine"), "--help"]),
    ]
