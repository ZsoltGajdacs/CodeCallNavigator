"""Archived compiler parity must not create fixtures outside the owned work tree."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
from csharp_flow_acceptance import materialize_baseline_source, rebase_expected
from csharp_flow_ingestion_acceptance import prepare_fixture


def ingestion_fixtures():
    return json.loads((ROOT / 'fixtures/flow/csharp-baselines/pre-ingestion.json').read_text())['fixtures']


def export_fixture():
    return json.loads((ROOT / 'fixtures/flow/csharp-baselines/pre-export-baseline.json').read_text())['fixture']


class ExpectedPathRebasingTests(unittest.TestCase):
    def test_expected_projection_rebases_only_complete_path_prefixes(self):
        before = {'file': '/recorded/root/Probe.cs', 'id': '/recorded/root/Fixture.csproj|Run',
                  'roots': ['/recorded/root', '/recorded/root-other/Probe.cs'],
                  'source': 'text containing /recorded/root/Probe.cs', 'count': 3}
        original = copy.deepcopy(before)
        actual = rebase_expected(before, Path('/recorded/root'), Path('/owned/root'))
        self.assertEqual(actual, {'file': '/owned/root/Probe.cs', 'id': '/owned/root/Fixture.csproj|Run',
                                 'roots': ['/owned/root', '/recorded/root-other/Probe.cs'],
                                 'source': 'text containing /recorded/root/Probe.cs', 'count': 3})
        self.assertEqual(before, original)

    def test_recorded_canonical_and_historical_artifacts_remain_exact(self):
        for fixture in ingestion_fixtures():
            original = copy.deepcopy(fixture)
            old, new = Path(fixture['root']), Path('/owned/work') / fixture['name']
            native = rebase_expected(fixture['native_projection'], old, new)
            self.assertNotEqual(native, fixture['native_projection'])
            self.assertEqual(rebase_expected(fixture['canonical_projection'], old, new), fixture['canonical_projection'])
            self.assertEqual(rebase_expected(fixture['artifact'], old, new), fixture['artifact'])
            self.assertEqual(fixture, original)


class SourceFixtureOwnershipTests(unittest.TestCase):
    def test_ingestion_materializes_all_encodings_under_owned_work(self):
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory).resolve()
            for fixture in ingestion_fixtures():
                original = copy.deepcopy(fixture)
                paths = prepare_fixture(fixture, work)
                self.assertEqual(paths.root, work / fixture['name'])
                self.assertEqual(paths.source.read_bytes(), fixture['source'].encode(fixture['source_encoding']))
                self.assertEqual(hashlib.sha256(paths.source.read_bytes()).hexdigest(), fixture['source_sha256'])
                self.assertEqual(paths.project.read_text(), fixture['project'])
                self.assertEqual(fixture, original)

    def test_raw_flow_source_is_owned_and_recorded_snapshot_is_unchanged(self):
        fixture = export_fixture()
        original = copy.deepcopy(fixture)
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory).resolve()
            source = materialize_baseline_source(fixture, work)
            self.assertEqual(source, work / 'task22-flow-baseline-source' / fixture['source_file'])
            self.assertEqual(source.read_bytes(), fixture['source'].encode('utf-8'))
            self.assertEqual(fixture, original)

    def test_symlinked_source_is_rejected_without_changing_external_bytes(self):
        fixture = export_fixture()
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory).resolve()
            external = work / 'external.cs'
            external.write_bytes(b'unchanged external source')
            folder = work / 'task22-flow-baseline-source'
            folder.mkdir()
            (folder / fixture['source_file']).symlink_to(external)
            with self.assertRaisesRegex(ValueError, 'symlink'):
                materialize_baseline_source(fixture, work)
            self.assertEqual(external.read_bytes(), b'unchanged external source')
            escaped = ingestion_fixtures()[0]
            escaped['project_file'] = '../escaped.csproj'
            with self.assertRaisesRegex(ValueError, 'inside the owned fixture'):
                prepare_fixture(escaped, work)
            self.assertFalse((work / 'escaped.csproj').exists())
