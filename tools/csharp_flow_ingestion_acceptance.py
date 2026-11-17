#!/usr/bin/env python3
"""Verify C# flow ingestion through production engine analysis and query processes."""
from graph_bundle import load as load_bundle, publish_fixture, capture, restore
import argparse
from concurrent.futures import FIRST_COMPLETED, ThreadPoolExecutor, wait
from functools import partial
from threading import Event
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from csharp_flow_acceptance import objects, require, source_metadata, rebase_expected


ROOT = Path(__file__).resolve().parents[1]
BASELINE = ROOT / "fixtures/flow/csharp-baselines/pre-ingestion.json"
GRAPH = Path(".codecallnavigator/graph.json")
# Match the four-boundary compiler budget inside this one acceptance gate;
# outer native/performance gate exclusivity and timeouts remain unchanged.
CASE_WORKERS = 4
PROJECT = '<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net11.0</TargetFramework></PropertyGroup></Project>\n'
LINKED_SOURCE = ('namespace Linked; public static class Shared { public static int Leaf(int n)=>n; '
                 'public static int Leaf(string s)=>s.Length; public static int Run(){\n'
                 '#if FIRST\nreturn Leaf(1);\n#else\nreturn Leaf("s");\n#endif\n}}\n')


@dataclass(frozen=True)
class SourceFixture:
    root: Path
    project: Path
    source: Path
    raw: bytes


@dataclass(frozen=True)
class AnalyzedFixture:
    fixture: SourceFixture
    artifact: dict
    snapshot: dict


@dataclass(frozen=True)
class CompileContext:
    name: str
    constants: str
    code_page: int


class Client:
    def __init__(self, engine, environment=None, records=None):
        self.engine = engine.resolve()
        self.environment = dict(os.environ if environment is None else environment)
        for key in ["CODECALLNAVIGATOR_RESOURCES", "CODECALLNAVIGATOR_ENGINE", "CODECALLNAVIGATOR_TEST_ENGINE"]:
            self.environment.pop(key, None)
        self.records = [] if records is None else records

    def in_environment(self, environment):
        return Client(self.engine, environment, self.records)

    def command(self, arguments, root, value=None, succeeds=True):
        result = subprocess.run([str(self.engine), *map(str, arguments)], cwd=root,
                                env=self.environment,
                                input=None if value is None else json.dumps(value),
                                capture_output=True, text=True, timeout=180)
        record = {"arguments": list(map(str, arguments)), "exit_code": result.returncode,
                  "dotnet_on_path": shutil.which("dotnet", path=self.environment.get("PATH")) is not None}
        if value is not None:
            record["stdin"] = value
        self.records.append(record)
        require((result.returncode == 0) == succeeds,
                f"Engine command {arguments} returned {result.returncode}:\n{(result.stderr + result.stdout)[-4000:]}")
        return result

    def query(self, root, operation, generation=None, succeeds=True):
        request = {"version": 2, "project": str(root), "query": operation}
        if generation is not None:
            request["expected_generation"] = generation
        result = self.command(["query"], root, request, succeeds)
        reply = json.loads(result.stdout)
        if succeeds:
            require(reply["error"] is None, f"Unexpected query error: {reply['error']}")
        else:
            require(result.returncode == 2 and reply["result"] is None, "Expected structured query failure")
        return reply

    def analyze(self, root, target, succeeds=True):
        return self.command(["analyze", "--project", root, target], root,
                            succeeds=succeeds)


def prepare_fixture(fixture, work):
    recorded = Path(fixture["root"])
    require(recorded.parent == Path("/private/tmp/task22-ingestion-baseline"), "Unexpected baseline fixture root")
    root = work.resolve() / recorded.name
    require(not root.is_symlink(), "Refuse symlinked baseline fixture")
    root.mkdir(parents=True, exist_ok=True)
    source = root / fixture["source_file"]
    project = root / fixture["project_file"]
    require(source.parent == root and project.parent == root,
            "Baseline filenames must stay inside the owned fixture")
    raw = fixture["source"].encode(fixture["source_encoding"])
    write_fixture_file(source, raw)
    write_fixture_file(project, fixture["project"].encode("utf-8"))
    return SourceFixture(root, project, source, raw)


def write_fixture_file(path, raw):
    require(not path.is_symlink(), f"Refuse symlinked fixture file: {path}")
    if path.exists():
        require(path.read_bytes() == raw, f"Refuse modified fixture file: {path}")
    else:
        path.write_bytes(raw)


