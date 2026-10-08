#!/usr/bin/env python3
"""roots.py OUTDIR: per window (prof.N): instructions, shipouts and the root breakdown in M (nonzero >= 1 M)."""
import glob, os, re, sys
d = sys.argv[1]
def key(f):
    return int(f.rsplit('.', 1)[1])
for f in sorted(glob.glob(os.path.join(d, 'prof.*')), key=key):
    head = open(f).readline()
    instr = int(re.search(r'instr=(\d+)', head).group(1))
    ship = re.search(r'shipouts=(\d+)', head).group(1)
    parts = []
    for l in open(f):
        m = re.match(r'# root (\S+) (\d+)', l)
        if m and int(m.group(2)) >= 1e6:
            parts.append((int(m.group(2)), m.group(1)))
    parts.sort(reverse=True)
    tot = sum(p[0] for p in parts) or 1
    print('%s instr=%dM ship=%s  %s' % (os.path.basename(f), instr // 10**6, ship,
          ' '.join('%s=%dM(%d%%)' % (n, v // 10**6, round(100 * v / tot)) for v, n in parts)))
