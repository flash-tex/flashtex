#!/usr/bin/env python3
"""matrix.py ENGINE OUTDIR: the §1.2 latency matrix. For every benchmark
document (plain/full x 10/100/300/1000 pages), edit region (start, middle,
end: the first tenth, the middle tenth, the last tenth of the prose) and edit
type (a typing session of single-character edits near one place; sentence
insertions, which reflow), one host session: settle, then edit and revert.
Records go to OUTDIR/<doc>-<region>-<type>.jsonl; summarise with
matrix_sum.py."""
import os
import subprocess
import sys

E, OUT = sys.argv[1], sys.argv[2]
only = sys.argv[3:]
os.makedirs(OUT, exist_ok=True)
for kind in ('plain', 'full'):
    for n in (10, 100, 300, 1000):
        doc = f'{kind}-{n}'
        if only and doc not in only:
            continue
        for region in ('start', 'middle', 'end'):
            for typ, kinds, trials, extra in (('char', 'replace,insert,delete', 8, ['--window', '400']),
                                               ('sentence', 'sentence', 3, [])):
                out = f'{OUT}/{doc}-{region}-{typ}.jsonl'
                if os.path.exists(out):
                    continue
                p = subprocess.run([sys.executable, os.path.join(os.path.dirname(os.path.abspath(__file__)), 'incr_bench.py'), E, f'/tmp/p4f/src-{doc}', doc,
                                    '--region', region, '--kinds', kinds, '--trials', str(trials),
                                    '--seed', str(n + len(region)), '--quiet', '--out', out + '.tmp'] + extra,
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=3600)
                if p.returncode != 0:
                    print(f'{doc} {region} {typ}: FAILED {p.stderr[-400:]}', flush=True)
                    continue
                os.rename(out + '.tmp', out)
                print(f'{doc} {region} {typ}: {p.stdout.strip()[-200:]}', flush=True)