def projection(graph):
    return {key: graph.get(key) for key in ["nodes", "edges", "entry_points"]}


def graph_parity(client, fixture, work):
    paths = prepare_fixture(fixture, work)
    native = json.loads(client.command(["analyze-native", paths.project], paths.root).stdout)
    expected_native = rebase_expected(fixture["native_projection"], Path(fixture["root"]), paths.root)
    require(projection(native) == expected_native, f"Native graph changed for {fixture['name']}")
    client.analyze(paths.root, paths.project)
    artifact = load_bundle(paths.root / GRAPH)
    require(projection(artifact["graph"]) == fixture["canonical_projection"],
            f"Canonical graph changed for {fixture['name']}")
    require(paths.source.read_bytes() == paths.raw, "Analysis changed fixture source")
    snapshot = client.query(paths.root, {"kind": "snapshot"})
    return AnalyzedFixture(paths, artifact, snapshot)


def run_node(snapshot):
    matches = [node for node in snapshot["result"]["nodes"] if node["label"] == "Run"]
    require(len(matches) == 1, "Expected one Run callable")
    return matches[0]


def validate_flow(reply, node, expected_source, nodes):
    result = reply["result"]
    validate_flow_owner(result, node, expected_source)
    return validate_flow_calls(result["function"]["body"], nodes, expected_source.encode("utf-8"))


def validate_flow_owner(result, node, expected_source):
    require(result["source"]["content"] == expected_source, "Flow lost exact analyzed source bytes")
    function = result["function"]
    require(function["symbol"] == node["id"], "Flow callable identity differs from graph")
    require(function["build"] == node["build"] and function["project"] == node["project"],
            "Flow callable ownership differs from graph")
    require(function["file"] == node["file"] == result["source"]["file"], "Flow source has another owner")


def validate_flow_calls(body, nodes, raw):
    calls = [item for item in objects(body) if item.get("targets")]
    validate_flow_targets(calls, nodes)
    for call in calls:
        span = call["span"]
        require(raw[span["start"]:span["end"]].decode("utf-8") == call["label"],
                "Flow call span does not select exact original source")
    return calls


def validate_flow_targets(calls, nodes):
    targets = [target for call in calls for target in call["targets"]]
    require({target["symbol"] for target in targets} <= {item["id"] for item in nodes},
            "Flow references absent graph targets")
    require(all(target["certainty"] and target["provenance"] for target in targets),
            "Flow target lost certainty or provenance")


def no_dotnet_environment(client):
    environment = {**client.environment, "PATH": "/usr/bin:/bin"}
    require(shutil.which("dotnet", path=environment["PATH"]) is None, "Restricted query PATH still contains dotnet")
    return environment


def cached_after_source_changes(client, fixture, reply):
    saved = (fixture.root / GRAPH).read_bytes()
    operation = {"kind": "flow", "symbol": reply["result"]["function"]["symbol"]}
    source_operation = {"kind": "source", "symbol": operation["symbol"]}
    try:
        fixture.source.write_text("this is deliberately invalid C# source")
        edited = client.query(fixture.root, operation, reply["generation"])
        edited_source = client.query(fixture.root, source_operation, reply["generation"])
        fixture.source.unlink()
        removed = client.query(fixture.root, operation, reply["generation"])
        removed_source = client.query(fixture.root, source_operation, reply["generation"])
        require(edited["result"] == removed["result"] == reply["result"],
                "Flow query mixed live source with analyzed flow")
        require(edited_source["result"]["content"] == removed_source["result"]["content"]
                == reply["result"]["source"]["content"], "Analyzed source query read live files")
        require(edited["freshness"]["input_status"] == removed["freshness"]["input_status"] == "stale",
                "Source mutation/removal did not report stale graph inputs")
        require((fixture.root / GRAPH).read_bytes() == saved, "Query modified canonical graph")
    finally:
        fixture.source.write_bytes(fixture.raw)
    return {"edited_source_ignored": True, "removed_source_ignored": True,
            "query_without_dotnet": True, "canonical_bytes_unchanged": True,
            "analyzed_source_query_after_edit_delete": True, "staleness_reported": True}


