#!/usr/bin/env python3
"""Main-thread cost per keystroke from a cell (tp.xml + sp.xml + bench.json).

usage: mainthread.py <celldir> [focus-substring ...]
Window = first .. last editorChange signpost (+0.3 s), i.e. the traced part of the
typing phase (xctrace attaches a few seconds after launch); per-keystroke
figures divide by the keystrokes inside that window.
Prints main-thread sampled ms in the window, per typed keystroke, and for each
focus substring the ms of main-thread stacks containing it.
"""
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))


def window(sp):
    import xml.etree.ElementTree as ET
    ids = {}
    ts = []
    begins = set()
    for _, row in ET.iterparse(sp, events=("end",)):
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
        if v.get("subsystem") == "tech.jay3332.flashtex.mac" and v.get("signpost-name") == "editorChange":
            ts.append(int(v["event-time"]))
            if v.get("event-type") == "Begin":
                begins.add(int(v["event-time"]))
    return min(ts) / 1e9, max(ts) / 1e9, len(begins)


def total(tp, a, b, focus=None):
    args = [sys.executable, os.path.join(HERE, "tp_summary.py"), tp, "--from", str(a), "--to", str(b), "--top", "1"]
    if focus:
        args += ["--focus", focus]
    out = subprocess.run(args, capture_output=True, text=True).stdout.splitlines()[0]
    return float(out.split("total ")[1].split(" ms")[0])


def main():
    c = sys.argv[1]
    a, b, typed = window(os.path.join(c, "sp.xml"))  # keystrokes seen by the trace
    b += 0.3  # the last keystroke's paint
    t = total(os.path.join(c, "tp.xml"), a, b)
    line = f"{os.path.basename(c)}: window {b - a:.2f} s, main {t:.0f} ms ({100 * t / ((b - a) * 1000):.0f}% busy), {t / max(typed, 1):.2f} ms/keystroke over {typed} traced keystrokes"
    for f in sys.argv[2:]:
        v = total(os.path.join(c, "tp.xml"), a, b, f)
        line += f" | {f} {v:.0f} ms ({v / max(typed, 1):.2f}/key)"
    print(line)


if __name__ == "__main__":
    main()
