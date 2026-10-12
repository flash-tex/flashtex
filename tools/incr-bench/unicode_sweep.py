#!/usr/bin/env python3
"""Unicode mode's soundness sweep (lane XETEX-INCR): `unicode.py --verify`
over the Unicode corpus, the PDF parity cases (#1708) and the lockstep's
LaTeX cases, `-j` documents at a time; every compile of the measured host
must equal a from-scratch compile (unicode.py's docstring says how).

usage: unicode_sweep.py HOST [-j 2] [--keys 2] [--trials 3] [--kinds ...]
                        [--only SUBSTRING] [--out JSONL]

Exit status 1 when any compile differs (or a document's run fails).
"""
import argparse
import concurrent.futures
import glob
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, '../..'))
ap = argparse.ArgumentParser()
ap.add_argument('host')
ap.add_argument('-j', type=int, default=2)
ap.add_argument('--keys', type=int, default=2)
ap.add_argument('--trials', type=int, default=3)
ap.add_argument('--kinds', default='replace,insert,delete,sentence')
ap.add_argument('--only')
ap.add_argument('--out')
ap.add_argument('--host-env', action='append', default=[])
ap.add_argument('--interleave', action='store_true', help='sweep D: cancel each edit\'s compile with a second edit saved meanwhile (unicode.py --interleave)')
a = ap.parse_args()

docs = sorted(glob.glob(os.path.join(HERE, 'unicode-corpus', '*.tex')))
docs += sorted(glob.glob(os.path.join(ROOT, 'tools/xetex-pdfparity/cases', '*.tex')))
docs += sorted(glob.glob(os.path.join(ROOT, 'tools/xetex-lockstep/latex-cases', '*.tex')))
if a.only:
    docs = [d for d in docs if a.only in os.path.basename(d)]


def one(doc):
    cmd = [sys.executable, os.path.join(HERE, 'unicode.py'), a.host, doc, '--verify', '--keys', str(a.keys),
           '--trials', str(a.trials), '--kinds', a.kinds, '--at', 'mid']
    if a.interleave:
        cmd.append('--interleave')
    for kv in a.host_env:
        cmd += ['--host-env', kv]
    r = subprocess.run(cmd, capture_output=True, text=True)
    lines = [json.loads(l) for l in r.stdout.splitlines() if l.startswith('{')]
    summary = lines[-1] if lines and 'compiles' in lines[-1] else None
    return doc, r.returncode, lines, summary, r.stderr


bad = 0
out = open(a.out, 'w') if a.out else None
with concurrent.futures.ThreadPoolExecutor(a.j) as ex:
    for doc, rc, lines, summary, err in ex.map(one, docs):
        name = os.path.basename(doc)
        if summary is None:
            bad += 1
            print('%-40s FAILED (exit %d) %s' % (name, rc, err.strip().splitlines()[-1:] if err else ''), flush=True)
            continue
        mism = [l for l in lines[:-1] if l.get('mismatch')]
        bad += len(mism) or (rc != 0)
        keys = [l for l in lines if str(l.get('tag', '')).startswith('key')]
        med = sorted(l['edited_ms'] for l in keys)[len(keys) // 2] if keys else None
        ins = sorted(l['instructions_m'] for l in keys if l.get('instructions_m') is not None)
        first = [(l.get('starts') or [None])[0] for l in lines[:-1]]
        s0, page = first.count('s0'), first.count('page')
        print('%-40s %3d compiles (%d from S0, %d from a page), %d mismatches (%d accounting only); keystroke edited page p50 %s ms, %s M instr'
              % (name, summary['compiles'], s0, page, summary['mismatches'], summary.get('accounting', 0), med,
                 ins[len(ins) // 2] if ins else None),
              flush=True)
        for m in mism:
            print('    %s: %s' % (m['tag'], m['mismatch']), flush=True)
        if out:
            for l in lines:
                l['doc'] = name
                out.write(json.dumps(l) + '\n')
print('TOTAL: %d bad' % bad)
sys.exit(1 if bad else 0)