def historical_guidance(client, root, fixture, symbol):
    path = root / GRAPH
    current = capture(path)
    try:
        artifact = {**fixture["artifact"], "schema": "codecallnavigator.graph.v2"}
        publish_fixture(path, artifact)
        client.query(root, {"kind": "status"})
        unavailable = client.query(root, {"kind": "flow", "symbol": symbol}, succeeds=False)
        require(unavailable["error"]["code"] == "flow_unavailable", "Absent inventory must preserve structural queries")
        require("refresh" in unavailable["error"]["message"].lower(), "Absent inventory lacks refresh guidance")
    finally:
        restore(path, current)
    return {"structure_without_inventory_readable": True, "missing_inventory_refresh_guidance": True}


def failed_source_refresh(client, fixture):
    saved = (fixture.root / GRAPH).read_bytes()
    try:
        fixture.source.write_text("namespace Fixture; class Broken { return invalid; }")
        client.analyze(fixture.root, fixture.project, succeeds=False)
        require((fixture.root / GRAPH).read_bytes() == saved, "Failed source refresh replaced prior generation")
    finally:
        fixture.source.write_bytes(fixture.raw)
    return {"invalid_source_refresh_retains_prior_generation": True}


def fault_environment(client, work):
    real_dotnet = shutil.which("dotnet", path=client.environment.get("PATH"))
    require(real_dotnet is not None, "dotnet required for analysis fault injection")
    interpreter = shutil.which("python3.13")
    require(interpreter is not None, "Python 3.13 required for analyzer-output fault injection")
    folder = work / "fault-bin"
    folder.mkdir()
    wrapper = folder / "dotnet"
    wrapper.write_text(f'''#!{interpreter}
import json,os,subprocess,sys
from pathlib import Path
arguments=sys.argv[1:]
result=subprocess.run([{real_dotnet!r},*arguments])
if result.returncode==0 and len(arguments)>1 and arguments[1]=="map" and "-o" in arguments:
    output=Path(arguments[arguments.index("-o")+1])
    value=json.loads(output.read_text())
    assert value["flowFactsVersion"]=="2" and value["flowInventory"]["functions"]
    original=output.read_bytes()
    if os.environ.get("TASK22_FLOW_RAW_CAPTURE"):
        Path(os.environ["TASK22_FLOW_RAW_CAPTURE"]).write_bytes(original)
    if os.environ["TASK22_FLOW_FAULT"]=="missing":
        value.pop("flowInventory",None)
    elif os.environ["TASK22_FLOW_FAULT"]=="corrupt":
        value["flowInventory"]["functions"][0]["symbol"]="missing-owned-flow-symbol"
    else:
        assert os.environ["TASK22_FLOW_FAULT"]=="passthrough"
    if os.environ["TASK22_FLOW_FAULT"]!="passthrough":
        output.write_text(json.dumps(value))
    else:
        assert output.read_bytes()==original
    Path(os.environ["TASK22_FLOW_FAULT_MARKER"]).write_text(json.dumps({{"fault":os.environ["TASK22_FLOW_FAULT"],"raw_map_succeeded":True}}))
sys.exit(result.returncode)
''')
    wrapper.chmod(0o755)
    return {**client.environment, "PATH": str(folder) + os.pathsep + client.environment.get("PATH", "")}


def malformed_refreshes(client, root, project, work):
    environment = fault_environment(client, work)
    control = client.in_environment({**environment, "TASK22_FLOW_FAULT": "passthrough",
                                     "TASK22_FLOW_FAULT_MARKER": str(work / "fault-control.json")})
    control.analyze(root, project)
    saved = (root / GRAPH).read_bytes()
    for fault in ["missing", "corrupt"]:
        marker = work / f"fault-observed-{fault}.json"
        failing = client.in_environment({**environment, "TASK22_FLOW_FAULT": fault,
                                         "TASK22_FLOW_FAULT_MARKER": str(marker)})
        failure = failing.analyze(root, project, succeeds=False)
        require(marker.exists(), f"{fault} test did not reach successful analyzer-output injection")
        require(json.loads(marker.read_text()) == {"fault": fault, "raw_map_succeeded": True},
                "Analyzer fault marker did not confirm the intended case")
        expected = {"missing": "CallAtlas function flow facts are missing or unsupported",
                    "corrupt": "C# flow callable is absent from compiler graph: missing-owned-flow-symbol"}
        require(expected[fault] in failure.stderr, f"{fault} output was rejected for an unrelated reason")
        require((root / GRAPH).read_bytes() == saved, f"{fault} flow facts replaced prior generation")
    return {"missing_flow_capability_retains_generation": True,
            "corrupt_owned_flow_identity_retains_generation": True,
            "passthrough_wrapper_analysis_succeeds": True, "rejection_reasons_matched": True}


