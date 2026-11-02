#!/usr/bin/env python3
"""win_profile.py HOST_ERR PERF_SCRIPT: the flat profile of the samples inside the Unicode host's
`[win] begin`/`[win] end` windows (scratch: from a restart's resume to its first typeset page
sent; FLASHTEX_UNICODE_STAGES=1, `perf record --clockid monotonic`, `perf script -F time,sym --ns`).
"""
import collections
import sys

wins, b = [], None
for line in open(sys.argv[1], errors='replace'):
    if line.startswith('[win] begin '):
        b = int(line.split()[2])
    elif line.startswith('[win] end ') and b is not None:
        wins.append((b, int(line.split()[2])))
        b = None
wins.sort()
total_ms = sum(e - s for s, e in wins) / 1e6
counts, n, out = collections.Counter(), 0, 0
wi = 0
for line in open(sys.argv[2], errors='replace'):
    line = line.strip()
    if not line:
        continue
    # "<secs>.<ns>: <ip> <sym>" or "<secs>.<ns>: <sym>"
    head, _, rest = line.partition(':')
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
        sym = ' '.join(w[1:]) if len(w) > 1 and all(c in '0123456789abcdef' for c in w[0]) else ' '.join(w)
        counts[sym[:150]] += 1
        n += 1
    else:
        out += 1
print(f'{len(wins)} windows, {total_ms:.1f} ms in all ({total_ms / max(len(wins), 1):.2f} ms each); '
      f'{n} samples inside, {out} outside')
for sym, c in counts.most_common(60):
    print(f'  {100 * c / max(n, 1):6.2f}%  {sym}')
