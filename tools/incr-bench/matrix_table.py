#!/usr/bin/env python3
"""matrix_table.py DIR: the §1.2 table of the latency matrix (matrix.py's
DIR/<doc>-<region>-<type>.jsonl): per document and region (the char and
sentence sessions together), the edited page's p50 / p95 wall and thread-CPU
milliseconds, and over all the document's compiles; then what the later
passes (DESIGN.md 5.5, the background's) cost. A compile whose first pass
changed no page (an edit that changes nothing visible) has no edited page and
is not counted."""
import collections
import glob
import json
import os
import sys


def pct(xs, q):
    if not xs:
        return None
    xs = sorted(xs)
    k = min(len(xs) - 1, max(0, int(round(q / 100 * (len(xs) - 1)))))
    return xs[k]


def f(x):
    return '-' if x is None else (f'{1000 * x:,.1f}' if x < 10 else f'{1000 * x:,.0f}')


rows = collections.defaultdict(list)
for fn in sorted(glob.glob(os.path.join(sys.argv[1], '*.jsonl'))):
    base = os.path.basename(fn)[:-6]
    doc, region, typ = base.rsplit('-', 2)
    for ln in open(fn):
        rows[(doc, region)].append(json.loads(ln))

order = lambda d: (d.split('-')[0], int(d.split('-')[1]))
docs = sorted({k[0] for k in rows}, key=order)
print('| doc | start p50 / p95 ms (CPU) | middle p50 / p95 ms (CPU) | end p50 / p95 ms (CPU) | all: p50 / p95 wall | p95 CPU | n |')
print('|---|---|---|---|---|---|---|')
bg = []
for doc in docs:
    cells = []
    allw, allc = [], []
    for region in ('start', 'middle', 'end'):
        v = [r for r in rows.get((doc, region), []) if r.get('edited_page_s') is not None]
        w = [r['edited_page_s'] for r in v]
        c = [r['edited_page_cpu'] for r in v]
        allw += w
        allc += c
        cells.append(f'{f(pct(w, 50))} / {f(pct(w, 95))} ({f(pct(c, 50))} / {f(pct(c, 95))})')
        for r in rows.get((doc, region), []):
            later = (r.get('pass_s') or [])[1:]
            if later:
                bg.append((doc, region, r['tag'], len(later), sum(later), r.get('l5')))
    print(f'| {doc} | ' + ' | '.join(cells) + f' | {f(pct(allw, 50))} / {f(pct(allw, 95))} | {f(pct(allc, 95))} | {len(allw)} |')

print()
print('Later passes (the background: DESIGN.md 5.5), per document: compiles with more than one pass, '
      'and the p50 / max of their later passes\' total time')
print()
print('| doc | compiles with later passes | later passes p50 / max ms |')
print('|---|---|---|')
per = collections.defaultdict(list)
for doc, region, tag, n, t, l5 in bg:
    per[doc].append(t)
for doc in docs:
    v = per.get(doc, [])
    print(f'| {doc} | {len(v)} | {f(pct(v, 50))} / {f(max(v) if v else None)} |')
