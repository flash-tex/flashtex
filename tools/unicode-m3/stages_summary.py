#!/usr/bin/env python3
"""stages_summary.py HOST_ERR: the median of each stage of the Unicode host's runs (scratch).

Reads the `[stage]` lines FLASHTEX_UNICODE_STAGES=1 prints (out::stage): each run starts at
`begin`; a named stage's line has its ms since the run began and since the last stage; `page`
lines are pages sent (the resumed pages first, then those typeset after `pre_main`).
"""
import statistics
import sys

runs = []
cur = None
for line in open(sys.argv[1], errors='replace'):
    if not line.startswith('[stage] '):
        continue
    w = line.split()
    if w[1] == 'begin':
        cur = {'named': {}, 'pages_before_main': 0, 'first_typeset': None, 'last_resumed': None}
        runs.append(cur)
        continue
    if cur is None:
        continue
    name, at, d = w[1], float(w[2]), float(w[3].lstrip('+'))
    if name == 'page':
        if 'pre_main' in cur['named']:
            if cur['first_typeset'] is None:
                cur['first_typeset'] = (at, d)
        else:
            cur['pages_before_main'] += 1
            cur['last_resumed'] = at
    else:
        cur['named'][name] = cur['named'].get(name, 0) + d
        cur['named'][name + '@'] = at

# the keystrokes only: runs that resumed pages (the open, a run from S0, has none)
ks = [r for r in runs if r['pages_before_main'] > 0 and r['first_typeset']]
print(f'{len(runs)} runs, {len(ks)} restarted from a page')
if not ks:
    sys.exit(0)
names = []
for r in ks:
    for n in r['named']:
        if not n.endswith('@') and n not in names:
            names.append(n)
med = lambda xs: statistics.median(xs) if xs else float('nan')
for n in names:
    xs = [r['named'][n] for r in ks if n in r['named']]
    at = [r['named'][n + '@'] for r in ks if n in r['named']]
    print(f'  {n:16s} +{med(xs):8.3f} ms   (at {med(at):8.3f})')
print(f"  resumed pages      {med([r['pages_before_main'] for r in ks]):.0f}")
print(f"  first typeset page at {med([r['first_typeset'][0] for r in ks]):.3f} ms "
      f"(+{med([r['first_typeset'][1] for r in ks]):.3f} after the last stage)")
