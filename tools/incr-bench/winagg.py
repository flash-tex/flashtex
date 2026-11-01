#!/usr/bin/env python3
"""winagg.py SCRIPT MARKS OUT: the samples of `perf script -F comm,tid,time,sym` (SCRIPT) that fall in
the host's edit windows (MARKS: `b NS` .. `e NS`, CLOCK_MONOTONIC), the first window left out, on
the engine thread, by symbol (measurement only, lane BEAMER-FRAME)."""
import collections
import re
import sys

script, marks, out = sys.argv[1:4]
wins = []
b = None
for l in open(marks):
    k, ns = l.split()
    if k == 'b':
        b = int(ns) / 1e9
    elif k == 'e' and b is not None:
        wins.append((b, int(ns) / 1e9))
        b = None
wins = wins[1:]
c = collections.Counter()
n = 0
import bisect
starts = [w[0] for w in wins]
for l in open(script, errors='replace'):
    m = re.match(r'\s*(\S+)\s+(\d+)\s+([\d.]+):\s+(.*)$', l)
    if not m or m.group(1) != 'engine':
        continue
    t = float(m.group(3))
    i = bisect.bisect_right(starts, t) - 1
    if i < 0 or t > wins[i][1]:
        continue
    sym = re.sub(r'\+0x[0-9a-f]+$', '', m.group(4).strip())
    c[sym] += 1
    n += 1
with open(out, 'w') as f:
    f.write(f'# {len(wins)} windows, {n} engine-thread samples in them\n')
    for s, k in c.most_common(300):
        f.write(f'{100 * k / max(1, n):6.2f}% {k:8d} {s}\n')
