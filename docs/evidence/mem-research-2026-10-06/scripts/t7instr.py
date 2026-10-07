#!/usr/bin/env python3
"""t7instr.py DIR...: per T7 run dir and phase, edited-page instructions (p50/p95, M), the whole keystroke's
instructions (p50), host RSS peak (MB), and the last keystroke's `mem` (sealed logs, page cache)."""
import json, os, sys
def pct(v, p):
    v = sorted(v); return v[round((len(v) - 1) * p)] if v else float('nan')
for D in sys.argv[1:]:
    for doc in sorted(os.listdir(D)):
        raw = f'{D}/{doc}/raw'
        if not os.path.isdir(raw):
            continue
        summ = json.load(open(f'{D}/{doc}/summary.json'))
        rss = max((r.get('rss') or 0) for r in summ['rows']) >> 20
        smp = f'{D}/{doc}-smaps.json'
        speak = json.load(open(smp))['peak'] >> 20 if os.path.exists(smp) else 0
        print(f'== {D} {doc}: host RSS peak {rss} MB (sampled {speak} MB)')
        for f in sorted(os.listdir(raw)):
            rs = []
            for l in open(f'{raw}/{f}'):
                if not l.startswith('{'):
                    continue
                r = json.loads(l)
                h = r.get('host') or {}
                st = h.get('stages') or {}
                if 'edited_instr_k' in st:
                    rs.append((st['edited_instr_k'] / 1e3, st.get('instr_k', 0) / 1e3, h.get('mem') or {}))
            rs = rs[1:]
            if not rs:
                continue
            e = [x[0] for x in rs]; w = [x[1] for x in rs]; m = rs[-1][2]
            print(f'  {f[len(doc)+1:-6]:16} n {len(e):3} edited instr p50 {pct(e,.5):7.1f} M p95 {pct(e,.95):7.1f} M'
                  f' | keystroke p50 {pct(w,.5):8.1f} M | logs {m.get("sealed_bytes",0)/2**20:6.1f} MB'
                  f' ckpts {m.get("checkpoints")} page_cache {m.get("page_cache",0)/2**20:5.1f}')
