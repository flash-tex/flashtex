#!/usr/bin/env python3
"""keyrun.py's summaries as a table: per document and phase, the client's edited page and DONE (p50/p95 ms) and
the host's stages (p50/p95: wall ms, then instructions in millions).

usage: table.py OUTDIR [ENGINE]"""
import glob, json, sys

out = sys.argv[1]
eng = sys.argv[2] if len(sys.argv) > 2 else '*'
WALL = ['queue', 'find', 'restore', 'edited_wall', 'first_page_dl', 'test']
INSTR = ['restore_instr_k', 'edited_instr_k', 'test_instr_k', 'instr_k']


def f(x):
    return '-' if x is None else (f'{x:.1f}' if x < 100 else f'{x:.0f}')


def pp(d, k, scale=1.0):
    s = d.get(k)
    if not s:
        return '-'
    return f"{f(s['p50'] * scale)}/{f(s['p95'] * scale)}"


print('| doc | phase | n | edited page p50/p95 | DONE p50/p95 | ' + ' | '.join(WALL) + ' | ' +
      ' | '.join(k.replace('_k', ' M') for k in INSTR) + ' | pages typeset p50 | conv | cancelled | load |')
print('|' + '---|' * (9 + len(WALL) + len(INSTR)))
for p in sorted(glob.glob(f'{out}/summary-{eng}-*.json')):
    s = json.load(open(p))
    for name, ph in s['phases'].items():
        st = ph['stages']
        e = ph.get('edited_page_ms') or {}
        d = ph.get('done_ms') or {}
        ts = ph['typeset_pages']
        row = [s['doc'], name, str(ph['n']), f"{f(e.get('p50'))}/{f(e.get('p95'))}",
               f"{f(d.get('p50'))}/{f(d.get('p95'))}"]
        row += [pp(st, k) for k in WALL]
        row += [pp(st, k, 1e-3) for k in INSTR]
        row += [str(ts[len(ts) // 2]) if ts else '-', str(ph['converged']), str(ph['cancelled']),
                f"{ph['load'][0]:.0f}-{ph['load'][1]:.0f}"]
        print('| ' + ' | '.join(row) + ' |')
