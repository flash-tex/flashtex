#!/usr/bin/env python3
"""Per-trial soundness results of xtools sound JSON-lines files."""
import json, sys
for f in sys.argv[1:]:
    print("==", f)
    for l in open(f):
        r = json.loads(l)
        if r["result"].startswith("n/a"):
            continue
        print(" ", r["doc"], r["kind"], r["trial"], r.get("edit"), r["result"][:40], r.get("mismatches"),
              str(r.get("page_detail"))[:200])
