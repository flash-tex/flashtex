#!/usr/bin/env python3
"""Print the first difference of fuzz.py findings. MIT.

    python3 tools/fuzz-unicode/show.py OUT/<class>/<seed> ...
"""
import json
import os
import sys


def show(d, ctx=4):
    with open(os.path.join(d, "result.json")) as fh:
        r = json.load(fh)
    print("===", d, r.get("cls"), r.get("rc"))
    det = r.get("detail")
    if isinstance(det, dict) and "line" not in det:
        for k, v in det.items():
            print("  %s: %s" % (k, v))
    elif isinstance(det, dict):
        print("line", det.get("line"))
        for c in det.get("context", [])[-ctx:]:
            print("   ", repr(c))
        print("  O", repr(det.get("oracle")))
        print("  C", repr(det.get("candidate")))
    else:
        print(" ", det)


if __name__ == "__main__":
    for a in sys.argv[1:]:
        show(a)
