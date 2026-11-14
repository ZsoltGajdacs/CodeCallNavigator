#!/usr/bin/env python3
"""Compare executed .NET cleanup markers with source-owned flow-chart paths."""
import argparse
from collections import deque
from dataclasses import dataclass
from enum import Enum
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

from csharp_flow_acceptance import require
from csharp_flow_ingestion_acceptance import Client, candidate_identity
from csharp_cleanup_boundaries import run_boundaries


ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "fixtures/flow/csharp-cleanup"
FINALLY_MARKERS = {f"TraceRuntime.Mark(TraceRuntime.{field}Marker)": name for field, name in
                   [("FilterFinally", "filter-outer-finally"), ("InnerFinally", "inner-finally"),
                    ("OuterFinally", "outer-finally"), ("Finally", "finally")]}


@dataclass(frozen=True)
class EventRule:
    operation: str
    name: str
    implicit: bool = False


@dataclass(frozen=True)
class SourceContext:
    content: bytes
    resources: dict


class Completion(Enum):
    NORMAL = "normal"
    EXCEPTION = "exception"


class CaseKind(Enum):
    RESOURCE = "resource"
    GUARDED_RESOURCE = "guarded-resource"
    CONTROL = "control"
    LOCK = "lock"


@dataclass(frozen=True)
class ResourceBinding:
    runtime_name: str
    owner_anchor: str


@dataclass(frozen=True)
class WalkState:
    node: int
    position: int
    path: tuple = ()
    completion: Completion = Completion.NORMAL


@dataclass(frozen=True)
class MappedFlow:
    chart: dict
    source: str
    rules: dict
    path: list | None


@dataclass(frozen=True)
class Case:
    name: str
    method: str
    resources: dict
    expected: tuple
    outcome: str = "normal"
    kind: CaseKind = CaseKind.RESOURCE


