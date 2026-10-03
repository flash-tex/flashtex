#!/usr/bin/env python3
"""Open one document in a fresh host (external tools on) and keep what it
left: the DONEs, the TOOL messages and the output directory, for looking
into a failure of `xtools.py parity`.

    debug_open.py --host BIN --formats DIR --pool FILE --doc DIR:MAIN --out DIR [--separate-out]
"""
import argparse
import json
import os
import shutil
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import xtools as x  # noqa: E402

ap = argparse.ArgumentParser()
ap.add_argument("--host", required=True)
ap.add_argument("--formats", required=True)
ap.add_argument("--pool", required=True)
ap.add_argument("--doc", required=True)
ap.add_argument("--out", required=True)
ap.add_argument("--separate-out", action="store_true")
ap.add_argument("--env", action="append", default=[])
a = ap.parse_args()
a.env = dict(kv.split("=", 1) for kv in a.env)
src, main = a.doc.split(":")
shutil.rmtree(a.out, ignore_errors=True)
cd = os.path.join(a.out, "cand")
x.copy_sources(src, cd, main)
h = x.Host(a.host, a.formats, a.pool, a.out, env_extra=a.env)
o = x.out_of(a, cd)
ev = h.cycle({"id": 1, "root": cd, "main": main, "output_dir": o, "external_tools": "auto", "incremental": True})
for d in ev["dones"]:
    print(json.dumps({k: d.get(k) for k in ("status", "mode", "cause", "restart_page", "converged_at", "rerun_pages",
                                             "typeset_pages", "pages", "cold_reason")}))
for t in ev["tools"]:
    print(json.dumps(t))
h.close()
