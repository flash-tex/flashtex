#!/usr/bin/env python3
"""Per-kind precision table from compare.py's --json output (Markdown).

    python3 tools/diag-oracle/table.py compare.json
"""
import json
import os
import sys

j = json.load(open(sys.argv[1]))
kinds = ['error', 'warning', 'show', 'box', 'pdfwarning']
t = {k: dict(n=0, pos=0, o_line=0, o_col=0, o_range=0, v_line=0, v_cover=0, v_col=0, v_any=0,
             nopos=0, o_extra_pos=0) for k in kinds}
for r in j['rows']:
    k = r['kind']
    c = t[k]
    c['n'] += 1
    e, o, v = r['expected'], r['ours'] or {}, r.get('v1')
    if k == 'box':
        if e.get('lines'):
            c['pos'] += 1
            c['o_line'] += o.get('lines') == e['lines']
            c['o_col'] += o.get('col') is not None
            c['v_line'] += bool(v)
        continue
    if e.get('line') is None:
        c['nopos'] += 1
        c['o_extra_pos'] += o.get('line') is not None
        continue
    c['pos'] += 1
    same_file = not e.get('file') or os.path.basename(o.get('file') or '') == os.path.basename(e['file'])
    c['o_line'] += same_file and o.get('line') == e['line']
    c['o_col'] += same_file and o.get('line') == e['line'] and o.get('col') == e.get('col')
    rng = o.get('range')
    c['o_range'] += bool(rng) and e.get('col') is not None and rng[0] <= e['col'] <= rng[1]
    c['v_line'] += bool(v)
    c['v_cover'] += bool(r.get('v1_cover'))
print('| kind | reports | with a pdflatex position | engine: same line | engine: same column | v1: same line | v1: span covers the column |')
print('|---|---:|---:|---:|---:|---:|---:|')
for k in kinds:
    c = t[k]
    if not c['n']:
        continue
    print('| %s | %d | %d | %d | %s | %d | %s |' % (
        k, c['n'], c['pos'], c['o_line'], ('%d (first char: %d)' % (c['o_line'], c['o_col'])) if k == 'box' else c['o_col'],
        c['v_line'], '—' if k == 'box' else c.get('v_cover', 0)))
print()
print(json.dumps(j['totals'], sort_keys=True))