CASES = [
    Case("using-scope", "CleanupCases::UseScope", {"resource": "one"},
         (("acquire", "one", "normal"), ("mark", "body", "normal"), ("dispose", "one", "normal"))),
    Case("acquisition-failure", "CleanupCases::AcquisitionFailure", {"first": "first", "second": "second"},
         (("acquire", "first", "normal"), ("acquire", "second", "throw"), ("dispose", "first", "normal")), "throw"),
    Case("dispose-throws-outer", "CleanupCases::DisposeThrowsOuter", {"outer": "outer", "inner": "inner"},
         (("acquire", "outer", "normal"), ("acquire", "inner", "normal"), ("mark", "body", "normal"),
          ("dispose", "inner", "throw"), ("dispose", "outer", "normal")), "throw"),
    Case("acquisition-failure-caught", "CleanupCases::AcquisitionFailureCaught", {"first": "first", "second": "second"},
         (("acquire", "first", "normal"), ("acquire", "second", "throw"), ("dispose", "first", "normal"),
          ("mark", "caught", "normal"), ("mark", "after-catch", "normal"))),
]
CASES += [
    Case("nested-using", "CleanupCases::NestedUsing", {"outer": "outer", "inner": "inner"},
         (("acquire", "outer", "normal"), ("acquire", "inner", "normal"), ("mark", "body", "normal"),
          ("dispose", "inner", "normal"), ("mark", "after-inner", "normal"), ("mark", "after-body", "normal"),
          ("dispose", "outer", "normal"))),
    Case("return-before-dispose", "CleanupCases::ReturnBeforeDispose", {"resource": "one"},
         (("acquire", "one", "normal"), ("return-value", "result", "normal"), ("dispose", "one", "normal"))),
    Case("capture-mutation", "CleanupCases::CaptureMutation", {"using expression": ResourceBinding("original", "current")},
         (("acquire", "original", "normal"), ("acquire", "replacement", "normal"), ("mark", "mutated", "normal"),
          ("dispose", "original", "normal")), kind=CaseKind.GUARDED_RESOURCE),
    Case("null-reference", "CleanupCases::NullReference", {"using expression": ResourceBinding("null", "missing")},
         (("acquire", "null", "normal"), ("mark", "null-body", "normal")), kind=CaseKind.GUARDED_RESOURCE),
    Case("null-nullable", "CleanupCases::NullNullable", {"using expression": ResourceBinding("nullable", "missing")},
         (("acquire", "nullable", "normal"), ("mark", "nullable-body", "normal")), kind=CaseKind.GUARDED_RESOURCE),
    Case("loop-transfers", "CleanupCases::LoopTransfers", {"iteration": "iteration"},
         (("acquire", "iteration", "normal"), ("mark", "body", "normal"), ("dispose", "iteration", "normal"),
          ("acquire", "iteration", "normal"), ("mark", "body", "normal"), ("dispose", "iteration", "normal"),
          ("mark", "after-loop", "normal"))),
    Case("rethrow", "ExceptionCases::Rethrow", {},
         (("mark", "body", "normal"), ("mark", "caught", "normal"), ("mark", "inner-finally", "normal"),
          ("mark", "outer-finally", "normal")), "throw", CaseKind.CONTROL),
    Case("finally-throws", "ExceptionCases::FinallyThrows", {},
         (("mark", "body", "normal"), ("return-value", "result", "normal"), ("mark", "finally", "normal")),
         "throw", CaseKind.CONTROL),
]
GET = ("get-enumerator", "enumerator", "normal")
MOVE = ("move-next", "enumerator", "normal")
CURRENT = ("current", "enumerator", "normal")
ITEM = ("mark", "item", "normal")
DISPOSE = ("dispose", "enumerator", "normal")
AFTER = ("mark", "after-loop", "normal")
ITERATION = (MOVE, CURRENT, ITEM)
CASES += [
    Case("foreach-normal", "ForeachCases::Run", {}, (GET, *ITERATION, *ITERATION, MOVE, DISPOSE, AFTER)),
    Case("foreach-break", "ForeachCases::Break", {}, (GET, *ITERATION, DISPOSE, AFTER)),
    Case("foreach-continue", "ForeachCases::Continue", {},
         (GET, *ITERATION, ("continue-decision", "iteration", "normal"), *ITERATION,
          ("continue-decision", "iteration", "normal"), ("mark", "tail-item", "normal"), MOVE, DISPOSE, AFTER)),
    Case("foreach-acquire-failure", "ForeachCases::Run", {},
         (("get-enumerator", "enumerator", "throw"),), "throw"),
    Case("foreach-move-failure", "ForeachCases::Run", {},
         (GET, ("move-next", "enumerator", "throw"), DISPOSE), "throw"),
    Case("foreach-current-failure", "ForeachCases::Run", {},
         (GET, MOVE, ("current", "enumerator", "throw"), DISPOSE), "throw"),
    Case("foreach-dispose-failure", "ForeachCases::Break", {},
         (GET, *ITERATION, ("dispose", "enumerator", "throw")), "throw"),
    Case("foreach-body-failure", "ForeachCases::BodyFailure", {},
         (GET, MOVE, CURRENT, ("mark", "body", "normal"), DISPOSE), "throw"),
    Case("lock-normal", "LockCases::Normal", {},
         (("gate", "lock", "normal"), ("lock-inside", "lock", "normal"),
          ("mark", "locked-body", "normal"), ("mark", "after-lock", "normal")), kind=CaseKind.LOCK),
    Case("lock-throwing", "LockCases::Throwing", {},
         (("gate", "lock", "normal"), ("lock-inside", "lock", "normal"),
          ("mark", "locked-body", "normal")), "throw", CaseKind.LOCK),
]


def call_rule(call, context):
    label = call["label"]
    if label.startswith("synthetic C#:") and "enumerator" in label.lower():
        return enumerator_rule(call, context)
    if label.startswith("synthetic C#: Dispose captured "):
        return dispose_rule(call, context)
    return observed_call_rule(call, context)


def observed_call_rule(call, context):
    label = call["label"]
    if label in FINALLY_MARKERS:
        return explicit_rule("mark", FINALLY_MARKERS[label], call, context)
    if label in ["Gate()", "Inside()"]:
        return explicit_rule("gate" if label == "Gate()" else "lock-inside", "lock", call, context)
    if label == "TraceRuntime.ShouldContinue(item)":
        return explicit_rule("continue-decision", "iteration", call, context)
    if label == "TraceRuntime.ReturnValue()":
        return explicit_rule("return-value", "result", call, context)
    match = re.match(r'TraceRuntime\.(Acquire|AcquireReference|AcquireNull|AcquireNullable|Mark)\((?:new\()?"([^"]+)"', label)
    if match is None:
        return None
    return explicit_rule("mark" if match[1] == "Mark" else "acquire", match[2], call, context)


