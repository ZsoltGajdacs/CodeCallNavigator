"""Compare complete persisted analysis semantics independently of node numbering."""
import hashlib
import json
from pathlib import Path
import sqlite3


def active_database(cache):
    with sqlite3.connect(Path(cache) / "catalog.sqlite3") as connection:
        name = connection.execute("SELECT value FROM catalog WHERE key='active'").fetchone()[0]
    return Path(cache) / name


def graph_snapshot(cache):
    with sqlite3.connect(active_database(cache)) as connection:
        connection.row_factory = sqlite3.Row
        nodes = [dict(row) for row in connection.execute("SELECT * FROM nodes ORDER BY idx")]
        identities = {node["idx"]: node["id"] for node in nodes}
        metadata = json.loads(connection.execute("SELECT value FROM metadata WHERE key='graph'").fetchone()[0])
        provenance = metadata.get("edge_provenance", [])
        evidence = metadata.get("edge_evidence", [])
        edges = []
        for row in connection.execute("SELECT * FROM edges ORDER BY idx"):
            edge = dict(row)
            index = edge.pop("idx")
            edge["from"] = identities[edge.pop("from_idx")]
            edge["to"] = identities[edge.pop("to_idx")]
            edge["provenance"] = provenance[index] if provenance else ""
            if evidence and evidence[index]:
                edge["evidence"] = evidence[index]
            edges.append(edge)
    for node in nodes:
        del node["idx"]
        decode_visibility(node)
    return normalize_sets({
        "nodes": sorted(nodes, key=lambda node: node["id"]),
        "edges": sorted(edges, key=canonical),
        "entry_points": metadata.get("entry_points"),
        "diagnostics": metadata["diagnostics"],
        "roots": metadata["roots"],
        "engine_versions": metadata["engine_versions"],
        "method_policy": metadata["method_policy"],
        "architecture": metadata.get("architecture"),
        "flow_inventory": metadata.get("flow_inventory"),
        # Builds can rewrite identical outputs with new mtimes. Keep all content
        # and membership facts used for freshness; only omit observation times.
        "fingerprints": sorted((fingerprint_facts(item) for item in metadata.get("fingerprints", [])), key=canonical),
    })


def normalize_sets(graph):
    normalize_edges(graph.get('edges', []))
    for field in ('roots', 'diagnostics'):
        graph.get(field, []).sort()
    for section, field in [('architecture', 'files'), ('flow_inventory', 'functions')]:
        if graph.get(section):
            graph[section][field].sort(key=canonical)
    return graph


def normalize_edges(edges):
    for edge in edges:
        normalize_evidence(edge.get('evidence', []))
    edges.sort(key=canonical)


def normalize_evidence(records):
    for record in records:
        record.get('supporting_symbols', []).sort()
    records.sort(key=canonical)


def decode_visibility(node):
    for field in ('visibility', 'container_visibility'):
        value = node.get(field)
        if isinstance(value, str):
            node[field] = json.loads(value)


def fingerprint_facts(item):
    return {"path": item["path"], "size": item["size"], "hash": item["hash"],
            "missing": item.get("missing", False), "excluded_children": item.get("excluded_children", [])}


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def digest(snapshot):
    return hashlib.sha256(canonical(snapshot).encode()).hexdigest()


def volatile_generated_hashes(caches):
    snapshots = [graph_snapshot(cache) for cache in caches]
    if len(snapshots) < 2:
        return set()
    generated = set.intersection(*({node["file"] for node in snapshot["nodes"] if node.get("is_generated")}
                                   for snapshot in snapshots))
    records = [{item["path"]: item for item in snapshot["fingerprints"]} for snapshot in snapshots]
    return {path for path in generated if hash_varies_only(path, records)}


def hash_varies_only(path, records):
    if not all(path in record for record in records):
        return False
    facts = [record[path] for record in records]
    hashes = {item["hash"] for item in facts}
    structural = {canonical({key: value for key, value in item.items() if key != "hash"}) for item in facts}
    return len(hashes) > 1 and len(structural) == 1


def compare(reference, current, volatile_hashes=()):
    old, new = graph_snapshot(reference), graph_snapshot(current)
    volatile_hashes = set(volatile_hashes)
    changes = volatile_changes(old, new, volatile_hashes)
    for snapshot in (old, new):
        for item in snapshot["fingerprints"]:
            if item["path"] in volatile_hashes:
                item["hash"] = "baseline-proven-volatile-generated-content"
        snapshot["fingerprints"].sort(key=canonical)
    differences = [key for key in old if old[key] != new[key]]
    return {"equal": not differences, "different_fields": differences,
            "baseline_sha256": digest(old), "current_sha256": digest(new),
            "volatile_generated_hash_changes": changes,
            "nodes": len(new["nodes"]), "edges": len(new["edges"])}


def volatile_changes(old, new, allowed):
    before = {item["path"]: item["hash"] for item in old["fingerprints"]}
    return sorted({item["path"] for item in new["fingerprints"]
                   if item["path"] in allowed and before.get(item["path"]) != item["hash"]})
