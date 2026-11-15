"""Produce the two saved C# artifacts and derive native expectations through the public CLI."""
from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

from csharp_flow_acceptance import require
from csharp_flow_ingestion_acceptance import Client
from csharp_desktop_acceptance import clean_environment
from comparison_desktop_facts import BASE_REF, HEAD_REF, Expectations, comparison, remove_legacy_source

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "fixtures/flow/csharp-comparison"
GRAPH = ".codecallnavigator/graph.json"



@dataclass(frozen=True)
class Prepared:
    project: Path
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




def commit(project, name):
    git(project, "add", "Comparison.csproj", "Changed.cs", "Unchanged.cs", "Legacy.cs")
    git(project, "add", "-f", GRAPH)
    git(project, "commit", "-qm", name)
    git(project, "tag", name)




def binding(result):
    return {"comparison_id": result["comparison_id"],
            "base_commit": result["base"]["commit"], "head_commit": result["head"]["commit"],
            "base_digest": result["base"]["digest"], "head_digest": result["head"]["digest"]}




def compile_revision(client, project, revision):
    (project / "Changed.cs").write_bytes(revision_source(revision))
    inputs = {name: (project / name).read_bytes() for name in
              ("Changed.cs", "Unchanged.cs", "Legacy.cs")}
    client.analyze(project, project / "Comparison.csproj")
    artifact = project / GRAPH
    original = artifact.read_bytes()
    metadata = {"compiler_sha256": hashlib.sha256(original).hexdigest()}
    if revision == "Before":
        artifact.write_text(json.dumps(remove_legacy_source(json.loads(original))))
        metadata["mutation"] = {"path": "/graph/architecture/files", "removed_file": "Legacy.cs",
                                "removed_source_sha256": hashlib.sha256(inputs["Legacy.cs"]).hexdigest(),
                                "removed_rows": 1, "nodes_and_ir_unchanged": True}
    metadata.update(committed_sha256=hashlib.sha256(artifact.read_bytes()).hexdigest(),
                    committed_bytes=artifact.stat().st_size)
    return inputs, metadata


def input_metrics(inputs):
    return {revision: {name: {"sha256": hashlib.sha256(raw).hexdigest(), "bytes": len(raw)}
                       for name, raw in files.items()} for revision, files in inputs.items()}


def prepare(engine, project, timeout):
    project.mkdir()
    for name in ("Comparison.csproj", "Unchanged.cs", "Legacy.cs"):
        shutil.copyfile(FIXTURE / name, project / name)
    git(project, "init", "-q")
    git(project, "config", "user.name", "Native comparison fixture")
    git(project, "config", "user.email", "native-comparison@example.invalid")
    client = Client(engine, clean_environment())
    source_hashes = {}
    inputs, artifacts = {}, {}
    for revision, tag in (("Before", "fixture-before"), ("After", "fixture-after")):
        inputs[revision], artifacts[tag] = compile_revision(client, project, revision)
        source_hashes[revision] = hashlib.sha256(inputs[revision]["Unchanged.cs"]).hexdigest()
        commit(project, tag)
    require(source_hashes["Before"] == source_hashes["After"], "Unchanged physical source was modified")
    git(project, "commit", "--allow-empty", "-qm", "same bytes later endpoint")
    moved = git(project, "rev-parse", "HEAD")
    initial_result = comparison(client, project, {"kind": "catalog", "limit": 100})
    initial = binding(initial_result)
    expected = Expectations(client, project, initial, inputs).cases(initial_result)
    git(project, "update-ref", HEAD_REF, moved, initial["head_commit"])
    reloaded = binding(comparison(client, project, {"kind": "catalog", "limit": 100}))
    git(project, "update-ref", HEAD_REF, initial["head_commit"], moved)
    require(initial["head_digest"] == reloaded["head_digest"]
            and initial["comparison_id"] != reloaded["comparison_id"], "Ref move must rebind identical artifact bytes")
    (project / GRAPH).unlink()
    for name in ("Changed.cs", "Unchanged.cs", "Legacy.cs"):
        (project / name).unlink()
    config = {"version": 2, "project": str(project), "base_ref": BASE_REF, "head_ref": HEAD_REF,
              "expected_viewport": [1050, 680], "timeout_seconds": timeout,
              "initial": initial, "reloaded": reloaded, "cases": expected}
    metrics = {"fixture": str(FIXTURE.relative_to(ROOT)), "commands": client.records,
               "unchanged_source_hashes": source_hashes, "artifacts": artifacts,
               "compiler_inputs": input_metrics(inputs),
               "source_bytes_verified_against_inputs": True, "same_bytes_later_commit_rebinds": True,
               "live_sources_removed": True,
               "working_artifact_removed": True, "legacy_mutation": "one architecture.files row only"}
    return Prepared(project, config, moved, metrics)
