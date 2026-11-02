#!/usr/bin/env python3
"""abs_summary.py HOST_ERR: per compile, the median ms from the compile's arrival (job_t0) to
each `[abs]` event of the Unicode host (scratch; FLASHTEX_UNICODE_STAGES=1). Events of a
compile run from its job_t0 up to the next one; the first three compiles (the open) are left out.
"""
import statistics
import sys

jobs = []
for line in open(sys.argv[1], errors='replace'):
    if not line.startswith('[abs] '):
        continue
    _, name, ms = line.split()
    ms = float(ms)
    if name == 'job_t0':
        jobs.append({'t0': ms, 'ev': []})
    elif jobs:
        jobs[-1]['ev'].append((name, ms - jobs[-1]['t0']))
ks = jobs[3:]
print(f'{len(jobs)} compiles; {len(ks)} summarised')
order, by = [], {}
for j in ks:
    seen = {}
    for name, d in j['ev']:
        seen[name] = seen.get(name, 0) + 1
        key = name if seen[name] == 1 else f'{name}#{seen[name]}'
        if key not in by:
            by[key] = []
            order.append(key)
        by[key].append(d)
for k in sorted(order, key=lambda k: statistics.median(by[k])):
    xs = by[k]
    print(f'  {k:24s} {statistics.median(xs):9.3f} ms  (n {len(xs)}, min {min(xs):.3f}, max {max(xs):.3f})')
