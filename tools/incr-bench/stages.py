#!/usr/bin/env python3
"""stages.py FILE.jsonl...: p50 / p95 of each host stage (DONE's `stages`) and of the
client-side first/edited page over the keystroke lines of dl3-keys output."""
import json
import sys


def pct(v, p):
    v = sorted(v)
    return v[round((len(v) - 1) * p)] if v else float("nan")


for f in sys.argv[1:]:
    rows = [json.loads(l) for l in open(f) if l.startswith('{"key"')]
    cols = {}
    for r in rows:
        for k in ("first_page_ms", "edited_page_ms", "done_ms"):
            if isinstance(r.get(k), (int, float)):
                cols.setdefault("client_" + k.replace("_ms", ""), []).append(r[k])
        for k, v in (r.get("host", {}).get("stages") or {}).items():
            if isinstance(v, (int, float)):
                cols.setdefault(k, []).append(v)
        h = r.get("host", {})
        for k in ("run_ms", "elapsed_ms"):
            if isinstance(h.get(k), (int, float)):
                cols.setdefault("host_" + k.replace("_ms", ""), []).append(h[k])
    print(f"{f}: {len(rows)} keystrokes")
    print("   " + "  ".join(f"{k} {pct(v, .5):.2f}/{pct(v, .95):.2f}" for k, v in cols.items()))
