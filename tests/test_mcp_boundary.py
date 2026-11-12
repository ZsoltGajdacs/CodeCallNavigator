"""Reject forbidden MCP dependency and source coupling before packaging."""
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from mcp_boundary import validate


class McpBoundary(unittest.TestCase):
    def metadata(self, root, forbidden):
        names = ['codecallnavigator-mcp', 'adapter', forbidden]
        packages = [dict(id=name, name=name, manifest_path=str(root / name / 'Cargo.toml'))
                    for name in names]
        nodes = [dict(id=names[0], dependencies=[names[1]]),
                 dict(id=names[1], dependencies=[names[2]]),
                 dict(id=names[2], dependencies=[])]
        return dict(packages=packages, resolve=dict(nodes=nodes))

    def test_indirect_test_dependency_on_engine_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaisesRegex(ValueError, 'codecallnavigator-engine'):
                validate(self.metadata(root, 'codecallnavigator-engine'), root)

    def test_runtime_file_reads_and_source_inclusion_are_rejected(self):
        for source in ['use std::fs; fn main() {}', 'use std::{fs, io};',
                       'include!("../../engine.rs");']:
            with self.subTest(source=source), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                path = root / 'codecallnavigator-mcp/src/main.rs'
                path.parent.mkdir(parents=True)
                path.write_text(source)
                with self.assertRaisesRegex(ValueError, 'source boundary'):
                    validate(self.metadata(root, 'serde'), root)

    def test_independent_process_only_package_is_accepted(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / 'codecallnavigator-mcp/src/main.rs'
            path.parent.mkdir(parents=True)
            path.write_text('use tokio::process::Command; fn main() {}')
            self.assertEqual(len(validate(self.metadata(root, 'serde'), root)), 3)

    def test_test_helpers_cannot_include_shared_library_sources(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / 'codecallnavigator-mcp/tests/acceptance.rs'
            path.parent.mkdir(parents=True)
            path.write_text('#[path="../../core/src/lib.rs"] mod engine;')
            with self.assertRaisesRegex(ValueError, 'source boundary'):
                validate(self.metadata(root, 'serde'), root)
            path.write_text('use std::fs; fn fixtures() {}')
            self.assertEqual(len(validate(self.metadata(root, 'serde'), root)), 3)
