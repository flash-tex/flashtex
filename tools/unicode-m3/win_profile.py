#!/usr/bin/env python3
"""win_profile.py HOST_ERR PERF_SCRIPT NAME [MAX_MS]: the flat profile of the engine thread's
samples inside the Unicode host's `[win] NAME-begin`/`NAME-end` windows no longer than MAX_MS
(default 60: the opens' are left out). Scratch: FLASHTEX_UNICODE_STAGES=1; `pre` is a run's start
to its resume's end, `page` from there to its first typeset page sent. The perf data is
`perf record --clockid monotonic`'s, printed by `perf script -F comm,time,ip,sym --ns`.
"""
import collections
import statistics
import sys

name = sys.argv[3]
cap = float(sys.argv[4]) if len(sys.argv) > 4 else 60.0
wins, b = [], None
for line in open(sys.argv[1], errors='replace'):
    if line.startswith(f'[win] {name}-begin '):
        b = int(line.split()[2])
    elif line.startswith(f'[win] {name}-end ') and b is not None:
        e = int(line.split()[2])
        if (e - b) / 1e6 <= cap:
            wins.append((b, e))
        b = None
wins.sort()
lens = [(e - s) / 1e6 for s, e in wins]
counts, n = collections.Counter(), 0
wi = 0
for line in open(sys.argv[2], errors='replace'):
    head, _, rest = line.strip().partition(':')
    hw = head.split()
    try:
        t = int(round(float(hw[-1]) * 1e9))
    except (ValueError, IndexError):
        continue
    if hw[0] != 'unicode-engine':
        continue
    while wi < len(wins) and wins[wi][1] < t:
        wi += 1
    if wi < len(wins) and wins[wi][0] <= t <= wins[wi][1]:
        w = rest.split()
        hexy = len(w) > 1 and all(c in '0123456789abcdef' for c in w[0])
        sym = ' '.join(w[1:] if hexy else w)
        counts[sym[:150]] += 1
        n += 1
print(f'window {name}: {len(wins)} (at most {cap:.0f} ms), median {statistics.median(lens) if lens else 0:.2f} ms; '
      f'{n} engine-thread samples inside ({n / max(len(wins), 1):.1f} a window)')
for sym, c in counts.most_common(45):
    print(f'  {100 * c / max(n, 1):6.2f}%  {sym}')
