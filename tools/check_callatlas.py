#!/usr/bin/env python3
"""Check recorded CallAtlas acceptance output without implementing analysis."""

import argparse
import json
from pathlib import Path


def load_graph(path):
    graph = json.loads(path.read_text())
    assert graph["schema"] == "callatlas.ir.v3", "Unexpected analyzer schema"
    assert graph["nodes"] and graph["edges"], "Empty extraction"
    assert graph["diagnosticsTotal"] == 0, "Compilation diagnostics present"
    assert graph["bodyErrorCount"] == 0, "Method-body compilation errors present"
    assert not graph["diagnosticsTruncated"], "Diagnostics were truncated"
    assert not graph["unresolved"], "Unresolved calls present"
    nodes = {node["id"]: node for node in graph["nodes"]}
    assert len(nodes) == len(graph["nodes"]), "Duplicate symbol identities"
    for edge in graph["edges"]:
        assert edge["from"] in nodes and edge["to"] in nodes, "Dangling relationship"
    return graph, nodes


def edge(graph, source, target, relationship=("call", "direct")):
    kind, tier = relationship
    matches = [item for item in graph["edges"] if item["from"] == source
               and item["to"] == target and item["kind"] == kind
               and item["tier"] == tier]
    assert len(matches) == 1, f"Expected one {kind}/{tier}: {source} -> {target}"
    return matches[0]


def check_fixture(path):
    graph, nodes = load_graph(path)
    source = "[App]::Program::Main()"
    overloads = ["[Library]::Fixture.Worker::Overload(System.Int32)",
                 "[Library]::Fixture.Worker::Overload(System.String)"]
    for target in overloads:
        assert target in nodes
        edge(graph, source, target)
        assert nodes[source]["location"].startswith("App/")
        assert nodes[target]["location"].startswith("Library/")
    edge(graph, source, "[Library]::Fixture.Worker::Identity(T)")
    edge(graph, source, "[Library]::Fixture.Worker::..ctor()", ("new", "direct"))
    recursive = "[Library]::Fixture.Worker::Recurse(System.Int32)"
    edge(graph, recursive, recursive)
    disconnected = "[Library]::Fixture.Worker::Disconnected()"
    assert disconnected in nodes
    assert not any(item["to"] == disconnected for item in graph["edges"])
    edge(graph, disconnected, "[Library]::Fixture.Worker::Helper()")
    edge(graph, source, "[Library]::Fixture.Worker::Helper()", ("methodgroup", "direct"))
    assert not any(item["from"] == source and item["to"] == "[Library]::Fixture.Worker::Helper()"
                   and item["kind"] == "call" for item in graph["edges"]), "Method reference mislabeled as call"
    edge(graph, source, "[System.Runtime]::System.Action#delegate", ("call", "delegate"))
    dispatch = edge(graph, source, "[Library]::Fixture.IWorker::Work()", ("call", "dispatch"))
    targets = dispatch.get("virtualTargets", [])
    expected = {"[Library]::Fixture.Worker::Work()", "[Library]::Fixture.OtherWorker::Work()"}
    if targets:
        assert set(targets) == expected, "Unexpected implementation candidates"
    return {
        "nodes": len(nodes), "edges": len(graph["edges"]),
        "checks": ["overload binding", "cross-project direct calls", "constructor",
                   "generic binding", "recursion", "disconnected method retained",
                   "method reference distinguished", "delegate uncertainty preserved",
                   "interface dispatch distinguished"],
        "dispatchCandidatesReported": targets,
        "limitations": [] if targets else [
            "Cross-project interface dispatch reports its static target but no implementation candidates, although two project implementations exist."
        ],
    }


