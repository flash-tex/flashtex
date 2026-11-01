#!/usr/bin/env python3
"""Validate corpus integrity; optionally emit one runtime-v1 compile request.

This does not execute a compiler or establish feature support.
"""
import argparse
import json
from pathlib import Path, PurePosixPath
import sys

ROOT = Path(__file__).resolve().parent
KINDS = {'visible_text', 'source_navigation', 'math_layout', 'diagnostic',
         'recovered_text', 'required_feature', 'diagram_layout'}


def relative_path(value):
    if not isinstance(value, str) or not value or '\\' in value:
        raise ValueError('path must be a nonempty POSIX relative path')
    path = PurePosixPath(value)
    if path.is_absolute() or '..' in path.parts or str(path) != value:
        raise ValueError('unsafe or noncanonical path: ' + value)
    return path


def validate(root=ROOT):
    root = Path(root).resolve()
    manifest = json.loads((root / 'manifest.json').read_text(encoding='utf-8'))
    if (manifest.get('schema_version'), manifest.get('contract'),
            manifest.get('offset_unit'), manifest.get('range_end')) != (
            1, 'runtime-v1', 'utf8_bytes', 'exclusive'):
        raise ValueError('unexpected manifest version or source convention')
    if not isinstance(manifest.get('cases'), list) or not manifest['cases']:
        raise ValueError('cases must be a nonempty list')
    ids, covered, count = set(), set(), 0
    for case in manifest['cases']:
        ident = case['id']
        if (not isinstance(ident, str) or not ident or
                any(c not in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in ident)
                or ident in ids):
            raise ValueError('invalid/duplicate case id: ' + repr(ident))
        ids.add(ident)
        if case['support'] != 'required_unverified':
            raise ValueError(ident + ': engine support needs separate evidence')
        for field in ('description', 'profile'):
            if not isinstance(case[field], str) or not case[field].strip():
                raise ValueError(ident + ': missing ' + field)
        if not isinstance(case['features'], list) or not case['features'] or not all(
                isinstance(f, str) and f for f in case['features']):
            raise ValueError(ident + ': invalid features')
        covered.update(case['features'])
        docs = case['documents']
        if not isinstance(docs, list) or not docs or len(set(docs)) != len(docs):
            raise ValueError(ident + ': invalid/duplicate documents')
        sources = {}
        for name in docs:
            relative_path(name)
            path = (root / 'cases' / ident / name).resolve()
            if not path.is_relative_to(root / 'cases' / ident):
                raise ValueError(ident + ': source escapes case directory')
            sources[name] = path.read_bytes()
            sources[name].decode('utf-8')
        if case['entry_path'] not in sources:
            raise ValueError(ident + ': entry document missing')
        if not isinstance(case['expectations'], list) or not case['expectations']:
            raise ValueError(ident + ': missing expectations')
        for expectation in case['expectations']:
            if expectation['kind'] not in KINDS or not expectation['detail'].strip():
                raise ValueError(ident + ': invalid expectation')
            source = expectation['source']
            data = sources[source['path']]
            start, end = source['start_byte'], source['end_byte']
            if (type(start) is not int or type(end) is not int or
                    not 0 <= start < end <= len(data)):
                raise ValueError(ident + ': invalid source range')
            # Each boundary must independently decode; no split multibyte codepoint.
            data[:start].decode('utf-8')
            data[:end].decode('utf-8')
            if data[start:end].decode('utf-8') != source['text']:
                raise ValueError(ident + ': source witness differs from fixture')
            count += 1
    required = {'macros', 'scope', 'unicode', 'math', 'includes', 'recovery', 'source-mapping'}
    if not required <= covered:
        raise ValueError('required feature coverage missing: ' + repr(required - covered))
    declared = {str(Path('cases') / c['id'] / d) for c in manifest['cases'] for d in c['documents']}
    actual = {str(p.relative_to(root)) for p in (root / 'cases').rglob('*.tex')}
    if declared != actual:
        raise ValueError('undeclared or missing TeX fixtures: ' + repr(declared ^ actual))
    return manifest, count


def compile_request(case, revision, root=ROOT):
    return {'protocol_version': 1, 'id': 'corpus-' + case['id'], 'type': 'compile',
            'payload': {'project_id': 'corpus-' + case['id'], 'revision': revision,
                        'entry_path': case['entry_path'], 'documents': [
                            {'path': name, 'text': (root / 'cases' / case['id'] / name).read_text(encoding='utf-8')}
                            for name in case['documents']]}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--emit-request', metavar='CASE_ID')
    parser.add_argument('--revision', type=int, default=1)
    args = parser.parse_args()
    try:
        manifest, count = validate()
        if args.revision < 0:
            raise ValueError('revision must be nonnegative')
        if args.emit_request:
            case = next((c for c in manifest['cases'] if c['id'] == args.emit_request), None)
            if case is None:
                raise ValueError('unknown case: ' + args.emit_request)
            print(json.dumps(compile_request(case, args.revision), ensure_ascii=False))
        else:
            print(f"Validated {len(manifest['cases'])} cases and {count} UTF-8 source witnesses; compiler behavior NOT tested.")
    except (KeyError, TypeError, ValueError, OSError, UnicodeError) as error:
        print('Corpus validation failed: ' + str(error), file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
