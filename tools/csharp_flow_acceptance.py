#!/usr/bin/env python3
"""Exercise raw C# flow export through the real pinned CallAtlas CLI."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import xml.etree.ElementTree as ET


ROOT = Path(__file__).resolve().parents[1]
BASELINE = ROOT / "evidence/flowcharts/csharp/pre-export-baseline.json"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def objects(value):
    pending = [value]
    while pending:
        current = pending.pop()
        if isinstance(current, dict):
            yield current
        pending.extend(children(current))


def children(value):
    if isinstance(value, dict):
        return value.values()
    return value if isinstance(value, list) else []


def command(arguments, operation):
    result = subprocess.run(arguments, capture_output=True, text=True, timeout=180)
    require(result.returncode == 0,
            f"{operation} failed ({result.returncode}):\n{(result.stderr + result.stdout)[-4000:]}")


def map_project(cli, project, output):
    command(["dotnet", str(cli), "map", str(project), "-o", str(output)], f"CallAtlas map for {project}")
    graph = json.loads(output.read_text())
    require(graph.get("flowFactsVersion") == "2", "CLI has no current flow capability; rebuild Release CLI")
    require(graph.get("flowInventory", {}).get("version") == 1, "Unsupported flow inventory")
    source_metadata(graph)
    return graph


def source_metadata(graph):
    require(graph.get("sourceFactsVersion") == "1", "Unsupported compiler source-facts capability")
    pages = [facts.get("sourceCodePage") for facts in graph["architectureFiles"]]
    require(all(page is None or type(page) is int and page > 0 for page in pages),
            "Compiler source code page must be a nullable positive integer")
    return pages


def historical_projection(graph):
    pages = source_metadata(graph)
    graph.pop("sourceFactsVersion")
    for facts in graph["architectureFiles"]:
        facts.pop("sourceCodePage", None)
    return pages


def rebase_expected_string(value, recorded_root, owned_root):
    if not isinstance(value, str):
        return value
    if value == recorded_root or value.startswith(recorded_root + "/"):
        return owned_root + value[len(recorded_root):]
    return value


def rebase_expected(value, recorded_root, owned_root):
    """Relocate archived physical identities only; never normalize compiler output."""
    if isinstance(value, dict):
        return {key: rebase_expected(item, recorded_root, owned_root) for key, item in value.items()}
    if isinstance(value, list):
        return [rebase_expected(item, recorded_root, owned_root) for item in value]
    return rebase_expected_string(value, str(recorded_root), str(owned_root))


def recorded_baseline_source(fixture):
    recorded = Path(fixture["graph"]["architectureFiles"][0]["file"])
    require(recorded.name == fixture["source_file"] and recorded.parent.name == "task22-flow-baseline-source",
            "Baseline source is not the expected task-owned fixture")
    return recorded


def materialize_baseline_source(fixture, work):
    recorded = recorded_baseline_source(fixture)
    root = work.resolve() / recorded.parent.name
    source = root / recorded.name
    require(not root.is_symlink() and not source.is_symlink(), "Refuse symlinked baseline source")
    root.mkdir(exist_ok=True)
    require(set(root.iterdir()) <= {source}, "Refuse baseline directory containing unrelated files")
    expected_bytes = fixture["source"].encode("utf-8")
    if source.exists():
        require(source.read_bytes() == expected_bytes, "Refuse modified baseline source")
    else:
        source.write_bytes(expected_bytes)
    return source


def baseline_parity(cli, baseline, work):
    fixture = baseline["fixture"]
    source = materialize_baseline_source(fixture, work)
    root = source.parent
    expected = rebase_expected(fixture["graph"], recorded_baseline_source(fixture).parent, root)
    graph = map_project(cli, root, work / "baseline.json")
    inventory = graph.pop("flowInventory")
    graph.pop("flowFactsVersion")
    graph.pop("root")
    source_pages = historical_projection(graph)
    changed = sorted(key for key in graph.keys() | expected.keys()
                     if graph.get(key) != expected.get(key))
    require(not changed, f"Existing graph projection changed: {changed}")
    require(source.read_bytes() == fixture["source"].encode("utf-8"), "Mapping changed baseline source")
    repeated = map_project(cli, root, work / "baseline-repeated.json")
    require(repeated["flowInventory"] == inventory, "Flow export is not deterministic")
    return {"nodes": len(graph["nodes"]), "edges": len(graph["edges"]),
            "functions": len(inventory["functions"]), "existing_graph_equal": True,
            "flow_inventory_repeat_equal": True, "source_facts_version": "1",
            "source_code_pages": source_pages}


def collision_ownership(cli, work):
    fixture = work / "collisions"
    shutil.copytree(ROOT / "fixtures/dotnet-collisions", fixture,
                    ignore=shutil.ignore_patterns("bin", "obj"))
    command(["dotnet", "restore", str(fixture / "Collisions.sln"), "--nologo"],
            "Restore disposable collision fixture")
    graph = map_project(cli, fixture / "Collisions.sln", work / "collisions.json")
    require(set(source_metadata(graph)) == {65001}, "UTF-8 solution sources lost compiler encoding metadata")
    functions = graph["flowInventory"]["functions"]
    node_ids = {node["id"] for node in graph["nodes"]}
    work_functions = [f for f in functions if f["symbol"].endswith("::Worker::Work()")]
    require(len(work_functions) == 4, "Expected independent Worker.Work flows in all four assemblies")
    expected = {"EntryA", "EntryB", "TopA", "TopB"}
    require({f["project"] for f in work_functions} == expected, "Assembly ownership was merged")
    targets_checked = sum(validate_function(function, node_ids, expected) for function in functions)
    require(targets_checked >= 6, "Collision fixture did not exercise caller-to-target attribution")
    return {"assemblies": sorted(expected), "functions": len(functions), "fixture_restored": True,
            "same_named_target_sites_checked": targets_checked}


def validate_function(function, node_ids, expected):
    assembly = function["project"]
    require(assembly in expected, f"Unexpected callable owner: {assembly}")
    require(function["symbol"].startswith(f"[{assembly}]::"), "Callable identity lost assembly owner")
    require(Path(function["file"]).parent.name == assembly, "Callable source has another project owner")
    require(function["symbol"] in node_ids, "Flow callable is absent from graph")
    targets = (target for item in objects(function["body"]) for target in item.get("targets", []))
    return sum(validate_target(target, assembly, node_ids) for target in targets)


def validate_target(target, assembly, node_ids):
    require(target["symbol"] in node_ids, "Flow target is absent from graph")
    if "::Worker::Work()" not in target["symbol"]:
        return 0
    require(target["symbol"].startswith(f"[{assembly}]::"),
            "Flow target crossed same-named assembly contexts")
    return 1


def source_and_limits(cli, work):
    root = work / "source-probes"
    root.mkdir()
    source = ('class Probe {\r\n static string Leaf(string s) => s.Trim();\r\n'
              ' static string Run() { string s = "é😀"; return Leaf(s); }\r\n'
              ' static object Conditional(Probe p) => p?.ToString();\r\n'
              ' static void Deep() {' + 'if(true){' * 40 + 'Leaf("x");' + '}' * 40 + '}\r\n'
              ' static void Wide() {' + 'Leaf("x");' * 1600 + '}\r\n}\r\n')
    raw = b"\xef\xbb\xbf" + source.encode("utf-8")
    path = root / "Probe.cs"
    path.write_bytes(raw)
    graph = map_project(cli, root, work / "source-probes.json")
    functions = {f["symbol"].split("::")[-1]: f for f in graph["flowInventory"]["functions"]}
    run = functions["Run()"]
    calls = [item for item in objects(run["body"]) if "targets" in item]
    require(len(calls) == 1, "Run fixture should have one source call")
    span = calls[0]["span"]
    require(raw[span["start"]:span["end"]].decode("utf-8") == "Leaf(s)",
            "UTF-8 BOM/Unicode/CRLF call span does not select original source")
    for name in ["Conditional(Probe)", "Deep()", "Wide()"]:
        function = functions[name]
        require(function["diagnostics"], f"{name} has no unsupported/limit explanation")
        require(any(item.get("kind") == "unsupported" for item in objects(function["body"])),
                f"{name} has no opaque boundary")
        require(not any(item.get("targets") for item in objects(function["body"])),
                f"{name} exposes nested calls as eager supported flow")
    require(len(functions["Deep()"]["body"]) == len(functions["Wide()"]["body"]) == 1,
            "Budget exhaustion must produce a bounded callable-level opaque record")
    require(path.read_bytes() == raw, "Mapping changed the analyzed source")
    return {"exact_bom_unicode_crlf_span": True, "opaque_cases": 3,
            "wide_body_records": len(functions["Wide()"]["body"])}


def nameof_semantics(cli, work):
    root = work / "nameof-probes"
    root.mkdir()
    # Keep the intrinsic in another class: a real method named nameof can shadow it.
    source = ("class Intrinsic { Intrinsic Child=>this; int Value=>1; "
              "string Run()=>nameof(Child.Value); }\n"
              "class Named { static int Hit()=>1; static int @nameof(int value)=>value; "
              "int Run()=>@nameof(Hit()); }\n")
    (root / "Nameof.cs").write_text(source)
    graph = map_project(cli, root, work / "nameof-probes.json")
    validate_nameof_flows(graph)
    return {"intrinsic_atomic_non_eager": True, "argument_before_user_call": True,
            "missing_native_binding_diagnostic": True, "native_graph_binding_unchanged": True}


def validate_nameof_flows(graph):
    functions = graph["flowInventory"]["functions"]
    intrinsic = next(f for f in functions if f["symbol"].endswith("::Intrinsic::Run()"))
    validate_intrinsic_nameof(intrinsic["body"][0]["value"])
    named = next(f for f in functions if f["symbol"].endswith("::Named::Run()"))
    validate_user_nameof(named["body"][0]["value"])
    require(not any(edge["from"] == named["symbol"] and "::Named::nameof(" in edge["to"]
                    for edge in graph["edges"]),
            "Flow export must not silently add a missing native graph relationship")


def validate_intrinsic_nameof(value):
    require(value["kind"] == "value" and value["text"] == "nameof(Child.Value)",
            "Intrinsic nameof must be one atomic source value")
    require(value["calls"] == [], "Intrinsic nameof must not evaluate property operands")


def validate_user_nameof(sequence):
    require(sequence["kind"] == "sequence" and len(sequence["items"]) == 2,
            "User nameof method must evaluate its argument before the invocation")
    argument, invocation = (item["calls"][0] for item in sequence["items"])
    require(argument["label"] == "Hit()"
            and argument["targets"][0]["symbol"].endswith("::Named::Hit()"),
            "User nameof method lost its compiler-bound argument invocation")
    require(invocation["label"] == "@nameof(Hit())" and invocation["targets"] == []
            and "Native analyzer omitted" in invocation["diagnostic"],
            "Omitted native nameof binding must remain an explicit flow diagnostic")


def upstream_failures(trx, baseline):
    root = ET.parse(trx).getroot()
    failures = sorted(result.attrib["testName"] for result in root.findall(".//{*}UnitTestResult")
                      if result.attrib.get("outcome") == "Failed")
    require(failures == baseline["failed_tests"],
            f"Upstream failure set changed; new={sorted(set(failures) - set(baseline['failed_tests']))}, "
            f"missing={sorted(set(baseline['failed_tests']) - set(failures))}")
    counters = root.find(".//{*}Counters")
    require(counters is not None, "TRX omits counters")
    require(int(counters.attrib["total"]) >= int(baseline["counters"]["total"]),
            "TRX did not execute the complete upstream baseline")
    require(int(counters.attrib["passed"]) >= int(baseline["counters"]["passed"]),
            "TRX lost passing upstream tests")
    return {"counters": counters.attrib, "baseline_failure_names_equal": True,
            "failed_tests": failures}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "vendor/callatlas/src/CallAtlas.Cli/bin/Release/net11.0/callatlas.dll")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--trx", type=Path)
    args = parser.parse_args()
    baseline = json.loads(BASELINE.read_text())
    with tempfile.TemporaryDirectory(prefix="task22-raw-flow-acceptance-") as directory:
        work = Path(directory)
        result = {"status": "passed", "baseline": baseline_parity(args.cli.resolve(), baseline, work),
                  "collisions": collision_ownership(args.cli.resolve(), work),
                  "source_and_limits": source_and_limits(args.cli.resolve(), work),
                  "nameof": nameof_semantics(args.cli.resolve(), work)}
        if args.trx:
            result["upstream_tests"] = upstream_failures(args.trx, baseline)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"Raw C# flow CLI acceptance passed; report: {args.output}")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.SubprocessError, ET.ParseError) as error:
        raise SystemExit(f"Raw C# flow acceptance failed: {error}") from error