def explicit_rule(operation, name, call, context):
    span = call["span"]
    require(context.content[span["start"]:span["end"]].decode("utf-8") == call["label"],
            "Explicit marker call lost its exact source span")
    return EventRule(operation, name)


def enumerator_rule(call, context):
    span = call["span"]
    owner = context.content[span["start"]:span["end"]].decode("utf-8")
    require(span["end"] > span["start"] and "foreach" in owner, "Enumeration operation lost its foreach source owner")
    operations = {"GetEnumerator": "get-enumerator", "MoveNext": "move-next", "Current": "current", "Dispose": "dispose"}
    matched = [operation for name, operation in operations.items() if name in call["label"]]
    require(len(matched) == 1, "Unrecognized implicit enumeration marker contract")
    operation = next(name for name in operations if name in call["label"])
    implicit_contract(call, operation)
    return EventRule(matched[0], "enumerator", True)


def dispose_rule(call, context):
    implicit_contract(call, "Dispose")
    span = call["span"]
    owner = context.content[span["start"]:span["end"]].decode("utf-8")
    require(span["end"] > span["start"], "Implicit Dispose lacks a meaningful owning source span")
    names = [name for name in context.resources if re.search(r'\b' + re.escape(name) + r'\b', call["label"])]
    require(len(names) == 1, "Implicit Dispose lost its captured resource label slot")
    binding = context.resources[names[0]]
    if isinstance(binding, str):
        binding = ResourceBinding(binding, names[0])
    require(binding.owner_anchor in owner, "Implicit Dispose lost its fixture source owner")
    return EventRule("dispose", binding.runtime_name, True)


def implicit_contract(call, operation):
    require(call["targets"] == [] and call["diagnostic"].startswith(f"roslyn:implicit:{operation};"),
            "Compiler-lowered operation invented a graph target or lost its explicit operation diagnostic")


def chart_rules(chart, source, resources):
    context = SourceContext(source.encode("utf-8"), resources)
    result = {}
    for step in chart["steps"]:
        rules = [call_rule(call, context) for call in step["calls"]]
        result[step["id"]] = [rule for rule in rules if rule is not None]
        for rule in result[step["id"]]:
            if rule.operation == "move-next":
                require(step["kind"] == "decision", "MoveNext result must own its boolean branch decision")
    return result


def consume(rules, events, position):
    last = None
    for rule in rules:
        if position >= len(events):
            return None
        event = events[position]
        if (event["operation"], event["name"]) != (rule.operation, rule.name):
            return None
        position += 1
        last = event
    return position, last


def edge_matches(edge, event, state, target):
    if event is None:
        return True
    if event["outcome"] == "throw":
        return edge["kind"] == "exception"
    if edge["kind"] == "exception":
        return state.completion is Completion.EXCEPTION and target["kind"] == "catch"
    if event["operation"] in ["move-next", "continue-decision"] and edge["kind"] in ["yes", "no"]:
        require(event["value"] in ["true", "false"], "Runtime MoveNext did not record its returned boolean")
        return edge["kind"] == {"true": "yes", "false": "no"}[event["value"]]
    return True


def exit_matches(step, outcome):
    if outcome == "throw":
        return step["label"] == "Exception escapes"
    return step["label"] in ["Function exit", "Return to caller"]


def chart_index(chart):
    steps = {step["id"]: step for step in chart["steps"]}
    outgoing = {node: [] for node in steps}
    for edge in chart["links"]:
        outgoing[edge["from"]].append(edge)
    return steps, outgoing


def completed(step, position, event_count, outcome):
    if step["kind"] != "exit":
        return False
    return position == event_count and exit_matches(step, outcome)


def next_states(edges, state, event, steps):
    return [WalkState(edge["to"], state.position, state.path + (state.node,),
                      Completion.EXCEPTION if edge["kind"] == "exception" else state.completion)
            for edge in edges if edge_matches(edge, event, state, steps[edge["to"]])]


