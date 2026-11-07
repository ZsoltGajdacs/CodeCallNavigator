#!/usr/bin/env python3
"""Validate JDT evidence, retaining failed gate results for review."""

import argparse
import json
from pathlib import Path
import sys


def require(condition, message):
    if not condition:
        raise ValueError(message)


def load_graph(path):
    graph = json.loads(path.read_text())
    require(graph["complete"] is True, "Analyzer marked the extraction incomplete")
    require(graph.get("buildStatus") == 1, "JDT build did not succeed")
    require(not graph.get("failures"), "Hierarchy requests failed")
    require(not graph.get("emptySymbolFiles"), "Discovered source files lack symbols")
    require(graph.get("errorDiagnosticCount") == 0, "Compilation errors remain")
    require(graph.get("serverErrorCount", 0) == 0, "Server errors remain")
    require(graph["declarations"] and graph["edges"], "Extraction is empty")
    nodes = {node["id"]: node for node in graph["declarations"]}
    require(len(nodes) == len(graph["declarations"]), "Duplicate declaration identities")
    for item in graph["edges"]:
        require(item["from"] in nodes and item["to"] in nodes, "Noncanonical relationship endpoint")
        require(item["certainty"] == "analyzer-reported", "Unsupported certainty claim")
        require(nodes[item["to"]]["uri"] == item["target"]["uri"], "Target source identity mismatch")
        require(item["fromRanges"], "Missing analyzer call-site evidence")
    return graph, nodes


def find_method(nodes, file_suffix, name_prefix):
    matches = [node for node in nodes.values()
               if node["kind"] == 6 and node["file"].endswith(file_suffix)
               and node["name"].startswith(name_prefix)]
    require(len(matches) == 1, f"Expected unique method: {file_suffix}::{name_prefix}")
    return matches[0]


def require_edge(graph, source, target):
    matches = [item for item in graph["edges"]
               if item["from"] == source["id"] and item["to"] == target["id"]]
    require(len(matches) == 1, f"Expected unique relationship: {source['name']} -> {target['name']}")
    return {"from": source["name"], "to": target["name"],
            "fromFile": source["file"], "toFile": target["file"],
            "certainty": matches[0]["certainty"], "callSites": len(matches[0]["fromRanges"])}


def check_fixture(path):
    graph, nodes = load_graph(path)
    methods = [node for node in nodes.values() if node["kind"] == 6]
    require(len(methods) == 10, "Fixture must retain all ten authored methods")
    main = find_method(nodes, "fixture/Flow.java", "main(")
    targets = ["overloaded(int)", "overloaded(String)", "identity(T)", "recurse(int)", "helper()"]
    checked = [require_edge(graph, main, find_method(nodes, "fixture/Flow.java", target))
               for target in targets]
    recursive = find_method(nodes, "fixture/Flow.java", "recurse(")
    checked.append(require_edge(graph, recursive, recursive))
    disconnected = find_method(nodes, "fixture/Flow.java", "disconnected()")
    helper = find_method(nodes, "fixture/Flow.java", "helper()")
    checked.append(require_edge(graph, disconnected, helper))
    require(not any(item["to"] == disconnected["id"] for item in graph["edges"]),
            "Disconnected method unexpectedly has a caller")
    first = next((node for node in nodes.values() if node["name"] == "First"), None)
    require(first is not None, "Implicit-constructor class declaration absent")
    checked.append(require_edge(graph, main, first))
    return {"declarations": len(nodes), "authoredMethods": len(methods),
            "edges": len(graph["edges"]), "checkedRelationships": checked,
            "limitations": ["Method reference Flow::helper is reported without a reference/call distinction.",
                            "Implicit constructor target is the class declaration, not a separate callable symbol."]}


def check_storyengine(path):
    graph, nodes = load_graph(path)
    flow_file = "extraction/flow/AdvancedExtractionJobFlow.java"
    execute = find_method(nodes, flow_file, "execute(")
    shared = find_method(nodes, flow_file, "executeSharedStep(")
    targets = ["executeSharedStep(", "executeAdvancedStep(", "logCompletion(", "handleFailure("]
    checked = [require_edge(graph, execute, find_method(nodes, flow_file, target))
               for target in targets]
    step = find_method(nodes, "common/FlowStep.java", "execute(")
    checked.append(require_edge(graph, shared, step))
    files = {node["file"] for node in nodes.values()}
    require(any(name.startswith("build/generated/openapi/") for name in files),
            "Generated OpenAPI declarations absent")
    return {"declarations": len(nodes), "callableMethods": sum(node["kind"] == 6 for node in nodes.values()),
            "filesWithDeclarations": len(files), "sourceRoots": graph["sourceRoots"],
            "edges": len(graph["edges"]), "checkedRelationships": checked,
            "errorDiagnosticCount": graph["errorDiagnosticCount"], "limitations": graph["limitations"]}


def run_check(check, path):
    try:
        return {"passed": True, **check(path)}
    except (ValueError, KeyError, OSError) as error:
        return {"passed": False, "failure": str(error)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-dir", type=Path, default=Path("evidence/analyzers"))
    args = parser.parse_args()
    summary = {"fixture": run_check(check_fixture, args.evidence_dir / "fixture-jdt.json"),
               "storyengine": run_check(check_storyengine, args.evidence_dir / "storyengine-jdt.json")}
    passed = all(item["passed"] for item in summary.values())
    summary["status"] = "checks-passed-with-documented-limitations" if passed else "gate-failed"
    (args.evidence_dir / "jdt-summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))
    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main())
