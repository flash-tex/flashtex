"""Summarise a typst-timing Chrome trace: inclusive time per scope name at
nesting depth <= 3 (main thread + workers), plus top-level wall time."""
import json, sys
from collections import defaultdict

ev = json.load(open(sys.argv[1]))
stacks = defaultdict(list)
incl = defaultdict(float)
count = defaultdict(int)
depth_names = defaultdict(set)
first = {}
wall = [None, None]
for e in ev:
    tid = e["tid"]
    st = stacks[tid]
    if e["ph"] == "B":
        st.append((e["name"], e["ts"]))
    else:
        name, ts = st.pop()
        d = len(st)
        dur = e["ts"] - ts
        # only count outermost occurrence of a name on this stack
        if all(n != name for n, _ in st):
            incl[(d, name)] += dur
            count[(d, name)] += 1
        if d == 0:
            wall[0] = ts if wall[0] is None else min(wall[0], ts)
            wall[1] = e["ts"] if wall[1] is None else max(wall[1], e["ts"])
print("threads:", len(stacks), "wall_ms: %.2f" % ((wall[1] - wall[0]) / 1000))
for (d, name), us in sorted(incl.items(), key=lambda kv: (kv[0][0], -kv[1])):
    if d <= int(sys.argv[2] if len(sys.argv) > 2 else 3) and us > 300:
        print("  " * d + "%-40s %8.2f ms  x%d" % (name, us / 1000, count[(d, name)]))
