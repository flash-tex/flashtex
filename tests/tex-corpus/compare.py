#!/usr/bin/env python3
"""Gate corpus regressions against pinned evidence, never redefine expectations."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
from pathlib import Path
import sys

SPEC = importlib.util.spec_from_file_location('corpus_runner', Path(__file__).with_name('run.py'))
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)
ROOT = Path(__file__).resolve().parent
BASELINE_SHA = '9f1033ba75176236bc5fc66b5e64b8ebbfc15a37'
STATUSES = ('fail', 'unsupported', 'unverified', 'pass')


def overall(checks):
    statuses = {c['status'] for c in checks}
    return next((s for s in STATUSES[:-1] if s in statuses), 'pass')


def verify(artifact, expected_sha, expected_build, max_age=None, now=None):
    if len(expected_sha) != 40 or any(c not in '0123456789abcdef' for c in expected_sha):
        raise ValueError('expected compiler SHA must be an exact lowercase Git SHA')
    if not expected_build.strip():
        raise ValueError('expected build command must be explicit')
    if artifact.get('schema_version') != 1 or artifact.get('compiler_sha') != expected_sha:
        raise ValueError('compiler revision mismatch')
    if artifact.get('build_command') != expected_build:
        raise ValueError('build provenance mismatch')
    command = artifact.get('command')
    if not isinstance(command, list) or not command or not all(isinstance(x, str) and x for x in command):
        raise ValueError('missing executable provenance')
    stamp = datetime.fromisoformat(artifact['created_utc'])
    if stamp.tzinfo is None:
        raise ValueError('artifact timestamp must include timezone')
    age = ((now or datetime.now(timezone.utc)) - stamp).total_seconds()
    if age < -300 or (max_age is not None and age > max_age):
        raise ValueError('artifact is stale or implausibly future-dated')
    manifest, _ = runner.corpus.validate()
    if artifact.get('manifest_sha256') != hashlib.sha256((ROOT / 'manifest.json').read_bytes()).hexdigest():
        raise ValueError('manifest mismatch: regenerate comparable evidence for the same fixtures')
    expected = {c['id']: c for c in manifest['cases']}
    records = {}
    for result in artifact['results']:
        ident = result['case_id']
        if ident not in expected or ident in records:
            raise ValueError('unknown or duplicated result case: ' + ident)
        request = runner.corpus.compile_request(expected[ident], 1)
        if result['request'] != request:
            raise ValueError('stale/mismatched source request: ' + ident)
        if type(result.get('returncode')) is not int or not isinstance(result.get('stdout'), str):
            raise ValueError('incomplete process evidence: ' + ident)
        try:
            if result['returncode'] != 0:
                raise ValueError('compiler exited with ' + str(result['returncode']))
            lines = result['stdout'].splitlines()
            if len(lines) != 1:
                raise ValueError('expected exactly one JSONLines response, got ' + str(len(lines)))
            checks = runner.evaluate(expected[ident], request, json.loads(lines[0]))
        except (ValueError, TypeError, KeyError) as error:
            checks = [runner.check('compiler_execution', False, str(error))]
        if result.get('checks') != checks or result.get('status') != overall(checks):
            raise ValueError('recorded checks/status disagree with replayed evidence: ' + ident)
        records[ident] = result
    if set(records) != set(expected):
        raise ValueError('missing corpus cases')
    if artifact.get('summary') != dict(Counter(r['status'] for r in records.values())):
        raise ValueError('summary disagrees with case evidence')
    return records


def compare(before, after):
    if set(before) != set(after):
        raise ValueError('case sets differ')
    groups = {key: [] for key in ('newly_passing', 'regressed', 'still_passing', 'still_failing',
                                 'still_unsupported', 'still_unverified', 'changed_incomplete')}
    check_regressions = []
    for ident in sorted(before):
        old, new = before[ident], after[ident]
        left, right = old['status'], new['status']
        if right == 'pass' and left != 'pass':
            category = 'newly_passing'
        elif left == 'pass' and right == 'pass':
            category = 'still_passing'
        elif (left == 'pass' or (right == 'fail' and left != 'fail')):
            category = 'regressed'
        elif left == right:
            category = {'fail': 'still_failing', 'unsupported': 'still_unsupported', 'unverified': 'still_unverified'}[right]
        else:
            # Unsupported and unverified are not ordered measures of correctness.
            category = 'changed_incomplete'
        groups[category].append(ident)
        new_checks = {c['check']: c['status'] for c in new['checks']}
        for c in old['checks']:
            if c['status'] == 'pass' and new_checks.get(c['check']) != 'pass':
                check_regressions.append({'case_id': ident, 'check': c['check'], 'before': 'pass',
                                          'after': new_checks.get(c['check'], 'missing')})
    return {'groups': groups, 'check_regressions': check_regressions,
            'gate': 'fail' if groups['regressed'] or check_regressions else 'pass',
            'limitations': 'A non-regression gate is not project completion. Incomplete cases remain outstanding; improvements never cancel regressions.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline', type=Path, default=ROOT / 'evidence/compiler-9f1033b-included-source.json')
    parser.add_argument('--baseline-sha', default=BASELINE_SHA)
    parser.add_argument('--baseline-build-command', required=True)
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--candidate-sha', required=True)
    parser.add_argument('--candidate-build-command', required=True)
    parser.add_argument('--max-candidate-age-seconds', type=float, default=3600)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.max_candidate_age_seconds <= 0:
            raise ValueError('candidate maximum age must be positive')
        baseline = json.loads(args.baseline.read_text())
        candidate = json.loads(args.candidate.read_text())
        before = verify(baseline, args.baseline_sha, args.baseline_build_command)
        after = verify(candidate, args.candidate_sha, args.candidate_build_command, args.max_candidate_age_seconds)
        result = compare(before, after)
        result['baseline_sha'] = args.baseline_sha; result['candidate_sha'] = args.candidate_sha
        result['baseline_artifact_sha256'] = hashlib.sha256(args.baseline.read_bytes()).hexdigest()
        result['candidate_artifact_sha256'] = hashlib.sha256(args.candidate.read_bytes()).hexdigest()
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
        print(json.dumps(result, sort_keys=True))
        return int(result['gate'] != 'pass')
    except (OSError, ValueError, KeyError, TypeError) as error:
        print('Evidence rejected: ' + str(error), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
