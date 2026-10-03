#!/usr/bin/env python3
"""Byte comparison of the parity run's exported PDFs with latexmk's (oracle/ vs cand/)."""
import json, os, sys
W = sys.argv[1]
J = sys.argv[2]
same = diff = 0
diffs = []
for l in open(J):
    r = json.loads(l)
    if not r["result"].startswith("PASS"):
        continue
    d = os.path.join(W, "parity", r["doc"])
    stem = os.path.splitext(r["main"])[0]
    o = os.path.join(d, "oracle", os.path.dirname(r["main"]), stem + ".pdf")
    c = os.path.join(d, "cand", os.path.dirname(r["main"]), stem + ".pdf")
    if not (os.path.exists(o) and os.path.exists(c)):
        diffs.append((r["doc"], "missing"))
        diff += 1
        continue
    if open(o, "rb").read() == open(c, "rb").read():
        same += 1
    else:
        diff += 1
        diffs.append((r["doc"], os.path.getsize(o), os.path.getsize(c)))
print(f"byte-identical {same}, differ {diff}")
for x in diffs[:20]:
    print(" ", x)
