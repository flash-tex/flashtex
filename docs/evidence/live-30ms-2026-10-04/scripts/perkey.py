#!/usr/bin/env python3
"""Per keystroke of a dl3-keys run (keyrun.py's JSONL; `--overlap` runs print each DONE on its own line): the
client's edited page, and the host's stages of the compile that answered it (DONE's `queue`, `queue_by` -- what
the engine thread did while the request waited --, `find`, `restore`, `edited_wall`, `first_page_dl`), then p50/p95
of each and the queue's parts summed over the run.

usage: perkey.py FILE.jsonl [--quiet]"""
import gzip, json, sys


def pct(v, p):
    v = sorted(v)
    return v[min(len(v) - 1, round((len(v) - 1) * p))] if v else None


recs = [json.loads(l) for l in (gzip.open(sys.argv[1], 'rt') if sys.argv[1].endswith('.gz') else open(sys.argv[1])) if l.startswith('{')]
quiet = '--quiet' in sys.argv
# the keystroke lines come in order; their compiles' ids are the open's + 1, + 2, ...
keys = [r for r in recs if 'key' in r]
dones = {r['done']['id']: r['done'] for r in recs if 'done' in r}
for k in keys:
    if 'host' in k:
        dones[k['host']['id']] = k['host']
ids = sorted(dones)
first_key_id = max(ids) - len(keys) + 1 if ids else 0
cols = ['edited', 'first_page', 'queue', 'find', 'restore', 'edited_wall', 'first_page_dl', 'test']
vals = {c: [] for c in cols}
by = {}
for i, k in enumerate(keys[1:], start=1):
    d = dones.get(first_key_id + i, {})
    st = d.get('stages', {})
    row = dict(edited=k.get('edited_page_ms'), **{c: st.get(c) for c in cols[1:]})
    for c in cols:
        if isinstance(row[c], (int, float)):
            vals[c].append(row[c])
    qb = st.get('queue_by') or {}
    for n, v in qb.items():
        by[n] = by.get(n, 0) + v
    if not quiet:
        print(f"key {i:2d} id {first_key_id + i} {d.get('status', '?'):9s} {d.get('mode', ''):11s} " +
              ' '.join(f"{c} {row[c]:.1f}" if isinstance(row[c], (int, float)) else f'{c} -' for c in cols) +
              ' by ' + ', '.join(f'{n} {v:.1f}' for n, v in qb.items() if v >= 0.5))
print('p50/p95: ' + ', '.join(f"{c} {pct(vals[c], .5):.1f}/{pct(vals[c], .95):.1f}" for c in cols if vals[c]))
print('queue by part (ms, summed over the run): ' + ', '.join(f'{n} {v:.0f}' for n, v in sorted(by.items(), key=lambda x: -x[1])))
