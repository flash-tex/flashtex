#!/usr/bin/env python3
"""interval.py's latencies pooled per engine (over every document and phase given) and per engine and document.

usage: pool.py DIR..."""
import glob, gzip, json, os, sys

GHZ = 3.2


def pct(v, p):
    v = sorted(v)
    return v[min(len(v) - 1, round((len(v) - 1) * p))] if v else float('nan')


res = {}
for d in sys.argv[1:]:
    for f in glob.glob(f'{d}/*.jsonl') + glob.glob(f'{d}/*.jsonl.gz'):
        op = gzip.open(f, 'rt') if f.endswith('.gz') else open(f)
        recs = [json.loads(l) for l in op if l.startswith('{')]
        summ = [r for r in recs if 'interval_ms' in r]
        if not summ:
            continue
        s = summ[-1]
        keys = [r for r in recs if 'key' in r and 'by' in r]
        dones = {r['done']['id']: r['done'] for r in recs if 'done' in r}
        n = len(keys) + s['unpainted']
        fid = max(dones) - n + 1
        name = os.path.basename(f).replace('.gz', '')[:-6]
        parts = name.split('-')
        eng, doc = parts[0], '-'.join(parts[1:3])
        for r in keys:
            a = dones.get(fid + r['key'], {})
            am = a.get('arrival_mark_kc', a.get('stages', {}).get('arrival_mark_kc'))
            pm = dones.get(r['by'], {}).get('stages', {}).get('first_page_mark_kc')
            e = (pm - am) / (GHZ * 1e3) if am is not None and pm is not None and pm >= am else None
            for k in [(eng, 'all'), (eng, doc)]:
                v = res.setdefault(k, {'w': [], 'e': [], 'u': 0})
                v['w'].append(r['edited_page_ms'])
                if e is not None:
                    v['e'].append(e)
        for k in [(eng, 'all'), (eng, doc)]:
            res.setdefault(k, {'w': [], 'e': [], 'u': 0})['u'] += s['unpainted']
print('| docs | engine | keys | wall p50/p95/p99 ms | engine p50/p95/p99 ms | unpainted |')
print('|---|---|---|---|---|---|')
for (e, d), v in sorted(res.items(), key=lambda t: (t[0][1], t[0][0])):
    w, x = v['w'], v['e']
    print(f"| {d} | {e} | {len(w)} | {pct(w, .5):.1f}/{pct(w, .95):.1f}/{pct(w, .99):.1f} | "
          f"{pct(x, .5):.1f}/{pct(x, .95):.1f}/{pct(x, .99):.1f} | {v['u']} |")
