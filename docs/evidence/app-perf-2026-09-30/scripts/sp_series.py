#!/usr/bin/env python3
"""Per-occurrence durations of one signpost interval, in time order, in rows of 10.
usage: sp_series.py sp.xml <name>"""
import sys
import xml.etree.ElementTree as ET

ids = {}
rows = set()
for _, row in ET.iterparse(sys.argv[1], events=("end",)):
    if row.tag != "row":
        continue
    v = {}
    for c in row:
        if "ref" in c.attrib:
            v[c.tag] = ids.get(c.attrib["ref"])
        else:
            x = c.text if c.text is not None else c.attrib.get("fmt")
            if "id" in c.attrib:
                ids[c.attrib["id"]] = x
            v[c.tag] = x
    row.clear()
    if v.get("subsystem") == "tech.jay3332.flashtex.mac" and v.get("signpost-name") == sys.argv[2]:
        rows.add((int(v["event-time"]), v.get("event-type"), v.get("os-signpost-identifier")))
begins = {}
d = []
for t, k, s in sorted(rows):
    if k == "Begin":
        begins[s] = t
    elif k == "End" and s in begins:
        d.append((t - begins.pop(s)) / 1e6)
for i in range(0, len(d), 10):
    print(f"{i:4d}", " ".join(f"{x:6.2f}" for x in d[i:i + 10]))
