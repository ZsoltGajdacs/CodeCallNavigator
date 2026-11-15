"""Discover source registrations without importing or executing test modules.

Rust tests retain their executable target ownership. Compiled libtest lists are
checked separately before execution; Python preserves concrete inherited cases.
"""
from pathlib import Path

from .inventory_node import node_tests
from .inventory_python import python_tests
from .inventory_rust import RustModule, rust_mask, closing
from .inventory_targets import rust_targets


def discover_tests(root):
    root = Path(root).resolve()
    rows = [row for path, target in rust_targets(root) for row in RustModule(root, path, target).scan()]
    rows.extend(python_tests(root))
    rows.extend(node_tests(root))
    identities = [row['id'] for row in rows]
    if len(identities) != len(set(identities)):
        raise ValueError('Duplicate test registration discovered')
    return sorted(rows, key=lambda row: row['id'])