def utf8_acceptance(client, fixture, work):
    state = graph_parity(client, fixture, work)
    paths, snapshot = state.fixture, state.snapshot
    node = run_node(snapshot)
    restricted = client.in_environment(no_dotnet_environment(client))
    flow = restricted.query(paths.root, {"kind": "flow", "symbol": node["id"]}, snapshot["generation"])
    calls = validate_flow(flow, node, paths.raw.decode("utf-8"), snapshot["result"]["nodes"])
    targets = validate_overloads(calls, snapshot["result"]["nodes"])
    require(state.artifact["graph"]["flow_inventory"]["functions"], "Canonical artifact omitted C# flow inventory")
    return {"native_graph_equal": True, "canonical_graph_equal": True, "overload_targets": targets,
            "exact_bom_unicode_crlf_source": True,
            "cached_source": cached_after_source_changes(restricted, paths, flow),
            "historical": historical_guidance(client, paths.root, fixture, node["id"]),
            "invalid_refresh": failed_source_refresh(client, paths),
            "malformed_refreshes": malformed_refreshes(client, paths.root, paths.project, work)}


def validate_overloads(calls, nodes):
    expected = {"Leaf(label)": "Leaf(System.String)", "Leaf(value)": "Leaf(System.Int32)"}
    require({call["label"] for call in calls} == set(expected), "Overload call sites were merged or omitted")
    targets = {node["id"]: node for node in nodes}
    for call in calls:
        require(len(call["targets"]) == 1, "Exact overload call acquired ambiguous targets")
        target = targets[call["targets"][0]["symbol"]]
        require(target["signature"].endswith(expected[call["label"]]), "Call site resolved to the wrong overload")
    return len(expected)


def graph_only_encoding(client, fixture, work):
    state = graph_parity(client, fixture, work)
    root, snapshot = state.fixture.root, state.snapshot
    node = run_node(snapshot)
    source = client.query(root, {"kind": "source", "symbol": node["id"]}, snapshot["generation"])
    require(source["result"]["content"] == fixture["source"],
            f"{fixture['name']} compiler-decoded source became unavailable")
    flow = client.query(root, {"kind": "flow", "symbol": node["id"]}, snapshot["generation"], succeeds=False)
    require(flow["error"]["code"] == "flow_unavailable",
            f"{fixture['name']} must not acquire unsafe UTF-8 flow spans")
    require(any("not UTF-8" in reason for reason in snapshot["result"]["diagnostics"]),
            f"{fixture['name']} graph-only behavior lacks an encoding explanation")
    return {"native_graph_equal": True, "canonical_graph_equal": True,
            "decoded_source_available": True, "flow_unavailable_with_encoding_reason": True}


def linked_contexts(client, work):
    contexts = [CompileContext("A", "FIRST", 65001), CompileContext("B", "SECOND", 65001)]
    fixture = linked_fixture(work, "linked", contexts)
    client.analyze(fixture.root, fixture.project)
    snapshot = client.query(fixture.root, {"kind": "snapshot"})
    runs = [node for node in snapshot["result"]["nodes"] if node["label"] == "Run"]
    require({node["project"] for node in runs} == {"A", "B"}, "Linked source compile contexts were merged")
    for node in runs:
        validate_linked_flow(client, snapshot, node, LINKED_SOURCE)
    return {"distinct_contexts": 2, "shared_source": True, "conditional_targets_preserved": True}


def linked_fixture(work, name, contexts):
    root = work / name
    root.mkdir()
    source = root / "Shared.cs"
    source.write_text(LINKED_SOURCE)
    target = linked_projects(root, contexts)
    return SourceFixture(root, target, source, LINKED_SOURCE.encode("utf-8"))


def linked_projects(root, contexts):
    for context in contexts:
        folder = root / context.name
        folder.mkdir()
        properties = f"<DefineConstants>{context.constants}</DefineConstants><CodePage>{context.code_page}</CodePage>"
        project = PROJECT.replace("</PropertyGroup>", properties + "</PropertyGroup>")
        project = project.replace("</Project>", '<ItemGroup><Compile Include="../Shared.cs" Link="Shared.cs" /></ItemGroup></Project>')
        (folder / f"{context.name}.csproj").write_text(project)
    target = root / "Linked.slnx"
    projects = ''.join(f'<Project Path="{c.name}/{c.name}.csproj" />' for c in contexts)
    target.write_text(f'<Solution>{projects}</Solution>')
    return target


