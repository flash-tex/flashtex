#!/usr/bin/env python3
"""matrix_sum.py DIR: p50/p95 of the edited page's latency (wall and thread
CPU, ms) by document, region and edit type, and split into reflowing
(more than the edited page changed) and non-reflowing edits."""
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


def fmt(x):
    return '-' if x is None else f'{1000 * x:.1f}'


rows = collections.defaultdict(list)
for f in sorted(glob.glob(os.path.join(sys.argv[1], '*.jsonl'))):
    base = os.path.basename(f)[:-6]
    doc, region, typ = base.rsplit('-', 2)[0], base.rsplit('-', 2)[1], base.rsplit('-', 2)[2]
    for ln in open(f):
        r = json.loads(ln)
        if r.get('edited_page_s') is None:
            continue
        reflow = 'reflow' if (r.get('changed_pages') or 0) > 1 else 'local'
        rows[(doc, region, typ)].append((r['edited_page_s'], r.get('edited_page_cpu'), reflow, r))

order = lambda d: (d.split('-')[0], int(d.split('-')[1]))
docs = sorted({k[0] for k in rows}, key=order)
print('| doc | region | edits | edited page p50 / p95 wall ms | p50 / p95 CPU ms | reflowing | restore p50 ms |')
print('|---|---|---|---|---|---|---|')
agg = collections.defaultdict(list)
for doc in docs:
    for region in ('start', 'middle', 'end'):
        for typ in ('char', 'sentence'):
            v = rows.get((doc, region, typ))
            if not v:
                continue
            w = [x[0] for x in v]
            c = [x[1] for x in v if x[1] is not None]
            nre = sum(1 for x in v if x[2] == 'reflow')
            rs = [x[3]['restore_s'] for x in v]
            print(f'| {doc} | {region} {typ} | {len(v)} | {fmt(pct(w, 50))} / {fmt(pct(w, 95))} | '
                  f'{fmt(pct(c, 50))} / {fmt(pct(c, 95))} | {nre} | {fmt(pct(rs, 50))} |')
            for x in v:
                agg[(doc, x[2])].append(x)
print()
print('| doc | kind | edits | p50 / p95 wall ms | p50 / p95 CPU ms |')
print('|---|---|---|---|---|')
for doc in docs:
    for kind in ('local', 'reflow'):
        v = agg.get((doc, kind))
        if not v:
            continue
        w = [x[0] for x in v]
        c = [x[1] for x in v if x[1] is not None]
        print(f'| {doc} | {kind} | {len(v)} | {fmt(pct(w, 50))} / {fmt(pct(w, 95))} | {fmt(pct(c, 50))} / {fmt(pct(c, 95))} |')
