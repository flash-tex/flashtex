#!/usr/bin/env python3
"""jumps_sum.py JUMPS.jsonl: per document, jump (step) and configuration, the jump's first key in ms
(median over rounds, and each round's) and the other keys' median, against main's; then the bar
(Commander 2026-11-01): vp300 no worse than main, vp0 no worse than main by more than 10 %."""
import json, statistics as st, sys
from collections import defaultdict

rows = [json.loads(l) for l in open(sys.argv[1]) if l.strip()]
first, rest = defaultdict(list), defaultdict(list)
for r in rows:
    if r['step'] == 0 or r['first_ms'] is None:
        continue
    k = (r['doc'], r['step'], r['page'], r['cfg'])
    first[k].append(r['first_ms'])
    rest[k].extend(r['rest_ms'])
med = lambda v: round(st.median(v), 1) if v else None  # noqa: E731
verdict = []
for (doc, step, page, cfg) in sorted(first, key=lambda k: (k[0], k[1], ['main', 'far2', 'vp300', 'vp0'].index(k[3]))):
    f, m = first[(doc, step, page, cfg)], first.get((doc, step, page, 'main'))
    ratio = med(f) / med(m) if m and med(m) else None
    print(f"{doc:11} jump {step} (page {page}) {cfg:5}: first {med(f):>8} ms {sorted(f)}  rest {med(rest[(doc, step, page, cfg)])} ms"
          + (f"  x{ratio:.2f} of main" if ratio and cfg != 'main' else ''))
    if cfg in ('vp300', 'vp0') and ratio:
        ok = ratio <= (1.0 if cfg == 'vp300' else 1.10)
        verdict.append(ok)
        if not ok:
            print(f"  MISS: {cfg} on {doc} jump {step}: x{ratio:.2f} of main")
print('bar met' if verdict and all(verdict) else 'bar NOT met' if verdict else 'no verdict')