def observed_completion(step, state, event):
    if step["kind"] == "catch":
        return Completion.NORMAL
    if step["kind"] == "throw":
        return Completion.EXCEPTION
    if event is not None and event["outcome"] == "throw":
        return Completion.EXCEPTION
    return state.completion


def trace_path(chart, rules, events, outcome):
    steps, outgoing = chart_index(chart)
    pending = deque([WalkState(chart["entry"], 0)])
    seen = set()
    while pending:
        state = pending.popleft()
        key = state.node, state.position, state.completion
        if key in seen:
            continue
        seen.add(key)
        consumed = consume(rules[state.node], events, state.position)
        if consumed is None:
            continue
        position, last = consumed
        if completed(steps[state.node], position, len(events), outcome):
            return list(state.path + (state.node,))
        advanced = WalkState(state.node, position, state.path, observed_completion(steps[state.node], state, last))
        pending.extend(next_states(outgoing[state.node], advanced, last, steps))
    return None


def dotnet(arguments):
    result = subprocess.run(["dotnet", *map(str, arguments)], capture_output=True, text=True, timeout=180)
    require(result.returncode == 0, f"dotnet {arguments} failed:\n{(result.stderr + result.stdout)[-4000:]}")
    return result.stdout


def runtime_trace(root, case):
    executable = "TopLevel/bin/Release/net11.0/TopLevel.dll" if case.name == "top-level-using" else "Runtime/bin/Release/net11.0/CleanupRuntime.dll"
    output = dotnet([root / executable, case.name])
    actual = json.loads(output)
    require(tuple((event["operation"], event["name"], event["outcome"]) for event in actual["events"])
            == case.expected, f"Real .NET fixture {case.name} did not exercise the expected operations")
    require(actual["outcome"] == case.outcome, f"Unexpected real .NET completion for {case.name}")
    return actual


def build_fixture(root):
    project = root / "Runtime/CleanupRuntime.csproj"
    dotnet(["build", project, "-c", "Release", "--nologo"])
    return project


def analyze_fixture(client, root, project):
    client.analyze(root, project)
    return client.query(root, {"kind": "snapshot"})


def scope_chart(client, root, snapshot, case):
    nodes = [node for node in snapshot["result"]["nodes"]
             if f"{case.method}(" in node["signature"]]
    if case.name == "top-level-using":
        nodes = [node for node in snapshot["result"]["nodes"] if node["signature"].endswith("::<top-level>")]
    require(len(nodes) == 1, "Expected one source cleanup fixture callable")
    reply = client.query(root, {"kind": "flow", "symbol": nodes[0]["id"]}, snapshot["generation"])
    return reply["result"]


def event(operation, name, outcome="normal", value=None):
    return {"operation": operation, "name": name, "outcome": outcome, "value": value}


def forbidden_traces(case, actual):
    if case.kind in [CaseKind.GUARDED_RESOURCE, CaseKind.LOCK]:
        return {}  # Runtime values do not become a lifetime/nullness interpreter.
    if case.kind is CaseKind.CONTROL:
        return forbidden_control(case, actual)
    if case.name.startswith("foreach-"):
        return forbidden_foreach(case, actual)
    return forbidden_resource(case, actual)


def forbidden_resource(case, actual):
    events = actual["events"]
    if case.name == "acquisition-failure-caught":
        return {"skip-dispose-before-catch": ([item for item in events if item["operation"] != "dispose"], "normal"),
                "dispose-unacquired-before-catch": (events[:2] + [event("dispose", "second")] + events[2:], "normal")}
    dispose = max(index for index, item in enumerate(events) if item["operation"] == "dispose")
    negatives = {"skip-mandatory-dispose": (events[:dispose] + events[dispose + 1:], actual["outcome"])}
    if case.name == "acquisition-failure":
        negatives["dispose-unacquired-second"] = (events[:2] + [event("dispose", "second")] + events[2:], "throw")
        negatives["body-after-failed-acquisition"] = (events[:2] + [event("mark", "impossible-body")] + events[2:], "throw")
    if case.outcome == "throw":
        negatives["throw-resumes-normal-completion"] = (events, "normal")
    if case.name == "return-before-dispose":
        negatives["dispose-before-return-value"] = (events[:1] + [events[2], events[1]], "normal")
    return negatives


