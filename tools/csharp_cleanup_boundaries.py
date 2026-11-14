"""Real runtime observations for explicitly partial C# cleanup boundaries."""
import hashlib
import json

from csharp_flow_acceptance import require


BOUNDARIES = [
    ("filter-accepted", "Filtered", ["try-body", "filter", "filtered-catch", "filter-outer-finally"], "normal"),
    ("filter-rejected", "Filtered", ["try-body", "filter", "filter-outer-finally"], "throw"),
    ("async-body", "AsyncBody", ["async-before", "async-after"], "normal"),
    ("await-using", "AwaitUsing", ["await-using-body", "dispose-async"], "normal"),
    ("await-foreach", "AwaitForeach", ["await-foreach-body"], "normal"),
    ("iterator", "Iterator", ["iterator-body"], "normal"),
    ("guarded-switch", "GuardedSwitch", ["guard", "guarded-arm"], "normal"),
    ("goto", "Goto", ["after-goto"], "normal"),
]


def query_method(client, root, snapshot, method):
    matches = [node for node in snapshot["result"]["nodes"] if f"::{method}(" in node["signature"]]
    require(len(matches) == 1, f"Expected one boundary callable {method}")
    return client.query(root, {"kind": "flow", "symbol": matches[0]["id"]}, snapshot["generation"])["result"]


def runtime(execute, root, name):
    return json.loads(execute([root / "Runtime/bin/Release/net11.0/CleanupRuntime.dll", name]))


def opaque_case(client, root, snapshot, execute, case):
    name, method, markers, outcome = case
    actual = runtime(execute, root, name)
    require([item["name"] for item in actual["events"]] == markers and actual["outcome"] == outcome,
            f"Boundary runtime {name} did not execute expected source")
    result = query_method(client, root, snapshot, method)
    chart = result["chart"]
    opaque = [step for step in chart["steps"] if step["kind"] == "unsupported"]
    require(opaque, f"Unsupported boundary {name} lost its partial marker")
    calls = [call["label"] for step in chart["steps"] for call in step["calls"]]
    validate_opaque_calls(name, calls)
    return {"status": "passed", "runtime": actual, "chart": chart, "opaque_steps": len(opaque),
            "no_full_runtime_path_claim": True,
            "projection_source": result["source"]["content"] if name.startswith("filter-") else None,
            "source_sha256": hashlib.sha256(result["source"]["content"].encode()).hexdigest()}


def validate_opaque_calls(name, calls):
    if name.startswith("filter-"):
        validate_filter_calls(calls)
    elif name == "guarded-switch":
        require(not any("Guard(" in call or "Arm(" in call for call in calls), "Opaque guarded switch exposed eager branch calls")
    elif name != "goto":
        require(not calls, "Deferred/suspended callable exposed eager body calls")


def validate_filter_calls(calls):
    forbidden = ["Filter(", '"try-body"', '"filtered-catch"']
    require(not any(marker in call for call in calls for marker in forbidden), "Opaque filtered region exposed eager calls")
    require("TraceRuntime.Mark(TraceRuntime.FilterFinallyMarker)" in calls, "Filtered region lost outer finally")


def resource_budget(client, root, snapshot, execute):
    actual = runtime(execute, root, "resource-budget")
    events = actual["events"]
    expected = [("acquire", f"budget-{index}") for index in range(40)] + [("mark", "budget-body")]
    expected += [("dispose", f"budget-{index}") for index in reversed(range(40))]
    require([(item["operation"], item["name"]) for item in events] == expected,
            "Shallow multi-resource fixture did not execute real nested cleanup")
    result = query_method(client, root, snapshot, "ShallowResources")
    chart = result["chart"]
    require(any(step["kind"] == "unsupported" for step in chart["steps"]), "Synthetic resource depth did not become bounded/partial")
    require(len(chart["steps"]) < 100, "Shallow resource declaration escaped synthetic flow size bound")
    return {"status": "passed", "runtime": actual, "chart": chart,
            "no_full_runtime_path_claim": True, "resource_count": 40}


def run_boundaries(client, root, snapshot, execute):
    results = {case[0]: opaque_case(client, root, snapshot, execute, case) for case in BOUNDARIES}
    results["resource-budget"] = resource_budget(client, root, snapshot, execute)
    return results
