#!/usr/bin/env python3
"""profpick.py PROFILE.tsv NAME...: calls, inclusive and self time (ms, and us per call) of
the named macros in a FLASHTEX_MACRO_PROFILE file."""
import sys

want = set(sys.argv[2:])
for l in open(sys.argv[1]):
    if l.startswith('#'):
        continue
    s, i, c, name = l.rstrip('\n').split('\t', 3)
    if name in want:
        s, i, c = float(s), float(i), int(c)
        print(f"{name}: calls {c}, inclusive {i/1e6:.1f} ms ({i/1e3/c:.0f} us/call), self {s/1e6:.1f} ms")
