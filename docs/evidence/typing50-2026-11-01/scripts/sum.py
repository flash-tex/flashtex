#!/usr/bin/env python3
"""sum.py RUNDIR...: per typing phase, keys painted by their own compile, latency p50/p95, and per
compile that started: queue (by part), find, restore, wall/CPU to its first page, engine cycles from
arrival to the first page (Mcycles), and whether the compile shipped its page."""
import json, sys, glob, os, statistics as st


def pct(v, q):
    v = sorted(v)
    return v[min(len(v) - 1, int(round(q * (len(v) - 1))))] if v else None


for d in sys.argv[1:]:
    for f in sorted(glob.glob(f'{d}/raw/*typing@*.jsonl')):
        keys, dones = [], {}
        for l in open(f):
            r = json.loads(l)
            if 'key' in r:
                keys.append(r)
            elif 'done' in r:
                dones[r['done']['id']] = r['done']
        ids = sorted(dones)
        first = min(i for i in ids if i > 1) if ids else 0
        ks = [k for k in keys if k['key'] > 0]  # warm-up dropped
        own = [k for k in ks if k.get('by') == first + k['key']]
        lat = [k['edited_page_ms'] for k in ks]
        late = sum(1 for k in ks if (k.get('sent_late_ms') or 0) > 5)
        ran = [dones[i] for i in ids if i >= first and dones[i].get('stages', {}).get('find') is not None
               and dones[i].get('mode') != 'unchanged']
        q = [x['stages'].get('queue', 0) for x in ran]
        e_cyc = []
        for x in ran:
            s = x['stages']
            if s.get('first_page_mark_kc') and s.get('arrival_mark_kc'):
                e_cyc.append((s['first_page_mark_kc'] - s['arrival_mark_kc']) / 1e3)
        qb = {}
        for x in ran:
            for k, v in (x['stages'].get('queue_by') or {}).items():
                qb[k] = qb.get(k, 0) + v
        fp = [x['first_page_ms'] for x in ran if x.get('first_page_ms')]
        noship = sum(1 for x in ran if not x.get('first_page_ms'))
        print(f"{os.path.basename(d):28s} {os.path.basename(f).split('-', 2)[-1][:-6]:22s} own {len(own)}/{len(ks)} "
              f"late {late} lat p50/p95 {pct(lat, .5) or 0:.0f}/{pct(lat, .95) or 0:.0f} | ran {len(ran)} noship {noship} "
              f"fp p50 {pct(fp, .5) or 0:.0f} queue p50/max {pct(q, .5) or 0:.1f}/{max(q, default=0):.1f} "
              f"Mcyc-to-page p50 {pct(e_cyc, .5) or 0:.0f} | queue_by " +
              ' '.join(f'{k} {v:.0f}' for k, v in sorted(qb.items(), key=lambda kv: -kv[1]) if v >= 1))
