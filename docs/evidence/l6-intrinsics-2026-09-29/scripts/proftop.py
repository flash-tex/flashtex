#!/usr/bin/env python3
"""proftop.py PROFILE.tsv [N] [--incl]: the top N macros of a FLASHTEX_MACRO_PROFILE file,
by self time (default) or inclusive time, as a Markdown table with per-call and per-page
figures (pages = the profile's shipouts)."""
import sys

path = sys.argv[1]
n = int(sys.argv[2]) if len(sys.argv) > 2 and sys.argv[2].isdigit() else 20
incl = '--incl' in sys.argv
head = {}
rows = []
for l in open(path):
    if l.startswith('# total_ns'):
        for kv in l[2:].split():
            k, v = kv.split('=')
            head[k] = float(v)
        continue
    if l.startswith('#'):
        continue
    s, i, c, name = l.rstrip('\n').split('\t', 3)
    rows.append((float(s), float(i), int(c), name))
rows.sort(key=lambda r: -(r[1] if incl else r[0]))
pages = head.get('shipouts', 1) or 1
tot = head.get('total_ns', 1)
print(f"total {tot/1e6:.0f} ms profiled, {pages:.0f} pages, outside any macro {head.get('none_ns',0)/1e6:.0f} ms")
print('| # | macro | calls | self ms | self % | incl ms | self us/call | self us/page |')
print('|---|---|---|---|---|---|---|---|')
for k, (s, i, c, name) in enumerate(rows[:n], 1):
    print(f"| {k} | `\\{name}` | {c} | {s/1e6:.1f} | {100*s/tot:.1f} | {i/1e6:.1f} | {s/1e3/c:.2f} | {s/1e3/pages:.0f} |")
