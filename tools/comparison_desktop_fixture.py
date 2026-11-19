"""Prepare verified compiler snapshots and derive native expectations through the public CLI."""
from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

from csharp_flow_acceptance import require
from csharp_flow_ingestion_acceptance import Client
from csharp_desktop_acceptance import clean_environment
from comparison_desktop_facts import BASE_REF, HEAD_REF, Expectations, comparison
from graph_bundle import stored_bytes

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "fixtures/flow/csharp-comparison"
TARGET = "Comparison.csproj"
SOURCES = ("Changed.cs", "Unchanged.cs", "Legacy.cs")


@dataclass(frozen=True)
class Prepared:
    project: Path
    library: Path
    config: dict
    moved_commit: str
    metrics: dict


def git(project, *arguments):
    result = subprocess.run(["/usr/bin/git", "-C", str(project), *arguments],
                            capture_output=True, text=True, timeout=15)
    require(result.returncode == 0, f"Fixture Git {arguments} failed: {result.stderr}")
    return result.stdout.strip()


def revision_source(revision):
    source = (FIXTURE / f"{revision}.cs.txt").read_text()
    prefix = "// Saved revision café 🧭\r\n" * 220
    return b"\xef\xbb\xbf" + (prefix + source.replace("\n", "\r\n")).encode("utf-8")


def commit_revision(project, revision, tag):
    (project / "Changed.cs").write_bytes(revision_source(revision))
    legacy = (FIXTURE / "Legacy.cs").read_bytes()
    if revision == "Before":
        concrete = b"public int Legacy() => 8;"
        require(legacy.count(concrete) == 1, "Fixture must define one concrete Legacy declaration")
        legacy = legacy.replace(concrete, b"public abstract int Legacy();")
    (project / "Legacy.cs").write_bytes(legacy)
    inputs = {name: (project / name).read_bytes() for name in SOURCES}
    git(project, "add", ".gitattributes", TARGET, *SOURCES)
    git(project, "commit", "-qm", tag)
    git(project, "tag", tag)
    return inputs


def binding(result):
    return {"comparison_id": result["comparison_id"],
            "base_commit": result["base"]["commit"], "head_commit": result["head"]["commit"],
            "base_digest": result["base"]["digest"], "head_digest": result["head"]["digest"]}


def prepare_revision(client, project, revision):
    result = client.command(
        ["prepare-snapshot", "--project", project, "--revision", revision, "--target", TARGET],
        project,
    )
    return json.loads(result.stdout)


def graph_metrics(endpoint, receipt):
    path = Path(endpoint["path"])
    require(receipt["id"] == endpoint["snapshot"]["id"]
            and receipt["digest"] == endpoint["digest"]
            and receipt["identity"]["commit"] == endpoint["commit"]
            and endpoint["commit"] in path.parts,
            "CLI comparison did not read the verified compiler snapshot")
    return {"variant": receipt["id"], "manifest_digest": receipt["digest"],
            "stored_bytes": stored_bytes(path)}


def input_metrics(inputs):
    return {revision: {name: {"sha256": hashlib.sha256(raw).hexdigest(), "bytes": len(raw)}
                       for name, raw in files.items()} for revision, files in inputs.items()}


def prepare(engine, project, timeout):
    project.mkdir()
    library = project.with_name("comparison-library")
    for name in (TARGET, "Unchanged.cs"):
        shutil.copyfile(FIXTURE / name, project / name)
    git(project, "init", "-q")
    git(project, "config", "user.name", "Native comparison fixture")
    git(project, "config", "user.email", "native-comparison@example.invalid")
    # The fixture compares compiler inputs byte for byte, including CRLF.
    (project / ".gitattributes").write_text("*.cs -text\n", encoding="utf-8")
    client = Client(engine, dict(clean_environment(), CODECALLNAVIGATOR_GRAPH_LIBRARY=str(library)))
    inputs = {revision: commit_revision(project, revision, tag) for revision, tag in
              (("Before", "fixture-before"), ("After", "fixture-after"))}
    source_hashes = {revision: hashlib.sha256(files["Unchanged.cs"]).hexdigest()
                     for revision, files in inputs.items()}
    require(source_hashes["Before"] == source_hashes["After"], "Unchanged physical source was modified")
    git(project, "commit", "--allow-empty", "-qm", "same source later endpoint")
    moved = git(project, "rev-parse", "HEAD")

    before = prepare_revision(client, project, BASE_REF)
    after = prepare_revision(client, project, HEAD_REF)
    later = prepare_revision(client, project, moved)
    initial_result = comparison(client, project, {"kind": "catalog", "limit": 100})
    initial = binding(initial_result)
    expected = Expectations(client, project, initial, inputs).cases(initial_result)
    artifacts = {"fixture-before": graph_metrics(initial_result["base"], before),
                 "fixture-after": graph_metrics(initial_result["head"], after)}
    git(project, "update-ref", HEAD_REF, moved, initial["head_commit"])
    reloaded_result = comparison(client, project, {"kind": "catalog", "limit": 100})
    reloaded = binding(reloaded_result)
    artifacts["moved-after"] = graph_metrics(reloaded_result["head"], later)
    git(project, "update-ref", HEAD_REF, initial["head_commit"], moved)
    require(initial["head_digest"] == reloaded["head_digest"]
            and initial["head_commit"] != reloaded["head_commit"]
            and initial["comparison_id"] != reloaded["comparison_id"],
            "Ref move must rebind identical graph bytes to the newly resolved verified commit")
    client.command(["graph-path", "--project", project], project, succeeds=False)
    for name in SOURCES:
        (project / name).unlink()
    config = {"version": 2, "project": str(project), "base_ref": BASE_REF, "head_ref": HEAD_REF,
              "expected_viewport": [1050, 680], "timeout_seconds": timeout,
              "initial": initial, "reloaded": reloaded, "cases": expected}
    metrics = {"fixture": str(FIXTURE.relative_to(ROOT)), "commands": client.records,
               "unchanged_source_hashes": source_hashes, "artifacts": artifacts,
               "compiler_inputs": input_metrics(inputs),
               "source_bytes_verified_against_inputs": True,
               "moved_verified_commit_rebinds": True, "live_sources_removed": True,
               "no_working_graph_created": True, "historical_flow": "abstract declaration"}
    return Prepared(project, library, config, moved, metrics)