def forbidden_control(case, actual):
    events = actual["events"]
    markers = {"rethrow": ["inner-finally", "outer-finally"], "finally-throws": ["finally"]}
    negatives = {"exception-resumes-normal": (events, "normal")}
    for name in markers[case.name]:
        negatives["skip-" + name] = ([item for item in events if item["name"] != name], actual["outcome"])
    return negatives


def forbidden_foreach(case, actual):
    events = actual["events"]
    negatives = {}
    if case.name == "foreach-acquire-failure":
        negatives["dispose-unacquired-enumerator"] = (events + [event("dispose", "enumerator")], "throw")
    else:
        negatives["skip-enumerator-dispose"] = ([item for item in events if item["operation"] != "dispose"], actual["outcome"])
    if case.outcome == "throw":
        negatives["exception-resumes-normal"] = (events, "normal")
    negatives.update(forbidden_enumeration_failures(case, events))
    if case.name == "foreach-break":
        negatives["move-next-after-break"] = (events[:4] + [event("move-next", "enumerator", value="true")] + events[4:], "normal")
    if case.name == "foreach-continue":
        position = next(index for index, item in enumerate(events) if item["operation"] == "continue-decision" and item["value"] == "true") + 1
        negatives["tail-after-observed-continue"] = (events[:position] + [event("mark", "tail-item")] + events[position:], "normal")
    if case.name in ["foreach-normal", "foreach-continue"]:
        position = next(index for index, item in enumerate(events) if item["operation"] == "move-next" and item["value"] == "false") + 1
        negatives["current-after-move-next-false"] = (events[:position] + [event("current", "enumerator"), event("mark", "item")] + events[position:], "normal")
    return negatives


def forbidden_enumeration_failures(case, events):
    suffixes = {
        "foreach-acquire-failure": [event("move-next", "enumerator", value="true"), event("current", "enumerator"), event("mark", "item")],
        "foreach-move-failure": [event("current", "enumerator"), event("mark", "item")],
        "foreach-current-failure": [event("mark", "item")],
        "foreach-dispose-failure": [event("mark", "after-loop")],
    }
    suffix = suffixes.get(case.name)
    if suffix is None:
        return {}
    position = next(index for index, item in enumerate(events) if item["outcome"] == "throw") + 1
    return {"source-body-after-failed-operation": (events[:position] + suffix + events[position:], "throw")}


def case_acceptance(client, root, snapshot, case):
    actual = runtime_trace(root, case)
    result = scope_chart(client, root, snapshot, case)
    chart = result["chart"]
    rules = chart_rules(chart, result["source"]["content"], case.resources)
    path = trace_path(chart, rules, actual["events"], actual["outcome"])
    negatives = {name: trace_path(chart, rules, events, outcome)
                 for name, (events, outcome) in forbidden_traces(case, actual).items()}
    opaque = any(step["kind"] == "unsupported" for step in chart["steps"])
    structure = case_structure(case, actual, MappedFlow(chart, result["source"]["content"], rules, path))
    passed = path is not None and all(path is None for path in negatives.values()) and not opaque
    return {"status": "passed" if passed else "failed", "runtime": actual, "chart": chart,
            "runtime_trace_path": path, "forbidden_paths": negatives,
            "opaque_steps_present": opaque,
            "structure": structure,
            "source_sha256": hashlib.sha256(result["source"]["content"].encode()).hexdigest()}


def case_structure(case, actual, flow):
    if case.kind is CaseKind.LOCK:
        return lock_structure(actual, flow)
    if case.name == "return-before-dispose":
        require(actual["result"] == 7 and actual["sharedValue"] == 99, "C# return value was not captured before disposal mutation")
        return {"observed_return_before_dispose_mutation": True}
    if case.kind is CaseKind.GUARDED_RESOURCE:
        return guarded_structure(case, flow.chart, flow.rules, flow.path)
    return {}