def check_pacificfate(path):
    graph, nodes = load_graph(path)
    lifecycle = nodes["[PacificFate]::MainGameController::_Ready()"]
    assert lifecycle["location"] == "scripts/main/map/MainGameController.cs:36"
    linked = [node for node in nodes.values() if node["location"].startswith("core/")]
    assert linked, "Linked Core source declarations absent"
    cross = [item for item in graph["edges"]
             if nodes[item["from"]]["location"].startswith("scripts/")
             and nodes[item["to"]]["location"].startswith("core/")
             and item["kind"] == "call" and item["tier"] == "direct"]
    assert cross, "No application -> linked Core direct calls"
    checked = edge(graph,
                   "[PacificFate]::CampaignPersistenceController::CreateDocument(IGameSession,System.String,System.Boolean)",
                   "[PacificFate.Core]::CampaignCheckpointCodec::Encode(CampaignCheckpoint,ScenarioDefinition)")
    return {
        "nodes": len(nodes), "edges": len(graph["edges"]),
        "linkedCoreDeclarations": len(linked), "applicationToCoreDirectCalls": len(cross),
        "lifecycleMethod": lifecycle,
        "checkedCrossProjectCall": {
            **checked, "callerLocation": nodes[checked["from"]]["location"],
            "targetLocation": nodes[checked["to"]]["location"],
        },
        "diagnosticsTotal": graph["diagnosticsTotal"],
        "bodyErrorCount": graph["bodyErrorCount"], "unresolvedCalls": len(graph["unresolved"]),
    }


def check_collisions(path):
    graph, nodes = load_graph(path)
    entry_members = ["Program::Main()", "Program::Main()::Local#local", "Worker::..ctor()",
                     "Worker::Work()", "Worker::Seed()", "Worker::Counter#field", "Worker::Counter#init",
                     "Worker::Value#init", "Worker::get_Value()", "Worker::op_Addition(Worker,Worker)",
                     "Callback#delegate"]
    for assembly in ["EntryA", "EntryB"]:
        prefix = f"[{assembly}]::"
        assert all(prefix + member in nodes for member in entry_members), "An assembly-local declaration was merged"
        main = prefix + "Program::Main()"
        worker = prefix + "Worker::Work()"
        edge(graph, main, worker)
        edge(graph, main, worker, ("methodgroup", "direct"))
        edge(graph, main, prefix + "Callback#delegate", ("call", "delegate"))
        edge(graph, main, prefix + "Program::Main()::Local#local")
        edge(graph, prefix + "Worker::Counter#init", prefix + "Worker::Seed()")
        edge(graph, worker, prefix + "Worker::Counter#field", ("field", "direct"))
        assert nodes[main]["location"].startswith(assembly + "/")
    for assembly in ["TopA", "TopB"]:
        prefix = f"[{assembly}]::"
        root = prefix + "Program::<top-level>"
        assert nodes[root]["kind"] == "toplevel"
        edge(graph, root, prefix + "Worker::Work()")
    for relationship in graph["edges"]:
        assert relationship["from"].split("]::", 1)[0] == relationship["to"].split("]::", 1)[0], "Independent assemblies acquired a false cross-project edge"
    return {"nodes": len(nodes), "edges": len(graph["edges"]),
            "assemblies": ["EntryA", "EntryB", "TopA", "TopB"],
            "identityScheme": "[compiler AssemblyName]::compiler symbol identity",
            "checks": ["same named entry methods stay separate", "same named fields/accessors/initializers stay separate",
                       "delegate/operator/local-function identities stay separate", "both top-level roots retained",
                       "independent assemblies have no false cross-project edges"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-dir", type=Path, default=Path("evidence/analyzers"))
    args = parser.parse_args()
    summary = {
        "status": "checks-passed-with-documented-limitations",
        "fixture": check_fixture(args.evidence_dir / "fixture-callatlas.json"),
        "collisions": check_collisions(args.evidence_dir / "collisions-callatlas.json"),
        "pacificfate": check_pacificfate(args.evidence_dir / "pacificfate-callatlas.json"),
    }
    output = args.evidence_dir / "callatlas-summary.json"
    output.write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
