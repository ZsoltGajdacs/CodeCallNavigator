"""Owned source and named fixture-role proofs over actual CLI comparison replies."""
from dataclasses import dataclass
from pathlib import Path

from csharp_flow_acceptance import require
BASE_REF = "refs/tags/fixture-before"
HEAD_REF = "refs/tags/fixture-after"
ROLES = ("changed", "unchanged", "added", "removed", "legacy", "manual", "target")
LABELS = {"changed": "Changed", "unchanged": "Unchanged", "added": "Added", "removed": "Removed", "legacy": "Legacy", "manual": "Manual", "target": "Leaf"}

def comparison(client, project, selection, expected=None):
    operation = {"kind": "flow_comparison", "base": BASE_REF, "head": HEAD_REF,
                 "selection": selection}
    if expected is not None:
        operation["expected_comparison"] = expected
    return client.query(project, operation)["result"]


def present_nodes(catalog):
    for record in catalog["records"]:
        for name in ("before", "after"):
            side = record[name]
            if side["status"] == "present" and side.get("node"):
                yield record["symbol"], side["node"]


def unique_identity(nodes, label):
    found = {symbol for symbol, node in nodes if node["label"] == label}
    require(len(found) == 1, f"Compiler fixture requires one exact {label} identity")
    return found.pop()


def identities(catalog):
    nodes = list(present_nodes(catalog))
    return {role: unique_identity(nodes, label) for role, label in LABELS.items()}


def expected_side(side, inputs):
    if side["status"] == "ready":
        require(side["source"]["origin"] == "analyzed" and side["node"]["owner"]["language"] == "csharp",
                "Native expectations require actual C# analyzed source")
        filename = Path(side["source"]["file"]).name
        require(filename in inputs and side["source"]["file"] == side["node"]["owner"]["file"]
                and side["source"]["content"].encode("utf-8") == inputs[filename],
                "CLI cached source differs from the exact compiler input bytes")
        return {"status": "ready", "symbol": side["node"]["symbol"],
                "source": side["source"]["content"], "chart": side["chart"]}
    require(side["status"] in ("missing", "unavailable"), "Unexpected endpoint availability")
    return {"status": side["status"], **({"reason": side["reason"]}
                                        if side["status"] == "unavailable" else {})}


@dataclass
class Expectations:
    client: object
    project: Path
    initial: dict
    inputs: dict

    def cases(self, catalog):
        ids = identities(catalog)
        results = [self.case(role, ids) for role in ROLES]
        validate_roles(results)
        return results

    def case(self, role, ids):
        symbol = ids["changed"] if role == "manual" else ids[role]
        head_symbol = ids["manual"] if role == "manual" else None
        selection = {"kind": "detail", "symbol": symbol, "head_symbol": head_symbol}
        detail = comparison(self.client, self.project, selection, self.initial["comparison_id"])
        return {"kind": role, "symbol": symbol, "head_symbol": head_symbol,
                "before": expected_side(detail["before"], self.inputs["Before"]),
                "after": expected_side(detail["after"], self.inputs["After"])}


def ready(side):
    return side["status"] == "ready"


def same_facts(first, second):
    return (first["source"], first["chart"]) == (second["source"], second["chart"])


def recorded_call(chart, symbol):
    targets = [target for step in chart["steps"] for call in step["calls"] for target in call["targets"]]
    return any(value["symbol"] == symbol and value.get("kind", "call") == "call" for value in targets)


def validate_presence(roles):
    require(roles["added"]["before"]["status"] == "missing"
            and ready(roles["added"]["after"]), "Added role is not missing-to-ready")
    require(ready(roles["removed"]["before"])
            and roles["removed"]["after"]["status"] == "missing", "Removed role is not ready-to-missing")
    require(roles["legacy"]["before"] == {"status": "unavailable", "reason": "missing_function"}
            and ready(roles["legacy"]["after"]), "Historical abstract declaration must have unavailable flow")


def validate_correspondence(roles):
    changed, manual = roles["changed"], roles["manual"]
    require(manual["symbol"] == changed["symbol"] and manual["head_symbol"] != changed["symbol"],
            "Manual counterpart was not explicit")
    target = roles["target"]["symbol"]
    for side in (changed["before"], changed["after"]):
        require(recorded_call(side["chart"], target), "Target is not a recorded compiler call")


def validate_roles(results):
    roles = {case["kind"]: case for case in results}
    require(tuple(roles) == ROLES, "Expected all seven ordered native roles")
    for name in ("changed", "unchanged", "manual", "target"):
        require(ready(roles[name]["before"]) and ready(roles[name]["after"]), "Expected two compiler-ready sides")
    changed, unchanged = roles["changed"], roles["unchanged"]
    require(changed["before"]["source"] != changed["after"]["source"]
            and changed["before"]["chart"] != changed["after"]["chart"],
            "Changed fixture must change semantic chart and source")
    require(same_facts(unchanged["before"], unchanged["after"]), "Unchanged source/chart must be identical")
    validate_presence(roles)
    validate_correspondence(roles)