def lock_structure(actual, flow):
    chart, rules, path = flow.chart, flow.rules, flow.path
    probe = actual["lockProbe"]
    require(probe["released"] and probe["callerThread"] != probe["workerThread"],
            "Another thread could not acquire the actual monitor after scope exit")
    require(next(item for item in actual["events"] if item["operation"] == "lock-inside")["value"] == "true",
            "Monitor.IsEntered did not confirm the genuine lock body")
    enters, exits = monitor_steps(chart, flow.source)
    require(path is not None and any(node in path for node in enters), "Observed lock body does not follow compiler Monitor.Enter")
    body = next(node for node in path if any(rule.operation == "lock-inside" for rule in rules[node]))
    require(any(path.index(node) < path.index(body) for node in enters if node in path), "Monitor.Enter must precede observed body")
    lock_exit_guards(chart, exits)
    lock_entry_structure(chart, rules, enters)
    return {"other_thread_release_observed": True, "monitor_enter_steps": enters, "monitor_exit_steps": exits,
            "monitor_events_inferred_from_compiler_structure_not_runtime_instrumented": True}


def lock_exit_guards(chart, exits):
    guards = {step["id"] for step in chart["steps"] if step["kind"] == "decision" and "lockTaken is true" in step["label"]}
    guarded = {link["to"] for link in chart["links"] if link["from"] in guards and link["kind"] == "yes"}
    require(set(exits) <= guarded, "Monitor.Exit must retain its lockTaken guard")


def monitor_steps(chart, source):
    enters, exits = [], []
    for step in chart["steps"]:
        for call in step["calls"]:
            if not call["label"].startswith("synthetic C#: Monitor."):
                continue
            implicit_contract(call, "MonitorEnter" if "Monitor.Enter" in call["label"] else "MonitorExit")
            require(call["span"]["end"] > call["span"]["start"], "Monitor operation lost its source owner")
            owner = source.encode()[call["span"]["start"]:call["span"]["end"]].decode()
            require(owner.startswith("lock (") and "Gate()" in owner, "Monitor operation does not own the genuine lock syntax")
            (enters if "Monitor.Enter" in call["label"] else exits).append(step["id"])
    require(enters and exits, "Actual lock chart lacks compiler-selected Monitor operations")
    return enters, exits


def lock_entry_structure(chart, rules, enters):
    body = {node for node, values in rules.items() if any(rule.operation == "lock-inside" for rule in values)}
    reachable = reachable_without(chart, set(enters))
    require(len(enters) == 1 and body and not body.intersection(reachable), "Lock body is reachable without compiler Monitor.Enter")
    require(sum(any(rule.operation == "gate" for rule in values) for values in rules.values()) == 1,
            "Genuine lock expression must evaluate Gate exactly once")
    require(sum("capture lock object once" in step["label"] for step in chart["steps"]) == 1,
            "Lock object must be captured exactly once")
    lock_enter_failure_guard(chart, enters)


def lock_enter_failure_guard(chart, enters):
    guards = {step["id"] for step in chart["steps"] if "lockTaken is true" in step["label"]}
    exceptional = {edge["to"] for edge in chart["links"] if edge["from"] in enters and edge["kind"] == "exception"}
    require(exceptional and exceptional <= guards, "Monitor.Enter failure is not inside guarded cleanup protection")


def reachable_without(chart, blocked):
    _, outgoing = chart_index(chart)
    pending = [chart["entry"]]
    seen = set()
    while pending:
        node = pending.pop()
        if node in blocked or node in seen:
            continue
        seen.add(node)
        pending.extend(edge["to"] for edge in outgoing[node])
    return seen


def guarded_structure(case, chart, rules, path):
    dispose = {node for node, values in rules.items() if any(rule.operation == "dispose" and rule.implicit for rule in values)}
    guard_name = "implicit HasValue check" if case.name == "null-nullable" else "implicit non-null check"
    guards = {step["id"] for step in chart["steps"] if step["kind"] == "decision" and guard_name in step["label"]}
    require(dispose and guards, "Guarded using cleanup lost its intrinsic source guard")
    for node in dispose:
        require(any(edge["to"] == node and edge["from"] in guards and edge["kind"] == "yes" for edge in chart["links"]),
                "Dispose is not guarded by the compiler's intrinsic captured-value check")
    if case.name == "capture-mutation":
        capture_mutation_order(chart, rules, path)
    return {"intrinsic_guard": guard_name, "guarded_dispose_steps": len(dispose),
            "no_runtime_nullness_interpretation": True}


