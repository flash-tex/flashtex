#!/usr/bin/env python3
"""callers.py SAMPLEFILE FUNC [DEPTH] -> distribution of caller chains of FUNC."""
import bisect, collections, os, re, sys

HERE = os.path.dirname(os.path.abspath(__file__))
starts = []
for line in open(os.path.join(HERE, "prof/fstarts.txt")):
    m = re.match(r"\s+0x([0-9A-F]+)\s+(\S+)", line)
    if m:
        starts.append((int(m.group(1), 16), m.group(2)))
starts.sort()
addrs = [s[0] for s in starts]
fn = lambda off: starts[bisect.bisect_right(addrs, off) - 1][1]
txt = open(sys.argv[1]).read().split("Call graph:")[1].split("Total number in stack")[0]
target, depth = sys.argv[2], int(sys.argv[3]) if len(sys.argv) > 3 else 4
stack, agg = [], collections.Counter()
for ln in txt.splitlines():
    m = re.match(r"^([\s+!:|]*)(\d+)\s+(.*)$", ln)
    if not m:
        continue
    d, c, rest = len(m.group(1)), int(m.group(2)), m.group(3)
    ma = re.search(r"\(in pdftex\).*\+ 0x([0-9a-f]+)", rest)
    f = fn(0x100000000 + int(ma.group(1), 16)) if ma else rest.split()[0]
    while stack and stack[-1][0] >= d:
        stack.pop()
    if f == target and target not in [s[1] for s in stack]:
        agg[" < ".join(s[1] for s in reversed(stack[-depth:]))] += c
    stack.append((d, f))
for k, v in agg.most_common(12):
    print(v, k)
