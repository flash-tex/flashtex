#!/usr/bin/env python3
"""Summarise an `xctrace export` of a Time Profiler `time-profile` table.

usage: tp_summary.py tp.xml [--thread main|all] [--from S] [--to S] [--top N]
                            [--app FlashTeXMac] [--focus SUBSTR] [--children SUBSTR]

Prints, for the chosen thread(s) and time window:
  - total sampled ms (1 sample = its weight, normally 1 ms)
  - top "app frames": the innermost frame in the app binary on each stack
    (inclusive cost attributed to the app function that caused the work)
  - top self frames (leaf symbol, any binary)
  - top inclusive frames of any binary (each symbol counted once per stack)
  - with --children SUBSTR: the 3 frames called by the outermost frame matching SUBSTR
With --focus, only stacks containing a frame whose name contains SUBSTR.
The XML uses id/ref de-duplication; this resolves it.
"""
import sys
import xml.etree.ElementTree as ET
from collections import Counter


def main():
    a = sys.argv[1:]
    path = a[0]
    opt = {"--children": None, "--thread": "main", "--from": None, "--to": None, "--top": "25", "--app": "FlashTeXMac", "--focus": None}
    i = 1
    while i < len(a):
        opt[a[i]] = a[i + 1]
        i += 2
    top = int(opt["--top"])
    t_from = float(opt["--from"]) * 1e9 if opt["--from"] else None
    t_to = float(opt["--to"]) * 1e9 if opt["--to"] else None

    frames = {}   # id -> (name, binary)
    binaries = {}
    threads = {}  # id -> fmt
    weights = {}
    backtraces = {}  # id -> [frame ids]
    times = {}

    total = 0.0
    app_inner = Counter()
    self_c = Counter()
    incl = Counter()
    kids = Counter()
    n = 0
    for _, row in ET.iterparse(path, events=("end",)):
        if row.tag != "row":
            continue
        st = row.find("sample-time")
        if st is not None:
            if "ref" in st.attrib:
                t = times[st.attrib["ref"]]
            else:
                t = int(st.text); times[st.attrib["id"]] = t
        else:
            t = 0
        th = row.find("thread")
        if "ref" in th.attrib:
            tname = threads[th.attrib["ref"]]
        else:
            tname = th.attrib.get("fmt", ""); threads[th.attrib["id"]] = tname
        w = row.find("weight")
        if w is None:
            wv = 1.0
        elif "ref" in w.attrib:
            wv = weights[w.attrib["ref"]]
        else:
            wv = int(w.text) / 1e6; weights[w.attrib["id"]] = wv
        bt = row.find("tagged-backtrace") or row.find("backtrace")
        stack = []
        if bt is not None:
            if "ref" in bt.attrib:
                stack = backtraces.get(bt.attrib["ref"], [])
            else:
                for f in bt.iter("frame"):
                    if "ref" in f.attrib:
                        stack.append(f.attrib["ref"])
                    else:
                        b = f.find("binary")
                        if b is not None:
                            if "ref" in b.attrib:
                                bn = binaries.get(b.attrib["ref"], "?")
                            else:
                                bn = b.attrib.get("name", "?"); binaries[b.attrib["id"]] = bn
                        else:
                            bn = "?"
                        frames[f.attrib["id"]] = (f.attrib.get("name", f.attrib.get("addr", "?")), bn)
                        stack.append(f.attrib["id"])
                backtraces[bt.attrib["id"]] = stack
        row.clear()
        if opt["--thread"] == "main" and not tname.startswith("Main Thread"):
            continue
        if t_from is not None and t < t_from:
            continue
        if t_to is not None and t > t_to:
            continue
        names = [frames.get(fid, ("?", "?")) for fid in stack]
        if opt["--focus"] and not any(opt["--focus"] in nm for nm, _ in names):
            continue
        n += 1
        total += wv
        if names:
            self_c[names[0][0]] += wv
        for nm, bn in names:
            if bn == opt["--app"]:
                app_inner[nm] += wv
                break
        else:
            app_inner["<no app frame>"] += wv
        for nm in set(nm for nm, _ in names):
            incl[nm] += wv
        if opt["--children"]:
            # names are leaf-first: the callees of the outermost matching frame precede it
            idx = max((i for i, (nm, _) in enumerate(names) if opt["--children"] in nm), default=None)
            if idx is not None:
                chain = [nm for nm, _ in names[max(0, idx - 3):idx]][::-1]
                kids[" > ".join(chain) if chain else "<self>"] += wv
    print(f"samples {n}, total {total:.0f} ms (thread={opt['--thread']}, window {opt['--from']}..{opt['--to']} s, focus={opt['--focus']})")
    def show(title, c):
        print(f"\n## {title}")
        for k, v in c.most_common(top):
            print(f"{v:9.0f} ms {100 * v / max(total, 1):5.1f}%  {k[:170]}")
    if opt["--children"]:
        show(f"callees (3 deep) of the outermost '{opt['--children']}'", kids)
    show("innermost app frame (inclusive of what it called)", app_inner)
    show("self (leaf)", self_c)
    show("inclusive (any binary)", incl)


if __name__ == "__main__":
    main()
