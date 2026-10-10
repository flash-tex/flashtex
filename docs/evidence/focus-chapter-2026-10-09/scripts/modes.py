#!/usr/bin/env python3
"""Per-compile host fields of dl3-keys output files."""
import json
import sys

KEYS = ["mode", "cold_reason", "passes", "pass_modes", "restart_page", "restart_preamble", "typeset_pages", "rerun_pages",
        "converged_at", "first_page_ms", "elapsed_ms"]
for f in sys.argv[1:]:
    print("==", f)
    for i, l in enumerate(open(f)):
        j = json.loads(l)
        h = j.get("host") or {}
        if "summary" in j:
            continue
        if i < int(__import__("os").environ.get("N", "8")) or "open" in j:
            print({k: h.get(k) for k in KEYS if k in h},
                  {k: j.get(k) for k in ("open", "edited_page_ms", "first_page_ms", "done_ms") if k in j})
