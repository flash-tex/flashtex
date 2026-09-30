#!/usr/bin/env python3
"""Summarise the app's os_signpost intervals/events from an `xctrace export`
of the `os-signpost` PointsOfInterest table (subsystem tech.jay3332.flashtex.mac).

usage: sp_summary.py sp.xml
Prints per-name interval count, p50/p95/max/total ms and event counts, plus the
first/last signpost time (seconds into the trace) so a Time Profiler window
can be cut to the typing phase (tp_summary.py --from/--to).
"""
import sys
import xml.etree.ElementTree as ET
from collections import defaultdict


def pct(v, p):
    v = sorted(v)
    if not v:
        return float("nan")
    k = max(1, int(round(p / 100 * len(v) + 0.4999)))
    return v[min(k, len(v)) - 1]


def main():
    ids = {}
    begins = {}
    intervals = defaultdict(list)
    events = defaultdict(int)
    first = {}
    last = {}
    rows = set()
    for _, row in ET.iterparse(sys.argv[1], events=("end",)):
        if row.tag != "row":
            continue
        vals = {}
        for child in row:
            if "ref" in child.attrib:
                vals[child.tag] = ids.get(child.attrib["ref"])
            else:
                v = child.text if child.text is not None else child.attrib.get("fmt")
                if "id" in child.attrib:
                    ids[child.attrib["id"]] = v
                vals[child.tag] = v
        row.clear()
        name = vals.get("signpost-name")
        sub = vals.get("subsystem")
        if sub != "tech.jay3332.flashtex.mac" or not name:
            continue
        rows.add((int(vals["event-time"]), name, vals.get("event-type"), vals.get("os-signpost-identifier")))
    # The export repeats each signpost row; de-duplicate and pair in time order.
    for t, name, kind, sid in sorted(rows):
        first.setdefault(name, t)
        last[name] = t
        if kind == "Begin":
            begins[(name, sid)] = t
        elif kind == "End":
            b = begins.pop((name, sid), None)
            if b is not None:
                intervals[name].append((t - b) / 1e6)
        else:
            events[name] += 1
    for name in sorted(set(intervals) | set(events)):
        v = intervals.get(name, [])
        span = f"{first[name] / 1e9:.2f}..{last[name] / 1e9:.2f} s"
        if v:
            print(f"{name:14s} n {len(v):5d}  p50 {pct(v, 50):8.2f}  p95 {pct(v, 95):8.2f}  max {max(v):8.2f}  total {sum(v):9.1f} ms   {span}")
        else:
            print(f"{name:14s} events {events[name]:5d}   {span}")


if __name__ == "__main__":
    main()
