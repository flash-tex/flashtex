#!/usr/bin/env python3
"""Tables from contig-bench's artifacts (lane CONTIG-TOKENS).

usage: summary.py DIR [BASE]   DIR holds the downloaded out-*/ artifacts; BASE (default main) is the
engine every delta is against.

Prints, per document: each engine's exact instruction count (cachegrind, any part) and its delta,
the native runs' median user time and perf counters (when the runner had them), and whether every
engine's PDF, log and memory-usage lines are the same bytes as BASE's in every run."""
import collections, glob, json, os, statistics, sys

d = sys.argv[1]
base = sys.argv[2] if len(sys.argv) > 2 else 'main'
runs = []
for f in glob.glob(os.path.join(d, '**', 'runs.jsonl'), recursive=True):
    part = os.path.basename(os.path.dirname(f))
    for l in open(f):
        r = json.loads(l)
        r['part'] = part
        runs.append(r)
docs = sorted({r['doc'] for r in runs})
engines = sorted({r['engine'] for r in runs}, key=lambda e: (e != base, e))

print('## exact instructions (cachegrind Ir), delta against', base)
print('| document | ' + ' | '.join(engines) + ' |')
print('|---|' + '---:|' * len(engines))
tot = collections.Counter()
have = collections.Counter()
for doc in docs:
    ir = {}
    for r in runs:
        if r['doc'] == doc and r['kind'].startswith('cg') and r.get('cg_ir'):
            ir[r['engine']] = r['cg_ir']
    cells = []
    for e in engines:
        if e not in ir:
            cells.append('-')
            continue
        if base in ir and e != base:
            cells.append(f"{ir[e] / 1e9:.3f} G ({100.0 * (ir[e] - ir[base]) / ir[base]:+.2f} %)")
        else:
            cells.append(f"{ir[e] / 1e9:.3f} G")
    if base in ir and all(e in ir for e in engines):
        for e in engines:
            tot[e] += ir[e]
            have[e] += 1
    print(f'| {doc} | ' + ' | '.join(cells) + ' |')
if tot[base]:
    print('| **all with every engine** | ' + ' | '.join(
        f"{tot[e] / 1e9:.3f} G" + ('' if e == base else f" ({100.0 * (tot[e] - tot[base]) / tot[base]:+.2f} %)")
        for e in engines) + ' |')

print('\n## native runs: median user s (perf instructions / cycles when present), per part')
for part in sorted({r['part'] for r in runs if r['kind'] == 'native'}):
    print(f'\n### {part}')
    print('| document | ' + ' | '.join(engines) + ' |')
    print('|---|' + '---:|' * len(engines))
    for doc in docs:
        cells = []
        med = {}
        for e in engines:
            rs = [r for r in runs if r['part'] == part and r['doc'] == doc and r['engine'] == e and r['kind'] == 'native']
            if not rs:
                cells.append('-')
                continue
            u = statistics.median(r['user'] for r in rs)
            med[e] = u
            c = f'{u:.2f}'
            ins = [r['instructions'] for r in rs if r.get('instructions')]
            cyc = [r['cycles'] for r in rs if r.get('cycles')]
            if ins:
                c += f' ({statistics.median(ins) / 1e9:.2f} G / {statistics.median(cyc) / 1e9:.2f} G)'
            if base in med and e != base:
                c += f' {100.0 * (u - med[base]) / med[base]:+.1f} %'
            cells.append(c)
        if any(c != '-' for c in cells):
            print(f'| {doc} | ' + ' | '.join(cells) + ' |')

print('\n## output identity against', base)
bad = 0
for doc in docs:
    ref = [r for r in runs if r['doc'] == doc and r['engine'] == base and r['kind'] != 'settle']
    if not ref:
        continue
    want = (ref[0]['pdf'], ref[0]['log'], ref[0]['mem'])
    for r in runs:
        if r['doc'] != doc or r['kind'] == 'settle':
            continue
        got = (r['pdf'], r['log'], r['mem'])
        if got != want:
            bad += 1
            print(f"DIFFERS: {doc} {r['engine']} {r['kind']} rep {r['rep']} part {r['part']}: "
                  f"pdf {got[0] == want[0]}, log {got[1] == want[1]}, mem {got[2] == want[2]}")
n = sum(1 for r in runs if r['kind'] != 'settle')
print(f'{n} runs compared, {bad} differ from {base}\'s first run of the same document')
rc = collections.Counter((r['engine'], r['rc']) for r in runs)
print('exit codes:', dict(rc))
