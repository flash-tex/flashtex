#!/usr/bin/env python3
"""proftop.py PROF [N] [--incl] [--grep RE]: a macro profile's header and its top N rows (self or inclusive)."""
import sys, re
args = [a for a in sys.argv[1:] if not a.startswith('--')]
f = args[0]
n = int(args[1]) if len(args) > 1 else 30
incl = '--incl' in sys.argv
pat = None
if '--grep' in sys.argv:
    pat = re.compile(sys.argv[sys.argv.index('--grep') + 1])
rows = []
head = ''
for l in open(f):
    if l.startswith('#'):
        if 'total_ns' in l:
            head = l.strip()
        continue
    p = l.rstrip('\n').split('\t')
    if len(p) < 4:
        continue
    rows.append((int(p[0]), int(p[1]), int(p[2]), p[3]))
tot = int(re.search(r'total_ns=(\d+)', head).group(1))
print(head)
rows.sort(key=lambda r: -(r[1] if incl else r[0]))
k = 0
for s, i, c, name in rows:
    if pat and not pat.search(name):
        continue
    print('%6.1f%% self %6.1f%% incl %8d calls  %s' % (100 * s / tot, 100 * i / tot, c, name))
    k += 1
    if k >= n:
        break
