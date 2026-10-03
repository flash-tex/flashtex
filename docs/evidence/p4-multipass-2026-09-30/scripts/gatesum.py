#!/usr/bin/env python3
"""Summarize the gate bench (gate/*/result.json and the one-pass pdflatex times)."""
import json, os, statistics, sys
G = sys.argv[1] if len(sys.argv) > 1 else "/Users/dqi26/flashtex-wt/d1/gate"
NEW = sys.argv[2] if len(sys.argv) > 2 else "v4"


def med(xs):
    return round(statistics.median(xs), 2) if xs else None


for doc in ("biblatex-120", "book-300"):
    new = json.load(open(os.path.join(G, f"{doc}-{NEW}", "result.json")))
    base = json.load(open(os.path.join(G, f"{doc}-base", "result.json")))
    lo, le = new["lmk_open"], new["lmk_edits"]
    ho, he = new["host_open"], new["host_edits"]
    bo, be = base["host_open"], base["host_edits"]
    print(f"== {doc}")
    print(f"  latexmk open {lo['wall']}s (CPU {lo['cpu']}, load {lo['load']}, rules {lo['rules']})")
    print(f"  latexmk edits wall {[e['wall'] for e in le]} CPU {[e['cpu'] for e in le]} loads {[e['load'] for e in le]}"
          f" runs {[len(e['rules']) for e in le]}")
    for tag, o, es in (("new", ho, he), ("base", bo, be)):
        print(f"  host[{tag}] open {o['wall']}s (host CPU {o['host_cpu']}, load {o['load']});"
              f" edits wall {[e['wall'] for e in es]} host CPU {[e['host_cpu'] for e in es]}"
              f" loads {[e['load'] for e in es]}")
        print(f"     tools {[[(t['tool'], t['ms']) for t in e['tools']] for e in es][:1]}"
              f" dones {[[(d.get('mode'), d.get('passes'), d.get('deferred'), d.get('run_ms')) for d in e['dones']] for e in es][:1]}")
    lm = med([e["wall"] for e in le])
    hm = med([e["wall"] for e in he])
    bm = med([e["wall"] for e in be])
    print(f"  median edit: latexmk {lm}s, new {hm}s ({round(hm / lm, 2)}x), base {bm}s ({round(bm / lm, 2)}x);"
          f" open: latexmk {lo['wall']}, new {ho['wall']} ({round(ho['wall'] / lo['wall'], 2)}x),"
          f" base {bo['wall']} ({round(bo['wall'] / lo['wall'], 2)}x)")
    same = all(a["pdf"] == b["pdf"] for a, b in zip([lo] + le, [ho] + he))
    print(f"  exported PDFs byte-identical to latexmk (open + edits): {same}"
          f" {[a['pdf'] == b['pdf'] for a, b in zip([lo] + le, [ho] + he)]}")
    op = os.path.join(G, f"{doc}-pdflatex-onepass.json")
    if os.path.exists(op):
        print(f"  one pdflatex run: {open(op).read().strip()}")