def capture_mutation_order(chart, rules, path):
    captures = [step["id"] for step in chart["steps"] if "capture using resource using expression once" in step["label"]]
    replacements = [node for node, values in rules.items() if any(rule.operation == "acquire" and rule.name == "replacement" for rule in values)]
    disposals = [node for node in path or [] if any(rule.operation == "dispose" for rule in rules[node])]
    require(len(captures) == len(replacements) == len(disposals) == 1, "Using expression must capture once independently of body mutation")
    require(path.index(captures[0]) < path.index(replacements[0]) < path.index(disposals[0]),
            "Using expression capture does not precede reassignment and cleanup")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--case", action="append", choices=[case.name for case in CASES])
    args = parser.parse_args()
    client = Client(args.engine)
    candidate, producer = candidate_identity(client), producer_identity(client)
    with tempfile.TemporaryDirectory(prefix="task22-cleanup-") as directory:
        work = Path(directory).resolve()
        root = work / "fixture"
        shutil.copytree(FIXTURE, root, ignore=shutil.ignore_patterns("bin", "obj"))
        project = build_fixture(root)
        snapshot = analyze_fixture(client, root, project)
        selected = CASES if args.case is None else [case for case in CASES if case.name in args.case]
        results = {case.name: case_acceptance(client, root, snapshot, case) for case in selected}
        if args.case is None:
            results.update(run_boundaries(client, root, snapshot, dotnet))
            validate_boundary_projections(results)
            results["top-level-using"] = top_level_acceptance(client, root)
        status = "passed" if all(result["status"] == "passed" for result in results.values()) else "failed"
        require(candidate_identity(client) == candidate and producer_identity(client) == producer,
                "Engine or producer artifacts changed during cleanup acceptance")
        report = {"status": status, "candidate": candidate,
                  "producer": producer, "candidate_unchanged_during_run": True,
                  "cases": results, "commands": client.records,
                  "trace_mapping": "Explicit source calls and source-owned compiler-lowered operations; actual outcomes constrain exception edges, not a runtime-value interpreter."}
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2).replace(str(work), "<cleanup-work>") + "\n")
    print(f"C# cleanup acceptance {report['status']}; report: {args.output}")
    if report["status"] != "passed":
        raise SystemExit(1)


def validate_boundary_projections(results):
    for name, result in results.items():
        source = result.pop("projection_source", None)
        if source is None:
            continue
        chart = result["chart"]
        rules = chart_rules(chart, source, {})
        actual = result["runtime"]
        projected = [item for item in actual["events"] if item["name"] == "filter-outer-finally"]
        path = trace_path(chart, rules, projected, actual["outcome"])
        skipped = trace_path(chart, rules, [], actual["outcome"])
        require(path is not None and skipped is None, f"{name} does not preserve observed outer-finally execution across opacity")
        result["projected_outer_finally_path"] = path
        result["skip_outer_finally_path"] = skipped
        result["projection_limit"] = "Filtered region is opaque; only observed surrounding finally events are matched. Filter timing and internal calls are not represented."


def producer_identity(client):
    packaged = client.engine.parent.parent / "Resources"
    bundle = packaged if (packaged / "analyzers").is_dir() else ROOT
    choices = [bundle / "analyzers/callatlas/callatlas.dll",
               bundle / "vendor/callatlas/src/CallAtlas.Cli/bin/Release/net11.0/callatlas.dll"]
    cli = next(path for path in choices if path.is_file())
    core = cli.with_name("CallAtlas.Core.dll")
    return {"resource_root": str(bundle), "cli_path": str(cli), "cli_sha256": hashlib.sha256(cli.read_bytes()).hexdigest(),
            "core_path": str(core), "core_sha256": hashlib.sha256(core.read_bytes()).hexdigest(),
            "resolution": "Client clears resource overrides; sibling Resources/analyzers wins, otherwise the compiled workspace resource root is used."}


def top_level_acceptance(client, root):
    project = root / "TopLevel/TopLevel.csproj"
    dotnet(["build", project, "-c", "Release", "--nologo"])
    snapshot = analyze_fixture(client, root, project)
    case = Case("top-level-using", "", {"resource": "top"},
                (("acquire", "top", "normal"), ("mark", "top-after", "normal"), ("dispose", "top", "normal")))
    return case_acceptance(client, root, snapshot, case)


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        raise SystemExit(f"C# cleanup acceptance failed: {error}") from error
