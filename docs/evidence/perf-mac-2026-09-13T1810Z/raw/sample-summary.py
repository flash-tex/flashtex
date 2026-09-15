#!/usr/bin/env python3
"""Summarise a `sample` call graph: main-thread busy samples and where they go.
usage: sample-summary.py <sample.txt> [sample_ms=5000]"""
import re, sys

path = sys.argv[1]
total_ms = int(sys.argv[2]) if len(sys.argv) > 2 else 5000
lines = open(path, encoding="utf-8", errors="replace").read().split("\n")
start = next(i for i, l in enumerate(lines) if l.startswith("Call graph:"))
end = next((i for i, l in enumerate(lines) if l.startswith("Total number in stack")), len(lines))
graph = lines[start + 1:end]

def parse(line):
    m = re.match(r"^(\s*)([+!:| ]*)(\d+) (.*)$", line)
    if not m: return None
    depth = len(m.group(1)) + len(m.group(2))
    return depth, int(m.group(3)), m.group(4)

# main thread subtree
rows = [parse(l) for l in graph]
rows = [r for r in rows if r]
main_idx = next(i for i, r in enumerate(rows) if "Main Thread" in r[2])
main_depth, main_total, _ = rows[main_idx]
sub = []
for r in rows[main_idx + 1:]:
    if r[0] <= main_depth: break
    sub.append(r)

def sum_top(pred):
    """Sum counts of frames matching pred, counting only the outermost match on each path."""
    total = 0; skip_depth = None
    for depth, n, name in sub:
        if skip_depth is not None and depth > skip_depth: continue
        skip_depth = None
        if pred(name):
            total += n; skip_depth = depth
    return total

app_frames = sum_top(lambda s: "(in FlashTeXMac)" in s and "FlashTeXMac_main" not in s)
flush = sum_top(lambda s: "NSRunLoop.flushObservers" in s)
ca_commit = sum_top(lambda s: "CA::Transaction::commit" in s)
layout = sum_top(lambda s: "layoutIfNeeded" in s or "NSHostingView.layout" in s or "_layoutSubtreeIfNeeded" in s)
event = sum_top(lambda s: "sendEvent" in s)
idle = sum_top(lambda s: "mach_msg2_trap" in s or "__CFRunLoopServiceMachPort" in s)
busy = main_total - idle
print(f"main thread samples: {main_total} (of ~{total_ms}); idle (mach_msg) {idle}; busy {busy} = {100*busy/total_ms:.0f}% of wall")
for label, n in [("SwiftUI NSRunLoop.flushObservers (graph updates)", flush),
                 ("CA::Transaction::commit (layout + commit)", ca_commit),
                 ("  of which AppKit layoutIfNeeded / NSHostingView.layout", layout),
                 ("NSApplication.sendEvent (key events, text view)", event),
                 ("app code frames (in FlashTeXMac), outermost", app_frames)]:
    print(f"  {label}: {n} ({100*n/max(busy,1):.0f}% of busy)")
# top FlashTeXMac symbols by self+children (outermost only)
from collections import Counter
c = Counter()
skip_depth = None
for depth, n, name in sub:
    if skip_depth is not None and depth > skip_depth: continue
    skip_depth = None
    if "(in FlashTeXMac)" in name and "FlashTeXMac_main" not in name:
        c[name.split("  (in")[0][:90]] += n; skip_depth = depth
print("  top app frames:")
for name, n in c.most_common(12): print(f"    {n:5d}  {name}")
