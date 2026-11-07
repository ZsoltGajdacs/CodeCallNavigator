#!/usr/bin/env python3
"""Check existing java-callgraph2 TSV evidence without resolving Java calls."""

import argparse
from collections import Counter
import csv
import json
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_RUNS = ROOT / "tools/java-bytecode/build/acceptance"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read_rows(path, minimum_columns):
    with path.open(encoding="utf-8", newline="") as stream:
        rows = list(csv.reader(stream, delimiter="\t", quoting=csv.QUOTE_NONE))
    require(all(len(row) >= minimum_columns for row in rows), f"Malformed TSV: {path}")
    return rows


def read_status(path):
    properties = {}
    for line in path.read_text().splitlines():
        if line and not line.startswith("#"):
            key, value = line.split("=", 1)
            properties[key] = value.replace("\\:", ":").replace("\\=", "=")
    require(properties.get("engineSucceeded") == "true", "Engine did not report success")
    require(properties.get("frameSnapshotLimitMethods") == "0", "Frame snapshot limit truncated analysis")
    require(properties.get("engine") == "java-callgraph2", "Unexpected analyzer engine")
    return properties


def load_run(directory):
    status = read_status(directory / "analysis-status.properties")
    output = Path(status["outputDirectory"])
    method_rows = read_rows(output / "method_info.txt", 8)
    methods = {(row[0], row[2] + "[]" * int(row[3])): row for row in method_rows}
    require(len(methods) == len(method_rows), "Duplicate method identity including return type")
    metadata = read_rows(directory / "method_metadata.tsv", 11)
    declared = {(row[5], row[6]) for row in metadata[1:]}
    require(len(declared) == len(metadata) - 1, "Duplicate compiled method identities across inputs")
    require(set(methods) == declared,
            f"Engine declaration coverage differs from compiled metadata: {len(declared - set(methods))} missing, "
            f"{len(set(methods) - declared)} extra")
    calls = []
    for row in read_rows(output / "method_call.txt", 13):
        calls.append(parse_call(row))
    require(methods and calls, "Empty analyzer output")
    return status, methods, calls


def parse_call(row):
    require(row[3].startswith("(") and ")" in row[3], "Missing engine relationship type")
    kind, target = row[3][1:].split(")", 1)
    require(row[1] in {"0", "1"}, "Invalid enabled flag")
    return {"id": int(row[0]), "enabled": row[1] == "1", "kind": kind,
            "from": (row[2], row[5]), "to": (target, row[8]),
            "line": int(row[4]), "callerInput": row[10], "targetInput": row[11]}


def endpoint_coverage(methods, calls):
    missing_callers = {call["from"] for call in calls if call["from"] not in methods}
    missing_targets = {call["to"] for call in calls if call["to"] not in methods}
    return {"reportedCallerPlaceholders": len(missing_callers),
            "reportedTargetPlaceholders": len(missing_targets),
            "targetPlaceholdersWithEngineInheritanceLinks": len(missing_targets & missing_callers),
            "terminalTargetPlaceholders": len(missing_targets - missing_callers),
            "relationshipsWithUndeclaredEndpoints": sum(call["from"] not in methods or call["to"] not in methods
                                                       for call in calls)}


def require_edge(calls, endpoints, kind):
    source, target = endpoints
    matches = [call for call in calls if call["enabled"] and call["from"] == source
               and call["to"] == target and call["kind"] == kind]
    require(bool(matches), f"Missing reported {kind} relationship: {source} -> {target}")
    return {"from": source, "to": target, "engineType": kind,
            "callIds": [call["id"] for call in matches]}