def validate_linked_flow(client, snapshot, node, source):
    root = Path(node["file"]).parent
    flow = client.query(root, {"kind": "flow", "symbol": node["id"]}, snapshot["generation"])
    calls = validate_flow(flow, node, source, snapshot["result"]["nodes"])
    require(len(calls) == 1, "Linked conditional body has incorrect call membership")
    target_node = next(n for n in snapshot["result"]["nodes"] if n["id"] == calls[0]["targets"][0]["symbol"])
    require(target_node["project"] == node["project"], "Linked flow target crossed compiler contexts")
    parameter = "System.Int32" if node["project"] == "A" else "System.String"
    require(target_node["signature"].endswith(f"Leaf({parameter})"), "Linked flow used inactive conditional source")


def validate_linked_graph_only(client, snapshot, node, source):
    root = Path(node["file"]).parent
    cached = client.query(root, {"kind": "source", "symbol": node["id"]}, snapshot["generation"])
    require(cached["result"]["content"] == source, "Linked graph-only context lost compiler-decoded source")
    unavailable = client.query(root, {"kind": "flow", "symbol": node["id"]}, snapshot["generation"], succeeds=False)
    require(unavailable["error"]["code"] == "flow_unavailable", "Latin1 linked context acquired unsafe flow")
    require(any(node["signature"] in reason and "not UTF-8" in reason for reason in snapshot["result"]["diagnostics"]),
            "Linked encoding diagnostic lost compiler-context ownership")


def capture_linked_analysis(client, fixture, work):
    capture = work / "linked-capture"
    capture.mkdir()
    raw = capture / "map.json"
    marker = capture / "completed.json"
    environment = {**fault_environment(client, capture), "TASK22_FLOW_FAULT": "passthrough",
                   "TASK22_FLOW_FAULT_MARKER": str(marker), "TASK22_FLOW_RAW_CAPTURE": str(raw)}
    client.in_environment(environment).analyze(fixture.root, fixture.project)
    require(marker.exists() and raw.exists(), "Linked raw facts were not captured from successful analysis")
    require(json.loads(marker.read_text()) == {"fault": "passthrough", "raw_map_succeeded": True},
            "Linked analysis did not complete unchanged passthrough capture")
    return raw.read_bytes()


def actual_linked_pages(raw, fixture, contexts):
    value = json.loads(raw)
    source_metadata(value)
    rows = [facts for facts in value["architectureFiles"]
            if Path(facts["file"]).resolve() == fixture.source.resolve()]
    require(len(rows) == len(contexts), "Linked source facts have missing or duplicate compile contexts")
    pages = {facts["assembly"]: facts.get("sourceCodePage") for facts in rows}
    require(set(pages) == {c.name for c in contexts}, "Linked encoding facts lost assembly ownership")
    require(all(type(page) is int for page in pages.values()), "File-backed compiler context has unknown encoding")
    return pages


def validate_actual_linked_flows(client, snapshot, pages):
    runs = [node for node in snapshot["result"]["nodes"] if node["label"] == "Run"]
    require({node["project"] for node in runs} == set(pages), "Linked flow membership lost compile contexts")
    for node in runs:
        if pages[node["project"]] == 65001:
            validate_linked_flow(client, snapshot, node, LINKED_SOURCE)
        else:
            validate_linked_graph_only(client, snapshot, node, LINKED_SOURCE)


def mixed_encoding_contexts(client, work):
    contexts = [CompileContext("A", "FIRST", 65001), CompileContext("B", "SECOND", 65001),
                CompileContext("C", "THIRD", 28591)]
    fixture = linked_fixture(work, "mixed-linked", contexts)
    raw = capture_linked_analysis(client, fixture, work)
    pages = actual_linked_pages(raw, fixture, contexts)
    snapshot = client.query(fixture.root, {"kind": "snapshot"})
    validate_actual_linked_flows(client, snapshot, pages)
    utf8 = sorted(name for name, page in pages.items() if page == 65001)
    graph_only = sorted(set(pages) - set(utf8))
    return {"configured_code_pages": {c.name: c.code_page for c in contexts}, "actual_source_code_pages": pages,
            "raw_map_sha256": hashlib.sha256(raw).hexdigest(), "same_analysis_raw_capture_unchanged": True,
            "utf8_flow_contexts": utf8, "non_utf8_graph_only_contexts": graph_only,
            "utf8_flow_context_count": len(utf8), "non_utf8_graph_only_context_count": len(graph_only),
            "actual_mixed_encodings": len(set(pages.values())) > 1,
            "coverage": "Availability follows actual SourceText encoding; configured pages do not guarantee actual mixed groups."}


