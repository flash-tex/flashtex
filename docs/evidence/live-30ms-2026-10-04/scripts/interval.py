#!/usr/bin/env python3
"""`dl3-keys --interval-ms` runs (keyrun.py's JSONL) as a table: per engine, document and phase, the keystrokes'
latency to the watched page painted (a keystroke waits for the first PAGE of its compile or a later one):

- wall p50/p95/max ms (client side; the machine's load is in it);
- **engine p50/p95 ms**: the engine thread's cycles from the keystroke's COMPILE arriving to the first page of the
  compile that painted it (DONE's `arrival_mark_kc`, `first_page_mark_kc`), at 3.2 GHz: the work the engine did
  meanwhile, which load does not change (it leaves out the socket and the client, under 1 ms);
- keystrokes never painted, compiles cancelled.

usage: interval.py DIR... (each holding ENGINE-DOC-PHASE.jsonl[.gz] files; several DIRs pool rounds)"""
import glob, gzip, json, os, sys

GHZ = 3.2


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
        keys = [r for r in recs if 'key' in r and 'by' in r]
        lat = [r['edited_page_ms'] for r in keys]
        dones = {r['done']['id']: r['done'] for r in recs if 'done' in r}
        n = len([r for r in recs if 'key' in r and 'by' in r]) + s['unpainted']
        first_id = max(dones) - n + 1 if dones else 0
        eng = []
        for r in keys:
            a = dones.get(first_id + r['key'], {})
            am = a.get('arrival_mark_kc', a.get('stages', {}).get('arrival_mark_kc'))
            pm = dones.get(r['by'], {}).get('stages', {}).get('first_page_mark_kc')
            if am is not None and pm is not None and pm >= am:
                eng.append((pm - am) / (GHZ * 1e3))
        name = os.path.basename(f).replace('.gz', '')[:-6]
        rows.setdefault(name, []).append((lat, eng, s['unpainted'], s['cancelled']))
print('| engine-doc-phase | runs | keys painted | wall p50 | wall p95 | wall max | engine p50 | engine p95 | engine max | '
      'unpainted | cancelled |')
print('|---|---|---|---|---|---|---|---|---|---|---|')
for name, rs in sorted(rows.items()):
    lat = [x for r in rs for x in r[0]]
    eng = [x for r in rs for x in r[1]]
    mx = max(lat) if lat else float('nan')
    em = max(eng) if eng else float('nan')
    print(f'| {name} | {len(rs)} | {len(lat)} | {pct(lat, .5):.1f} | {pct(lat, .95):.1f} | {mx:.1f} | '
          f'{pct(eng, .5):.1f} | {pct(eng, .95):.1f} | {em:.1f} | {sum(r[2] for r in rs)} | {sum(r[3] for r in rs)} |')
