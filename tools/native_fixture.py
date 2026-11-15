"""Owned native acceptance fixtures and durable failure diagnostics."""
from contextlib import suppress
import json
import os
from pathlib import Path
import shutil
import tempfile

from verification.cleanup import check_usage, require, safe_path, tree_bytes

TOKEN = 'native-fixture.json'
RETAINED = 'retained-fixture.json'


def write_record(path, value):
    pending = None
    try:
        with tempfile.NamedTemporaryFile(mode='w', encoding='utf-8', prefix=path.name + '.',
                                         suffix='.tmp', dir=path.parent, delete=False) as stream:
            pending = Path(stream.name)
            stream.write(json.dumps(value, indent=2) + '\n')
        pending.replace(path)
    finally:
        if pending is not None:
            # Remove only this call's unique sibling; unrelated interrupted
            # writes remain diagnostics. Preserve the original write failure.
            with suppress(OSError):
                pending.unlink(missing_ok=True)


def fixture_record(path, output, category, completed=False):
    return dict(version=1, path=str(path), gate_output=str(output),
                category_output=str(category) if category is not None else None,
                completed=completed)


def category_owner(temporary, output):
    from verification.category_cleanup import MARKER, external_action, owned_category, temporary_path
    marker = safe_path(temporary, temporary / MARKER)
    if not marker.exists():
        return None
    value = json.loads(marker.read_text())
    root, category, kind = owned_category(value['root'], value['output'])
    require(kind == 'acceptance', 'Native fixtures require an acceptance category')
    external_action(root, category, kind)
    require(temporary == temporary_path(root, category), 'Native fixture TMPDIR has another owner')
    require(output.parent == category, 'Native gate output must belong directly to its category')
    safe_path(category, output)
    return category


class NativeFixture:
    """Category cleanup owns deletion; standalone successes require proven idle use."""
    def __init__(self, output, prefix, temporary_root=None):
        self.output, self.prefix = Path(output).resolve(), prefix
        current = temporary_root or os.environ.get('TMPDIR') or tempfile.gettempdir()
        self.temporary = Path(current).resolve(strict=True)
        self.category = category_owner(self.temporary, self.output)
        self.path = None

    def __enter__(self):
        self.output.mkdir(parents=True, exist_ok=True)
        require(not (self.output / RETAINED).exists(), 'Use new native evidence; retained diagnostics exist')
        self.path = Path(tempfile.mkdtemp(prefix=self.prefix, dir=self.temporary)).resolve()
        write_record(self.path / TOKEN, fixture_record(self.path, self.output, self.category))
        return self.path

    def __exit__(self, error_type, error, traceback):
        if error_type is not None:
            self.retain('Failed or inconclusive native acceptance; owned diagnostics retained')
            return False
        write_record(self.path / TOKEN, fixture_record(self.path, self.output, self.category, True))
        if self.category is not None:
            return False
        usage = check_usage(self.path)
        if usage.status != 'idle':
            self.retain(usage.reason or 'Native fixture inactivity could not be established')
            raise RuntimeError(f'Native fixture cleanup unavailable; retained {self.path}')
        shutil.rmtree(self.path)
        return False

    def retain(self, reason):
        value = fixture_record(self.path, self.output, self.category)
        write_record(self.output / RETAINED, value | {'reason': reason})


def fixture_binding(path, output, temporary):
    path = safe_path(temporary, path)
    token = safe_path(path, path / TOKEN)
    require(token.is_file(), f'Native fixture ownership marker is missing: {path}')
    value = json.loads(token.read_text())
    require(isinstance(value, dict) and isinstance(value.get('completed'), bool),
            f'Invalid native fixture ownership: {path}')
    gate = safe_path(output, Path(value.get('gate_output', '')))
    require(gate.parent == output, f'Native fixture belongs to another gate: {path}')
    require(value == fixture_record(path, gate, output, value['completed']),
            f'Native fixture ownership does not match its category: {path}')
    return gate, value


def retained_row(path, reason):
    size = tree_bytes(path)
    return dict(path=str(path), operation='retain-native-fixture', status='retained',
                reason=reason, estimated_bytes_before=size, estimated_bytes_after=size)


def retain_fixture(path, output, temporary, checker):
    gate, value = fixture_binding(path, output, temporary)
    marker = safe_path(output, gate / RETAINED)
    if value['completed'] and not marker.exists():
        return None
    reason = 'Native acceptance interrupted before fixture completion'
    if marker.exists():
        recorded = json.loads(marker.read_text())
        expected = fixture_record(path, gate, output)
        require(all(recorded.get(key) == item for key, item in expected.items()),
                f'Retained native fixture marker has another binding: {marker}')
        reason = recorded.get('reason', reason)
    usage = checker(path)
    require(usage.status == 'idle', f'Cannot retain active or unverified native fixture {path}: {usage.reason}')
    destination = safe_path(output, gate / 'retained-fixture')
    require(not destination.exists(), f'Native retained evidence cannot be overwritten: {destination}')
    shutil.move(str(path), str(destination))
    write_record(destination / TOKEN, fixture_record(destination, gate, output))
    write_record(marker, fixture_record(destination, gate, output) | {'reason': reason})
    return retained_row(destination, reason)


def validate_retained_marker(marker, output, temporary):
    marker = safe_path(output, marker)
    value = json.loads(marker.read_text())
    require(isinstance(value, dict), f'Invalid retained native marker: {marker}')
    path = Path(value.get('path', ''))
    gate = marker.parent
    require(value.get('gate_output') == str(gate) and value.get('category_output') == str(output),
            f'Retained native marker belongs to another gate: {marker}')
    if path == gate / 'retained-fixture':
        owner, _ = fixture_binding(path, output, output)
        require(owner == gate, f'Retained native fixture belongs to another gate: {marker}')
        return
    safe_path(temporary, path)
    require(path.parent == temporary, f'Retained native fixture is outside owned TMPDIR: {path}')
    owner, _ = fixture_binding(path, output, temporary)
    require(owner == gate, f'Retained native fixture belongs to another gate: {marker}')


def retain_native_fixtures(root, output, checker):
    from verification.category_cleanup import temporary_path
    temporary = temporary_path(root, output)
    rows = []
    try:
        if not temporary.exists():
            return rows
        for marker in output.glob('gate-*/' + RETAINED):
            validate_retained_marker(marker, output, temporary)
        for token in sorted(temporary.glob('*/' + TOKEN)):
            row = retain_fixture(token.parent, output, temporary, checker)
            if row is not None:
                rows.append(row)
    except (OSError, RuntimeError, ValueError, KeyError, TypeError) as error:
        rows.append(dict(path=str(temporary), operation='retain-native-fixture',
                         status='failed', reason=str(error),
                         estimated_bytes_before=None, estimated_bytes_after=None))
    return rows
