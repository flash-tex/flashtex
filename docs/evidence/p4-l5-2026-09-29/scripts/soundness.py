#!/usr/bin/env python3
"""Soundness test (P4-L2-L3, DESIGN.md §5.3): for every parity fixture (and
extra documents), random single-character edits at N positions; each
incremental compile (and each revert) must equal a from-scratch CLI run
byte for byte (PDF, log, aux, out, toc, terminal).

usage: soundness.py ENGINE [--trials N] [-j J] [--only NAME] [--extra DIR:DOC ...] [--out FILE]
"""
import argparse
import concurrent.futures
import json
import os
import subprocess
import sys

ROOT = '/Users/kubar/code/flashtex/.claude/worktrees/agent-adbdd56f16d07becc'
ap = argparse.ArgumentParser()
ap.add_argument('engine')
ap.add_argument('--trials', type=int, default=50)
ap.add_argument('-j', type=int, default=4)
ap.add_argument('--only')
ap.add_argument('--extra', action='append', default=[])
ap.add_argument('--out', default='/tmp/p4l5/soundness.jsonl')
ap.add_argument('--no-fixtures', action='store_true')
ap.add_argument('--dir', default='/tmp/p4l5/sound')
ap.add_argument('--kinds', default='replace,insert,delete')
a = ap.parse_args()


def fixtures():
    out = []
    for tier in ('fixtures/real-world', 'fixtures/divergence-probes'):
        base = os.path.join(ROOT, tier)
        for name in sorted(os.listdir(base)):
            d = os.path.join(base, name)
            if not os.path.isdir(d):
                continue
            texs = [f for f in os.listdir(d) if f.endswith('.tex')]
            main = 'main.tex' if 'main.tex' in texs else (texs[0] if len(texs) == 1 else None)
            if main:
                out.append((name, d, main[:-4]))
    return out


jobs = [] if a.no_fixtures else fixtures()
for e in a.extra:
    d, doc, *edit = e.split(':')
    jobs.insert(0, (doc, d, doc) + tuple(edit))
if a.only:
    jobs = [j for j in jobs if j[0] == a.only]


def run(job):
    name, d, doc = job[:3]
    edit = ['--edit', job[3]] if len(job) > 3 else []
    p = subprocess.run([sys.executable, '/tmp/p4l5/incr_bench.py', a.engine, d, doc, '--trials', str(a.trials)] + edit + [
                        '--verify', '--quiet', '--seed', str(abs(hash(name)) % 1000 + 1), '--any-letter', '--kinds', a.kinds,
                        '--out', f'{a.dir}/{name}.jsonl'],
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    last = [l for l in p.stdout.splitlines() if l.startswith('{')]
    if not last:
        return dict(name=name, error=(p.stderr or p.stdout)[-800:])
    r = json.loads(last[-1])
    r['name'] = name
    return r


os.makedirs(a.dir, exist_ok=True)
tot = dict(compiles=0, ok=0, bad=0, conv=0, acct=0, err=0)
with open(a.out, 'w') as out, concurrent.futures.ThreadPoolExecutor(a.j) as ex:
    for r in ex.map(run, jobs):
        out.write(json.dumps(r) + '\n')
        out.flush()
        if 'error' in r:
            tot['err'] += 1
            print(f"{r['name']}: ERROR {r['error'][-300:]}", flush=True)
            continue
        tot['compiles'] += r['compiles']
        tot['ok'] += r['verified_ok']
        tot['bad'] += r['mismatches']
        tot['conv'] += r['converged']
        tot['acct'] += r.get('accounting_only', 0)
        print(f"{r['name']}: {r['compiles']} compiles, {r['verified_ok']} ok, {r['mismatches']} mismatches, "
              f"{r['converged']} converged, {r.get('accounting_only', 0)} accounting-only log differences, modes {r['modes']}", flush=True)
print(json.dumps(tot))
