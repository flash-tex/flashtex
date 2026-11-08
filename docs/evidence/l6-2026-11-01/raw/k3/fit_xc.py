#!/usr/bin/env python3
"""fit_xc.py DIR: XC@col@rlet's net instructions (h3 - h2) per document against its hits and
recordings (h3's stats minus h2's), fitted per engine as net = -a*hits + b*recordings (least
squares, no intercept): a is what a hit saves, b what a recording costs."""
import json
import os
import re
import statistics as st
import sys

d = sys.argv[1]
rows = [json.loads(l) for l in open(f'{d}/runs.jsonl')]
engines = sorted({r['engine'] for r in rows})
docs = []
for r in rows:
    if r['doc'] not in docs:
        docs.append(r['doc'])


def m(doc, e, v):
    xs = [r['instr'] for r in rows if r['doc'] == doc and r['engine'] == e and r['variant'] == v]
    return st.median(xs) if xs else None


def stats(doc, e, v):
    f = f"{d}/bm-{doc.replace(':', '_')}-{e}-{v}.txt"
    if not os.path.exists(f):
        return 0, 0, 0
    t = open(f).read()
    g = lambda k: int(re.search(rf'\b{k}: (\d+)', t).group(1)) if re.search(rf'\b{k}: (\d+)', t) else 0
    return g('offers'), g('hits'), g('recordings')


for e in engines:
    pts = []
    for doc in docs:
        a, b = m(doc, e, 'h2'), m(doc, e, 'h3')
        if a is None or b is None:
            continue
        o2, h2, r2 = stats(doc, e, 'h2')
        o3, h3, r3 = stats(doc, e, 'h3')
        hx, rx = h3 - h2, r3 - r2
        if hx == 0 and rx == 0:
            continue
        pts.append((doc, b - a, hx, rx))
    # least squares for net = -A*h + B*r
    shh = sum(h * h for _, _, h, _ in pts)
    srr = sum(r * r for _, _, _, r in pts)
    shr = sum(h * r for _, _, h, r in pts)
    sny = sum(n * h for _, n, h, _ in pts)
    snr = sum(n * r for _, n, _, r in pts)
    det = shh * srr - shr * shr
    A = -(sny * srr - snr * shr) / det
    B = (snr * shh - sny * shr) / det
    print(f'{e}: a hit saves {A / 1e6:.3f} M, a recording costs {B / 1e6:.3f} M '
          f'(break-even {B / A:.2f} hits per recording)')
    for doc, n, h, r in pts:
        print(f'   {doc}: net {n / 1e9:+.3f} G, hits {h}, recordings {r}, fit {(-A * h + B * r) / 1e9:+.3f} G')
