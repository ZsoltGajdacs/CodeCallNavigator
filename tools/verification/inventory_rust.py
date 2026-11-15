"""Rust lexical masking and exact module paths for source test registration."""
from dataclasses import dataclass, field
import re
import sys

RAW_LITERAL = re.compile(r'(?:br|r)(\#*)"')
CHAR_LITERAL = re.compile(r"'(?:\\(?:u\{[0-9A-Fa-f_]+\}|x[0-9A-Fa-f]{2}|.)|[^'\\\n])'")
ITEM = re.compile(r'#\s*\[|\bmod\s+(\w+)\s*([;{])|\bfn\s+(\w+)|[{};]')


def block_end(source, start):
    depth, end = 1, start + 2
    while depth and end < len(source):
        pair = source[end:end + 2]
        if pair == '/*':
            depth, end = depth + 1, end + 2
        elif pair == '*/':
            depth, end = depth - 1, end + 2
        else:
            end += 1
    return end


def quoted_end(source, start):
    end = start + 1
    while end < len(source):
        if source[end] == '\\':
            end += 2
        elif source[end] == '"':
            return end + 1
        else:
            end += 1
    return end


def literal_end(source, position):
    if source.startswith('//', position):
        end = source.find('\n', position)
        return len(source) if end < 0 else end
    if source.startswith('/*', position):
        return block_end(source, position)
    raw = RAW_LITERAL.match(source, position)
    if raw:
        marker = '"' + raw[1]
        end = source.find(marker, raw.end())
        return len(source) if end < 0 else end + len(marker)
    if source[position] == '"':
        return quoted_end(source, position)
    character = CHAR_LITERAL.match(source, position)
    return character.end() if character else None


def blank_literal(text):
    return ''.join('\n' if character == '\n' else ' ' for character in text)


def rust_mask(source):
    """Preserve source offsets and delimiters while hiding literals/comments."""
    output, position, copied = [], 0, 0
    while position < len(source):
        end = literal_end(source, position)
        if end is None:
            position += 1
            continue
        output.extend((source[copied:position], blank_literal(source[position:end])))
        position = copied = end
    output.append(source[copied:])
    return ''.join(output)


def closing(mask, start, opening='{', close='}'):
    depth = 1
    for index in range(start + 1, len(mask)):
        depth += (mask[index] == opening) - (mask[index] == close)
        if depth == 0:
            return index
    raise ValueError('Unbalanced Rust source structure')


def enabled(attributes):
    if re.search(r'cfg\s*\(\s*unix\s*\)', attributes) and sys.platform == 'win32':
        return False
    operating_system = re.search(r'cfg\s*\(\s*target_os\s*=\s*"([^"]+)"\s*\)', attributes)
    actual = {'darwin': 'macos', 'win32': 'windows'}.get(sys.platform, sys.platform)
    return not operating_system or operating_system[1] == actual


@dataclass
class RustModule:
    root: object
    path: object
    target: dict
    modules: tuple = ()
    trail: tuple = ()
    source: str | None = None
    directory: object = None
    position: int = 0
    attributes: list = field(default_factory=list)

    def load(self):
        if self.source is not None:
            return
        self.path = self.path.resolve()
        if self.path in self.trail:
            raise ValueError(f'Recursive Rust module: {self.path}')
        self.trail = (*self.trail, self.path)
        self.source = self.path.read_text()
        self.directory = self.path.parent if not self.modules or self.path.name == 'mod.rs' else self.path.with_suffix('')

    def scan(self):
        self.load()
        self.mask = rust_mask(self.source)
        while match := ITEM.search(self.mask, self.position):
            self.position = match.end()
            yield from self.visit(match)

    def take_attributes(self):
        attributes = '\n'.join(self.attributes)
        self.attributes = []
        return attributes

    def visit(self, match):
        if match[0].startswith('#'):
            self.read_attribute(match)
        elif match[1]:
            yield from self.read_module(match)
        elif match[3]:
            yield from self.read_function(match[3])
        else:
            if match[0] == '{':
                self.position = closing(self.mask, match.start()) + 1
            self.attributes = []

    def read_attribute(self, match):
        end = closing(self.mask, match.end() - 1, '[', ']')
        self.attributes.append(self.source[match.start():end + 1])
        self.position = end + 1

    def read_module(self, match):
        name, form = match[1], match[2]
        attributes = self.take_attributes()
        if form == '{':
            start = self.position
            end = closing(self.mask, start - 1)
            self.position = end + 1
            if enabled(attributes):
                child = RustModule(self.root, self.path, self.target, (*self.modules, name),
                                   self.trail, self.source[start:end], self.directory / name)
                yield from child.scan()
        elif enabled(attributes):
            child = RustModule(self.root, self.module_path(name, attributes), self.target,
                               (*self.modules, name), self.trail)
            yield from child.scan()

    def module_path(self, name, attributes):
        override = re.search(r'path\s*=\s*"([^"]+)"', attributes)
        if override:
            return self.path.parent / override[1]
        alternatives = [self.directory / (name + '.rs'), self.directory / name / 'mod.rs']
        found = [item for item in alternatives if item.is_file()]
        if len(found) != 1:
            raise ValueError(f'Cannot resolve Rust module {self.path}: {name}')
        return found[0]

    def read_function(self, name):
        attributes = self.take_attributes()
        if not re.search(r'#\[\s*(?:tokio::)?test\s*\]', attributes):
            return
        if not enabled(attributes):
            return
        test_name = '::'.join((*self.modules, name))
        identity = f'{self.target["package"]}/{self.target["target_kind"]}:{self.target["target"]}::{test_name}'
        yield dict(id=identity, runner='rust', file=str(self.path.relative_to(self.root)),
                   test_name=test_name, ignored=bool(re.search(r'#\[\s*ignore\b', attributes)), **self.target)
