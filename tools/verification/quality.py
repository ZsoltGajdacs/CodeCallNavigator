#!/usr/bin/env python3
"""Run changed-source CodeScene reviews and the repository safeguard over stdio MCP."""
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
import subprocess

if __package__:
    from .mcp_stdio import McpClient, QualityError
else:
    from mcp_stdio import McpClient, QualityError

SOURCE_SUFFIXES = frozenset({
    '.rs', '.py', '.js', '.jsx', '.ts', '.tsx', '.java', '.cs', '.go', '.rb',
    '.swift', '.kt', '.kts', '.scala', '.php', '.c', '.cc', '.cpp', '.cxx', '.h', '.hpp',
})


def tool_document(result, required_keys):
    """Some server versions prefix JSON tool text with a version-update banner."""
    structured = result.get('structuredContent')
    if is_document(structured, required_keys):
        return structured
    for text in tool_texts(result):
        document = embedded_document(text, required_keys)
        if document is not None:
            return document
    raise QualityError('CodeScene tool response omitted the expected structured result')


def tool_texts(result):
    content = result.get('content', [])
    if not isinstance(content, list):
        raise QualityError('CodeScene tool response contained invalid content blocks')
    for block in content:
        if not isinstance(block, dict):
            continue
        if block.get('type') == 'text' and isinstance(block.get('text'), str):
            yield block['text']


def embedded_document(text, required_keys):
    decoder = json.JSONDecoder()
    for index, character in enumerate(text):
        if character != '{':
            continue
        try:
            value, _ = decoder.raw_decode(text[index:])
        except ValueError:
            continue
        if is_document(value, required_keys):
            return value
    return None


def is_document(value, required_keys):
    return isinstance(value, dict) and required_keys <= value.keys()


def source_files(root, requested):
    files, skipped = [], []
    for name in requested:
        path = review_path(root, name)
        relative = str(path.relative_to(root))
        if path.is_file() and path.suffix.lower() in SOURCE_SUFFIXES:
            files.append(relative)
        else:
            skipped.append(relative)
    return sorted(set(files)), sorted(set(skipped))


def review_path(root, name):
    path = (root / name).resolve()
    if not path.is_relative_to(root):
        raise QualityError('Review paths must remain inside the repository')
    return path


def tracked_count(root, files):
    """The safeguard covers local edits; committed base diffs receive source review only."""
    if not files:
        return 0
    result = subprocess.run(
        ['git', 'diff', '--name-only', '-z', 'HEAD', '--', *files], cwd=root, check=True,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
    )
    return len(set(filter(None, result.stdout.split(b'\0'))))


def require(condition, message):
    """Report invalid CodeScene evidence through the same recoverable failure path."""
    if not condition:
        raise QualityError(message)


def validate_review(document):
    validate_score(document['score'])
    require(isinstance(document['review'], list), 'CodeScene review omitted its findings list')


def validate_score(score):
    require(type(score) in (int, float), 'CodeScene review omitted a numeric score')
    require(math.isfinite(score), 'CodeScene review returned a nonfinite score')
    require(0 <= score <= 10, 'CodeScene review returned a score outside its valid range')


def coverage_counts(document):
    metadata = document.get('metadata')
    require(isinstance(metadata, dict), 'CodeScene safeguard omitted coverage metadata')
    status = metadata.get('status')
    require(isinstance(status, str) and bool(status), 'CodeScene safeguard omitted its coverage status')
    return tuple(coverage_count(metadata, key) for key in (
        'checked-file-count', 'code-health-eligible-file-count', 'total-modified-file-count',
    ))


def coverage_count(metadata, key):
    value = metadata.get(key)
    require(type(value) is int, 'CodeScene safeguard returned invalid coverage counts')
    require(value >= 0, 'CodeScene safeguard returned negative coverage counts')
    return value


def validate_coverage(counts, expected_tracked):
    checked, eligible, total = counts
    require(checked == eligible, 'CodeScene safeguard did not check every eligible file')
    require(checked >= expected_tracked, 'CodeScene safeguard did not cover the modified tracked source files')
    require(total >= eligible, 'CodeScene safeguard returned inconsistent coverage counts')


def validate_verdicts(document):
    results = document.get('results')
    require(isinstance(results, list), 'CodeScene safeguard omitted its results list')
    for result in results:
        require(isinstance(result, dict), 'CodeScene safeguard returned an invalid result')
        require(result.get('verdict') in ('stable', 'improved'),
                'CodeScene safeguard reported a regression or unknown verdict')


def validate_safeguard(document, expected_tracked):
    validate_coverage(coverage_counts(document), expected_tracked)
    require(document.get('quality_gates') == 'passed', 'CodeScene safeguard quality gate failed')
    validate_verdicts(document)


def run_checks(root, requested, command='cs-mcp', timeout=120):
    report = {'status': 'failed', 'reviews': [], 'safeguard': None, 'errors': []}
    try:
        root = Path(root).resolve(strict=True)
        files, report['skipped_files'] = source_files(root, requested)
        expected_tracked = tracked_count(root, files)
        with McpClient(command, root, timeout) as client:
            review_files(client, root, files, report['reviews'])
            report['safeguard'] = read_safeguard(client, root)
            validate_safeguard(report['safeguard'], expected_tracked)
        if any(review['review'] for review in report['reviews']):
            raise QualityError('CodeScene source reviews reported actionable findings')
        report['status'] = 'passed'
    except (QualityError, OSError, subprocess.SubprocessError) as error:
        report['errors'].append(str(error))
    return report


def review_files(client, root, files, reviews):
    for file in files:
        document = review_file(client, root, file)
        reviews.append({**document, 'path': file})


def review_file(client, root, file):
    try:
        document = tool_document(client.tool('code_health_review', {
            'file_path': str(root / file),
        }), {'score', 'review'})
        validate_review(document)
        return document
    except (QualityError, OSError) as error:
        raise QualityError(f'CodeScene review for {file}: {error}') from error


def read_safeguard(client, root):
    document = tool_document(client.tool('pre_commit_code_health_safeguard', {
        'git_repository_path': str(root),
    }), {'quality_gates', 'metadata', 'results'})
    return document


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--files', nargs='*', default=[])
    parser.add_argument('--timeout', type=float, default=120)
    parser.add_argument('--command', default='cs-mcp', help='CodeScene MCP executable')
    arguments = parser.parse_args()
    if not math.isfinite(arguments.timeout) or arguments.timeout <= 0:
        parser.error('--timeout must be positive')
    report = run_checks(arguments.root, arguments.files, arguments.command, arguments.timeout)
    print(json.dumps(report, indent=2, allow_nan=False))
    return 0 if report['status'] == 'passed' else 1


if __name__ == '__main__':
    raise SystemExit(main())