def candidate_identity(client):
    dependency = json.loads((ROOT / "analyzers.lock.json").read_text())["callatlas"]
    return {"engine_path": str(client.engine), "engine_sha256": hashlib.sha256(client.engine.read_bytes()).hexdigest(),
            "callatlas_revision": dependency["revision"], "combined_patch_sha256": dependency["patch_sha256"]}


def acceptance_cases(fixtures):
    return [
        ("utf8", lambda client, work: utf8_acceptance(client, fixtures["utf8-bom"], work)),
        ("utf16", lambda client, work: graph_only_encoding(client, fixtures["utf16"], work)),
        ("latin1_ascii", lambda client, work: graph_only_encoding(client, fixtures["latin1-ascii"], work)),
        ("linked", linked_contexts),
        ("mixed_encoding_contexts", mixed_encoding_contexts),
    ]


def run_case(client, work, named_case, stopped):
    name, case = named_case
    try:
        case_work = work / name
        case_work.mkdir()
        outcome = case(client, case_work)
        return outcome, client.records
    except BaseException:
        # Signal failure in the worker before the coordinator can refill slots.
        stopped.set()
        raise


def submit_cases(executor, pending, active, stopped):
    while len(active) < CASE_WORKERS and not stopped.is_set():
        item = next(pending, None)
        if item is None:
            break
        index, operation = item
        active[executor.submit(operation)] = index


def run_cases(engine, environment, cases, work):
    """Bound independent cases; each callback's mutation/recovery chain stays ordered."""
    stopped = Event()
    pending = iter((index, partial(run_case, Client(engine, environment), work, case, stopped))
                   for index, case in enumerate(cases))
    active, completed = {}, {}
    with ThreadPoolExecutor(max_workers=CASE_WORKERS) as executor:
        try:
            submit_cases(executor, pending, active, stopped)
            while active:
                done, _ = wait(active, return_when=FIRST_COMPLETED)
                for future in done:
                    completed[active.pop(future)] = future.result()
                submit_cases(executor, pending, active, stopped)
        except BaseException:
            stopped.set()
            for future in active:
                future.cancel()
            # Executor exit waits for every active callback before owned fixture
            # directories can be removed, including interruption/failure paths.
            raise
    return ordered_case_results(cases, completed)


def ordered_case_results(cases, completed):
    outcomes = {name: completed[index][0] for index, (name, _) in enumerate(cases)}
    records = [record for index in range(len(cases)) for record in completed[index][1]]
    return outcomes, records


def verify_candidate(client, identity):
    require(hashlib.sha256(client.engine.read_bytes()).hexdigest() == identity["engine_sha256"],
            "Candidate engine changed during acceptance")


def run_candidate_cases(client, identity, cases, work):
    try:
        result = run_cases(client.engine, client.environment, cases, work)
    except BaseException as original:
        # run_cases has drained its workers before this proof, even on failure.
        try:
            verify_candidate(client, identity)
        except BaseException as identity_error:
            original.add_note(f"Candidate identity verification also failed: {identity_error}")
        raise
    verify_candidate(client, identity)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    client = Client(args.engine)
    identity = candidate_identity(client)
    capabilities = json.loads(client.command(["capabilities"], ROOT).stdout)
    require("csharp" in capabilities["flow_languages"], "CLI does not advertise C# flow support")
    fixtures = {f["name"]: f for f in json.loads(BASELINE.read_text())["fixtures"]}
    with tempfile.TemporaryDirectory(prefix="task22-ingestion-acceptance-") as directory:
        work = Path(directory).resolve()
        outcomes, records = run_candidate_cases(client, identity, acceptance_cases(fixtures), work)
        report = {"status": "passed", "candidate": identity, **outcomes,
                  "commands": client.records + records}
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2).replace(str(work), "<acceptance-work>") + "\n")
    print(f"C# flow ingestion CLI acceptance passed; report: {args.output}")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        notes = ''.join(f"\n{note}" for note in getattr(error, "__notes__", []))
        raise SystemExit(f"C# flow ingestion acceptance failed: {error}{notes}") from error