def fixture_checks(methods, calls):
    require(len(methods) == 13, "Fixture must retain all thirteen compiled methods")
    main = ("fixture.Flow:main(java.lang.String[])", "void")
    disconnected = ("fixture.Flow:disconnected()", "void")
    helper = ("fixture.Flow:helper()", "void")
    recursive = ("fixture.Flow:recurse(int)", "void")
    interface = ("fixture.Work:run()", "void")
    expected = [("fixture.Flow:overloaded(int)", "void"),
                ("fixture.Flow:overloaded(java.lang.String)", "void"),
                ("fixture.Flow:identity(java.lang.Object)", "java.lang.Object"), recursive]
    require(all(target in methods for target in expected), "Overload/generic method identities absent")
    checked = [require_edge(calls, (main, target), "STA") for target in expected]
    checked.extend([require_edge(calls, (recursive, recursive), "STA"),
                    require_edge(calls, (disconnected, helper), "STA"),
                    require_edge(calls, (main, helper), "_LM"),
                    require_edge(calls, (main, interface), "INT")])
    for implementation in ["fixture.First:run()", "fixture.Second:run()"]:
        checked.append(require_edge(calls, (interface, (implementation, "void")), "_ITF"))
    check_disconnected_and_reference(methods, calls, (disconnected, main, helper))
    return checked


def check_disconnected_and_reference(methods, calls, endpoints):
    disconnected, main, helper = endpoints
    require(disconnected in methods, "Disconnected declaration absent")
    require(not any(call["to"] == disconnected for call in calls), "Disconnected method gained a caller")
    require(not any(call["from"] == main and call["to"] == helper and call["kind"] == "STA"
                    for call in calls), "Method reference mislabeled as a direct static call")

def storyengine_checks(methods, calls):
    owner = "eu.zsgw.storyengine.extraction.flow.AdvancedExtractionJobFlow:"
    context = "eu.zsgw.storyengine.extraction.flow.AdvancedExtractionJobFlowContext"
    mono = "reactor.core.publisher.Mono"
    execute = (owner + f"execute({context})", mono)
    callback = (owner + f"lambda$execute$0({context})", mono)
    shared = (owner + f"executeSharedStep(java.lang.String,eu.zsgw.storyengine.common.FlowStep,{context})", mono)
    completion = (owner + f"logCompletion({context})", "void")
    require(all(method in methods for method in [execute, callback, shared, completion]),
            "Expected authored or generated lambda methods absent")
    checked = [require_edge(calls, (execute, callback), "_LM"),
               require_edge(calls, (callback, shared), "VIR"),
               require_edge(calls, (execute, completion), "_LM")]
    require(any(key[0].startswith("eu.zsgw.storyengine.generated.") for key in methods),
            "Generated OpenAPI methods absent")
    return checked


def check_run(directory, checks):
    try:
        status, methods, calls = load_run(directory)
        checked = checks(methods, calls)
        project_calls = [call for call in calls if call["callerInput"] != "-" and call["targetInput"] != "-"]
        return {"passed": True, "engine": status["engine"], "engineVersion": status["engineVersion"],
                "bcelVersion": status["bcelVersion"], "methodInfoDeclarations": len(methods),
                "rawRelationships": len(calls), "projectInputRelationships": len(project_calls),
                "relationshipTypes": dict(Counter(call["kind"] for call in project_calls)),
                "endpointCoverage": endpoint_coverage(methods, project_calls),
                "checkedRelationships": checked, "frameSnapshotLimitMethods": 0}
    except (ValueError, KeyError, OSError) as error:
        return {"passed": False, "failure": str(error)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runs-dir", type=Path, default=DEFAULT_RUNS)
    parser.add_argument("--summary", type=Path, default=ROOT / "evidence/analyzers/bytecode-summary.json")
    args = parser.parse_args()
    summary = {"fixture": check_run(args.runs_dir / "fixture-25-full", fixture_checks),
               "fixtureJava21": check_run(args.runs_dir / "fixture-21-full", fixture_checks),
               "storyengine": check_run(args.runs_dir / "storyengine-25-full", storyengine_checks)}
    passed = all(item["passed"] for item in summary.values())
    summary["status"] = "checks-passed-with-documented-limitations" if passed else "gate-failed"
    summary["limitations"] = [
        "_LM denotes an engine-reported lambda/method reference, not proof that the callback runs.",
        "INT/VIR retain static JVM targets; _ITF/_ACT_I/_ACT_C and Spring types retain engine candidate provenance.",
        "Bytecode line numbers lack source columns; compiled synthetic and bridge methods are retained.",
        "Engine-reported endpoints absent from method declarations require body-unavailable placeholders, not name-based resolution.",
        "No adapter inference of DI, reflection, framework execution or cross-language calls.",
    ]
    args.summary.write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))
    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main())
