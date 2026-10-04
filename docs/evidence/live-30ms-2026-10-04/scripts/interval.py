#!/usr/bin/env python3
"""`dl3-keys --interval-ms` runs (keyrun.py's JSONL) as a table: per engine, document and phase, the keystrokes'
latency to the watched page painted (p50/p95/max ms; a keystroke waits for the first PAGE of its compile or a later
one), how many keystrokes were never painted, and the compiles cancelled.

usage: interval.py DIR... (each holding ENGINE-DOC-PHASE.jsonl files)"""
import glob, gzip, json, os, sys


def pct(v, p):
    v = sorted(v)
    return v[min(len(v) - 1, round((len(v) - 1) * p))] if v else float('nan')


rows = {}
for d in sys.argv[1:]:
    for f in sorted(glob.glob(f'{d}/*.jsonl') + glob.glob(f'{d}/*.jsonl.gz')):
        recs = [json.loads(l) for l in (gzip.open(f, 'rt') if f.endswith('.gz') else open(f)) if l.startswith('{')]
        summ = [r for r in recs if 'interval_ms' in r]
        if not summ:
            continue
        s = summ[-1]
        lat = [r['edited_page_ms'] for r in recs if 'key' in r and 'by' in r]
        name = os.path.basename(f).replace('.gz', '')[:-6]
        rows.setdefault(name, []).append((lat, s['unpainted'], s['cancelled'], s['pages']))
print('| engine-doc-phase | runs | keys painted | p50 | p95 | max | unpainted | cancelled |')
print('|---|---|---|---|---|---|---|---|')
for name, rs in sorted(rows.items()):
    lat = [x for r in rs for x in r[0]]
    print(f'| {name} | {len(rs)} | {len(lat)} | {pct(lat, .5):.1f} | {pct(lat, .95):.1f} | {max(lat) if lat else float("nan"):.1f} | '
          f'{sum(r[1] for r in rs)} | {sum(r[2] for r in rs)} |')
