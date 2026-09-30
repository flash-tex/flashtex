#!/usr/bin/env python3
"""incr_sum.py JSONL...: edited-page CPU and wall p50/p95 (ms) of incr_bench.py outputs
(docs/evidence/p4-l2-l3-2026-09-29/scripts/incr_bench.py --out)."""
import json
import statistics
import sys


def pct(v, q):
    v = sorted(v)
    return v[int(q * (len(v) - 1))]


for path in sys.argv[1:]:
    r = [json.loads(l) for l in open(path) if l.strip()]
    r = [x for x in r if x.get('edited_page_cpu') is not None]
    c = [x['edited_page_cpu'] * 1000 for x in r]
    w = [x['edited_page_s'] * 1000 for x in r]
    print(f"{path}: {len(r)} compiles; edited-page CPU p50 {statistics.median(c):.1f} p95 {pct(c, .95):.1f} ms; "
          f"wall p50 {statistics.median(w):.1f} p95 {pct(w, .95):.1f} ms")
