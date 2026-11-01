#!/usr/bin/env python3
"""Compare sequential persistent-process output with fresh-process output.

Equivalence is not semantic correctness, measured cache reuse, or a UI benchmark.
"""
import argparse
import copy
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import selectors
import subprocess
import sys
import tempfile
import time

SPEC = importlib.util.spec_from_file_location('corpus_run', Path(__file__).with_name('run.py'))
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)
ROOT = Path(__file__).resolve().parent


class Worker:
    def __init__(self, command, timeout):
        self.timeout = timeout
        self.stderr = tempfile.TemporaryFile()
        self.process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.stderr)
        self.selector = selectors.DefaultSelector()
        self.selector.register(self.process.stdout, selectors.EVENT_READ)
        self.buffer = b''

    def request(self, request):
        deadline = time.monotonic() + self.timeout
        self.process.stdin.write((json.dumps(request, ensure_ascii=False) + '\n').encode())
        self.process.stdin.flush()
        while b'\n' not in self.buffer:
            remaining = deadline - time.monotonic()
            if remaining <= 0 or not self.selector.select(remaining):
                raise TimeoutError('compiler response timeout')
            data = os.read(self.process.stdout.fileno(), 65536)
            if not data:
                raise ValueError('compiler closed stdout before a complete response')
            self.buffer += data
            if len(self.buffer) > 16 * 1024 * 1024:
                raise ValueError('response exceeds 16 MiB limit')
        line, self.buffer = self.buffer.split(b'\n', 1)
        raw = line.decode('utf-8')
        reply = json.loads(raw)
        return raw, reply

    def close(self):
        self.selector.close()
        self.process.stdin.close()
        try:
            self.process.wait(timeout=1)
        except subprocess.TimeoutExpired:
            self.process.kill(); self.process.wait()
        self.process.stdout.close()
        self.stderr.seek(0)
        stderr = self.stderr.read().decode('utf-8', errors='replace')
        self.stderr.close()
        return stderr


def scenarios():
    manifest, _ = runner.corpus.validate()
    cases = {c['id']: c for c in manifest['cases']}
    data = json.loads((ROOT / 'edit-scenarios.json').read_text())
    if data.get('schema_version') != 1 or not data.get('scenarios'):
        raise ValueError('invalid scenario manifest')
    seen = set()
    for scenario in data['scenarios']:
        if scenario['id'] in seen:
            raise ValueError('duplicate scenario')
        seen.add(scenario['id'])
        case = cases[scenario['case_id']]
        initial = runner.corpus.compile_request(case, 1)
        initial['payload']['project_id'] = 'edit-' + scenario['id']
        initial['id'] = scenario['id'] + '-1'
        requests = [('cold', initial)]
        warm = copy.deepcopy(initial); warm['payload']['revision'] = 2; warm['id'] = scenario['id'] + '-2'
        requests.append(('warm_unchanged', warm))
        for index, edit in enumerate(scenario['edits'], start=3):
            request = copy.deepcopy(requests[-1][1])
            document = next(d for d in request['payload']['documents'] if d['path'] == edit['path'])
            if not edit['old'] or document['text'].count(edit['old']) != 1:
                raise ValueError('edit target must occur exactly once: ' + scenario['id'])
            document['text'] = document['text'].replace(edit['old'], edit['new'], 1)
            request['payload']['revision'] = index; request['id'] = scenario['id'] + '-' + str(index)
            requests.append(('edit', request))
        yield scenario, case, requests


def correlation(request, reply):
    return (isinstance(reply, dict) and reply.get('protocol_version') == 1
            and reply.get('type') == 'compile_result' and reply.get('id') == request['id']
            and isinstance(reply.get('payload'), dict)
            and reply['payload'].get('project_id') == request['payload']['project_id']
            and reply['payload'].get('revision') == request['payload']['revision'])


def comparable(reply):
    p = reply['payload']
    value = {k: p[k] for k in ('status', 'pages', 'diagnostics')}
    # Artifact location is process-specific. Compare actual bytes when supplied.
    pdf = p.get('pdf_path')
    value['pdf_sha256'] = hashlib.sha256(Path(pdf).read_bytes()).hexdigest() if pdf else None
    return value


def run_scenario(scenario, case, requests, command, timeout=10, factory=Worker):
    result = {'scenario_id': scenario['id'], 'case_id': case['id'], 'purpose': scenario['purpose'], 'steps': []}
    worker = None
    started = time.monotonic()
    try:
        worker = factory(command, timeout)
        for index, (phase, request) in enumerate(requests):
            step = {'phase': phase, 'request': request}
            result['steps'].append(step)
            tick = started if index == 0 else time.monotonic()
            raw, reply = worker.request(request)
            step.update(persistent_raw=raw, persistent_seconds=time.monotonic() - tick)
            if not correlation(request, reply):
                raise ValueError('stale/mismatched persistent response rejected; no output accepted for this revision')
            left = comparable(reply)
            fresh = None
            try:
                tick = time.monotonic(); fresh = factory(command, timeout)
                clean_raw, clean_reply = fresh.request(request)
                step.update(clean_raw=clean_raw, clean_seconds=time.monotonic() - tick)
                if not correlation(request, clean_reply):
                    raise ValueError('stale/mismatched clean response rejected')
                # Capture the artifact hash before either process can replace its output.
                right = comparable(clean_reply)
                step['equivalent'] = left == right
                checks = runner.evaluate(case, request, reply)
                integrity = ('result_envelope', 'project_revision', 'result_structure', 'display_items_and_sources', 'diagnostic_sources')
                step['compiler_integrity_checks'] = [c for c in checks if c['check'] in integrity]
                step['integrity_ok'] = all(c['status'] == 'pass' for c in step['compiler_integrity_checks'])
                step['status'] = 'pass' if step['equivalent'] and step['integrity_ok'] else 'fail'
            finally:
                if fresh:
                    step['clean_stderr'] = fresh.close()
        result['status'] = 'pass' if all(s['status'] == 'pass' for s in result['steps']) else 'fail'
    except (OSError, ValueError, KeyError, TypeError, TimeoutError) as error:
        result['status'] = 'fail'; result['error'] = str(error)
    finally:
        if worker:
            result['persistent_stderr'] = worker.close()
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compiler-sha', required=True)
    parser.add_argument('--build-command', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--timeout', type=float, default=10)
    parser.add_argument('command', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ['--'] else args.command
    if not command or args.timeout <= 0 or len(args.compiler_sha) != 40 or any(c not in '0123456789abcdef' for c in args.compiler_sha):
        parser.error('command, positive timeout and exact SHA required')
    results = [run_scenario(s, c, r, command, args.timeout) for s, c, r in scenarios()]
    evidence = {'schema_version': 1, 'compiler_sha': args.compiler_sha, 'command': command,
                'build_command': args.build_command, 'created_utc': datetime.now(timezone.utc).isoformat(),
                'limitations': 'Sequential worker equivalence only: not proof of semantic correctness, cache reuse, native stale-preview suppression, or sub-200ms UI latency. Only revision-specific transport/source integrity checks are used; baseline semantic witnesses are not reused after edits.',
                'results': results}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, indent=2, ensure_ascii=False) + '\n')
    summary = {s: sum(r['status'] == s for r in results) for s in ('pass', 'fail')}
    print(json.dumps(summary))
    return int(summary['fail'] > 0)


if __name__ == '__main__':
    sys.exit(main())
