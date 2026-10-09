#!/usr/bin/env python3
"""makeindex alone: TeX Live's binary against the port's command line
(`flashtex-makeindex`), each under `/usr/bin/time -l`, on the same .idx in
fresh directories, interleaved, REPS times. Also `/bin/sh -c "makeindex
'X.idx'"`, the process the engine starts for restricted \\write18.

    measure.py --port <binary> [--reps N] [--out results.jsonl] FILE.idx ...
"""

import argparse
import json
import os
import re
import shutil
import statistics
import subprocess
import tempfile
import time

ORACLE = "/Library/TeX/texbin/makeindex"


def timed(cmd, cwd):
    t0 = time.perf_counter()
    p = subprocess.run(["/usr/bin/time", "-l"] + cmd, cwd=cwd, capture_output=True)
    wall = time.perf_counter() - t0
    e = p.stderr.decode(errors="replace")

    def g(pat):
        m = re.findall(pat, e)
        return float(m[-1]) if m else None
    return dict(rc=p.returncode, wall=wall, instr=g(r"(\d+)\s+instructions retired"),
                cycles=g(r"(\d+)\s+cycles elapsed"), user=g(r"([\d.]+) user"),
                load=os.getloadavg()[0])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", required=True)
    ap.add_argument("--reps", type=int, default=5)
    ap.add_argument("--out")
    ap.add_argument("idx", nargs="+")
    a = ap.parse_args()
    out = open(a.out, "a") if a.out else None
    tools = {
        "texlive": lambda n: [ORACLE, n],
        "texlive-via-sh": lambda n: ["/bin/sh", "-c", "%s '%s'" % (ORACLE, n)],
        "port": lambda n: [os.path.abspath(a.port), n],
    }
    rows = {}
    for path in a.idx:
        name = os.path.basename(path)
        for rep in range(a.reps):
            for tool, mk in tools.items():
                d = tempfile.mkdtemp(prefix="mki-m-")
                shutil.copy(path, d)
                r = timed(mk(name), d)
                shutil.rmtree(d, ignore_errors=True)
                rec = {"idx": name, "tool": tool, "rep": rep, **r}
                rows.setdefault((name, tool), []).append(r)
                if out:
                    out.write(json.dumps(rec) + "\n")
    print("%-24s %-15s %14s %14s %10s" % ("idx", "tool", "instr (med)", "cycles (med)", "wall s"))
    for (name, tool), rs in sorted(rows.items()):
        med = lambda k: statistics.median(r[k] for r in rs if r[k] is not None)
        print("%-24s %-15s %14.0f %14.0f %10.3f" % (name, tool, med("instr"), med("cycles"), med("wall")))


if __name__ == "__main__":
    main()
