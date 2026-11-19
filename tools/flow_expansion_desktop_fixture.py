"""Compiler-owned, multi-file source expectations for native expansion acceptance."""
from dataclasses import dataclass
import hashlib
from pathlib import Path
import shutil
import time

from csharp_desktop_acceptance import callable_node, clean_environment
from csharp_flow_acceptance import require
from csharp_flow_ingestion_acceptance import Client
from graph_bundle import graph_path, stored_bytes

FIXTURE = Path(__file__).resolve().parents[1] / "fixtures/flow/expansion-desktop"


@dataclass(frozen=True)
class Prepared:
    project: Path
    entry: str
    source: str
    generation: str
    config: dict
    metrics: dict


def source_snapshot(client, path, symbol, generation):
    result = client.query(path.parent, {"kind": "flow", "symbol": symbol}, generation)["result"]
    content = result["source"]["content"]
    raw = path.read_bytes()
    require(content.encode("utf-8") == raw, f"Cached {path.name} differs from actual compiler input")
    require(not any(step["kind"] == "unsupported" for step in result["chart"]["steps"]),
            f"Native expansion fixture {path.name} unexpectedly has an opaque ordinary chart")
    return {"symbol": symbol, "path": str(path), "content": content,
            "sha256": hashlib.sha256(raw).hexdigest()}


def verify_cached_sources(client, project, sources, generation):
    for source in sources:
        Path(source["path"]).unlink()
    for source in sources:
        result = client.query(project, {"kind": "flow", "symbol": source["symbol"]}, generation)["result"]
        require(result["source"]["content"] == source["content"],
                "Ordinary flow reconstructed source after the live file was removed")


def prepare(engine, project, timeout):
    shutil.copytree(FIXTURE, project)
    client = Client(engine, clean_environment(project))
    started = time.monotonic()
    client.analyze(project, project / "ExpansionDesktop.csproj")
    analyze_seconds = time.monotonic() - started
    snapshot = client.query(project, {"kind": "snapshot"})
    entry = callable_node(snapshot, "Run", "System.Int32")
    helper = callable_node(snapshot, "Step", "System.Int32")
    generation = snapshot["generation"]
    sources = [source_snapshot(client, project / "Root.cs", entry, generation),
               source_snapshot(client, project / "Helpers.cs", helper, generation)]
    artifact = graph_path(project, engine, client.environment)
    digest = hashlib.sha256(artifact.read_bytes()).hexdigest()
    verify_cached_sources(client, project, sources, generation)
    require(hashlib.sha256(artifact.read_bytes()).hexdigest() == digest,
            "Cached-source preflight changed the analyzed artifact")
    config = {"version": 3, "project": str(project), "root_symbol": entry,
              "helper_symbol": helper, "recursive_symbol": helper, "sources": sources,
              "generation": generation, "expected_viewport": [1050, 680],
              "timeout_seconds": timeout}
    metrics = {"fixture": "fixtures/flow/expansion-desktop", "analyze_seconds": analyze_seconds,
               "artifact_sha256": digest, "artifact_bytes": stored_bytes(artifact),
               "compiler_sources": [{k: v for k, v in source.items() if k != "content"}
                                    for source in sources],
               "live_sources_removed": True, "cached_sources_exact": True,
               "commands": client.records}
    return Prepared(project, entry, sources[0]["content"], generation, config, metrics)
