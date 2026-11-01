#!/usr/bin/env python3
"""Run original compiler JSONLines requests; preserve evidence, never infer full compatibility."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import subprocess
import sys
import time

SPEC = importlib.util.spec_from_file_location('corpus_validate', Path(__file__).with_name('validate.py'))
corpus = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(corpus)

# Narrow literal assertions supplement, never replace, prose/layout expectations.
TEXT = {
    'plain-paragraphs': ['First paragraph.', 'Second paragraph.'],
    'macro-arguments': ['left then right'], 'macro-scope': ['outer inner outer'],
    'unicode-literals': ['Café naïve — 東京.', 'After Unicode.'],
    'included-file': ['Before.', 'Included café.', 'After.'],
    'include-scope': ['inner outer'],
    'comments-escapes': ['AB', 'Price $5; 50%; A&B; C_D; #1.'],
    'unknown-command': ['Before.', 'After.'], 'unclosed-group': ['Before.', 'inside', 'Still inside.'],
    'extra-closing-group': ['Before.', 'After.'], 'missing-include': ['Before.', 'After.'],
    'literal-source-map': ['éé', 'UniqueAnchor', '尾'],
}


def check(label, passed, detail):
    return {'check': label, 'status': 'pass' if passed else 'fail', 'detail': detail}


def source_valid(source, documents):
    if not isinstance(source, dict) or source.get('path') not in documents:
        return False
    start, end = source.get('start_byte'), source.get('end_byte')
    data = documents[source['path']].encode('utf-8')
    if type(start) is not int or type(end) is not int or not 0 <= start <= end <= len(data):
        return False
    try:
        data[:start].decode('utf-8'); data[:end].decode('utf-8')
    except UnicodeError:
        return False
    return True


def evaluate(case, request, reply):
    checks = []
    p = reply.get('payload') if isinstance(reply, dict) else None
    envelope = (isinstance(reply, dict) and reply.get('protocol_version') == 1
                and reply.get('id') == request['id'] and reply.get('type') == 'compile_result'
                and isinstance(p, dict))
    checks.append(check('result_envelope', envelope, 'One correlated runtime-v1 compile_result is required.'))
    if not envelope:
        return checks
    correlated = p.get('project_id') == request['payload']['project_id'] and p.get('revision') == request['payload']['revision']
    checks.append(check('project_revision', correlated, 'Result must describe the exact submitted revision.'))
    pages, diags = p.get('pages'), p.get('diagnostics')
    structure = (p.get('status') in ('ok', 'recovered', 'failed') and isinstance(pages, list)
                 and isinstance(diags, list) and all(isinstance(d, dict) for d in diags))
    checks.append(check('result_structure', structure, 'Status, pages and diagnostics must follow runtime-v1.'))
    if not structure:
        return checks
    documents = {d['path']: d['text'] for d in request['payload']['documents']}
    items = []
    valid_pages = True
    for page in pages:
        if not isinstance(page, dict) or not isinstance(page.get('items'), list):
            valid_pages = False; continue
        dimensions = [page.get('width_pt'), page.get('height_pt')]
        valid_pages &= all(type(n) in (int, float) and math.isfinite(n) and n > 0 for n in dimensions)
        items.extend(page['items'])
    valid_items = all(isinstance(i, dict) and i.get('kind') == 'text'
                      and isinstance(i.get('text'), str) and source_valid(i.get('source'), documents)
                      and all(type(i.get(k)) in (int, float) and math.isfinite(i[k])
                              for k in ('x_pt', 'baseline_y_pt', 'font_size_pt')) for i in items)
    checks.append(check('display_items_and_sources', valid_pages and valid_items,
                        'v1 text positions must be finite and source spans valid UTF-8 ranges into submitted files.'))
    valid_diags = all(d.get('severity') in ('error', 'warning') and isinstance(d.get('message'), str)
                      and (d.get('source') is None or source_valid(d['source'], documents))
                      and (d.get('recovery') is None or isinstance(d['recovery'], str)) for d in diags)
    checks.append(check('diagnostic_sources', valid_diags, 'Diagnostic source ranges must refer to this input revision.'))
    if not valid_items:
        return checks
    text = ' '.join(' '.join(i['text'] for i in items).split())
    if case['id'] in TEXT:
        remaining = text
        found = True
        for expected in TEXT[case['id']]:
            pos = remaining.find(expected)
            if pos < 0:
                found = False; break
            remaining = remaining[pos + len(expected):]
        checks.append(check('required_text_in_order', found, {'required': TEXT[case['id']], 'observed': text}))
    unsupported = [d for d in diags if any(s in d.get('message', '').lower() for s in ('unsupported', 'not supported', 'only the entry document'))]
    if unsupported:
        checks.append({'check': 'unsupported_constructs', 'status': 'unsupported',
                       'detail': 'Compiler explicitly reported unsupported input; this cannot establish case compatibility.',
                       'diagnostics': unsupported})
    for index, expectation in enumerate(case['expectations']):
        kind, witness = expectation['kind'], expectation['source']
        label = f'expectation_{index}_{kind}'
        if kind == 'source_navigation':
            matching = [i for i in items if i['source']['path'] == witness['path']
                        and witness['start_byte'] <= i['source']['start_byte']
                        and i['source']['end_byte'] <= witness['end_byte']]
            # Literal anchors may be split into words; verify exact outer byte boundaries.
            passed = (bool(matching) and matching[0]['source']['start_byte'] == witness['start_byte']
                      and matching[-1]['source']['end_byte'] == witness['end_byte']
                      and ' '.join(' '.join(i['text'] for i in matching).split()) == ' '.join(witness['text'].split()))
            checks.append(check(label, passed, 'Literal display items retain the witness file and exclusive byte boundaries; native click behavior still unverified.'))
        elif kind == 'diagnostic':
            matches = [d for d in diags if d.get('severity') == 'error' and d.get('recovery')
                       and isinstance(d.get('source'), dict) and d['source'].get('path') == witness['path']
                       and type(d['source'].get('start_byte')) is int and type(d['source'].get('end_byte')) is int
                       and d['source']['start_byte'] < witness['end_byte'] and d['source']['end_byte'] > witness['start_byte']]
            if case['id'] == 'unclosed-group' and not matches:
                checks.append({'check': label, 'status': 'unverified', 'detail': 'EOF/related-location recovery needs manual inspection; retained raw diagnostics.'})
            else:
                checks.append(check(label, bool(matches), 'Error with recovery explanation must overlap the offending source witness.'))
        elif kind == 'recovered_text':
            checks.append(check(label, p['status'] == 'recovered' and bool(items), 'Usable provisional output must have recovered status.'))
        else:
            checks.append({'check': label, 'status': 'unsupported' if unsupported else 'unverified',
                           'detail': expectation['detail'], 'unsupported_diagnostics': unsupported})
    return checks


def run_case(case, command, timeout=10, root=corpus.ROOT):
    request = corpus.compile_request(case, 1, root)
    result = {'case_id': case['id'], 'request': request, 'checks': []}
    started = time.monotonic()
    try:
        process = subprocess.run(command, input=json.dumps(request, ensure_ascii=False) + '\n',
                                 text=True, encoding='utf-8', capture_output=True, timeout=timeout, check=False)
        result.update(returncode=process.returncode, stdout=process.stdout, stderr=process.stderr)
        if process.returncode != 0:
            raise ValueError('compiler exited with ' + str(process.returncode))
        lines = process.stdout.splitlines()
        if len(lines) != 1:
            raise ValueError('expected exactly one JSONLines response, got ' + str(len(lines)))
        reply = json.loads(lines[0])
        result['checks'] = evaluate(case, request, reply)
    except (OSError, subprocess.TimeoutExpired, ValueError, TypeError, KeyError) as error:
        result['checks'].append(check('compiler_execution', False, str(error)))
    result['elapsed_seconds'] = time.monotonic() - started
    statuses = {c['status'] for c in result['checks']}
    result['status'] = next((s for s in ('fail', 'unsupported', 'unverified') if s in statuses), 'pass')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compiler-sha', required=True, help='Exact reviewed source revision used to build the executable')
    parser.add_argument('--build-command', required=True, help='Recorded command used to build; never executed by this runner')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--timeout', type=float, default=10)
    parser.add_argument('command', nargs=argparse.REMAINDER, help='Executable argv after --; no shell interpolation')
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ['--'] else args.command
    if not command or args.timeout <= 0 or len(args.compiler_sha) != 40 or any(c not in '0123456789abcdef' for c in args.compiler_sha):
        parser.error('provide a command, positive timeout, and exact lowercase 40-digit compiler SHA')
    manifest, _ = corpus.validate()
    results = [run_case(case, command, args.timeout) for case in manifest['cases']]
    evidence = {'schema_version': 1, 'created_utc': datetime.now(timezone.utc).isoformat(),
                'compiler_sha': args.compiler_sha, 'build_command': args.build_command, 'command': command,
                'manifest_sha256': hashlib.sha256((corpus.ROOT / 'manifest.json').read_bytes()).hexdigest(),
                'limitations': 'Subprocess elapsed time includes startup; no incremental/latency benchmark. Manual semantic/layout gates remain unverified. Not proof of full compatibility.',
                'summary': dict(Counter(r['status'] for r in results)), 'results': results}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
    print(json.dumps(evidence['summary']))
    return 1 if any(r['status'] == 'fail' for r in results) else 0


if __name__ == '__main__':
    sys.exit(main())
