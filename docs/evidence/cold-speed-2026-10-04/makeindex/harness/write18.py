#!/usr/bin/env python3
"""The engine's restricted \\write18 path, in-process makeindex against the
child process (FLASHTEX_MAKEINDEX=external), and both against TeX Live's
pdflatex: one engine pass per mode on a fresh copy of each document, then
every file the run left (.ind, .ilg, .idx, .aux, .log, .pdf) compared
between the two engine modes byte for byte, and the .ind/.ilg files against
pdflatex's.

    write18.py --engine-dir DIR --fmt-dir DIR [--reps N] [--out results.jsonl] NAME=SRC_DIR:ENTRY ...

DIR holds `pdftex` (flashtex-initex) and `pdftex.pool`; the format is
DIR/pdflatex.fmt. Each pass runs under `/usr/bin/time -l` (instructions
retired and cycles of the engine process, wall and user+sys), with the same
environment as P6-HYPEROPT's cold.py. SRC_DIR is copied, never modified.
"""

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time

BASE = {k: v for k, v in os.environ.items() if not k.startswith("FLASHTEX_")}
BASE.update(SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1", max_print_line="10000",
            error_line="254", half_error_line="238")


def timed(cmd, cwd, env):
    t0 = time.perf_counter()
    p = subprocess.run(["/usr/bin/time", "-l"] + cmd, cwd=cwd, env=env,
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, stdin=subprocess.DEVNULL)
    wall = time.perf_counter() - t0
    e = p.stderr.decode(errors="replace")

    def g(pat):
        m = re.findall(pat, e)
        return float(m[-1]) if m else None
    return dict(rc=p.returncode, wall=wall, user=g(r"([\d.]+) user"), sys=g(r"([\d.]+) sys"),
                instr=g(r"(\d+)\s+instructions retired"), cycles=g(r"(\d+)\s+cycles elapsed"),
                load=os.getloadavg()[0]), p.stdout, p.stderr


def snapshot(d):
    out = {}
    for root, _, names in os.walk(d):
        for n in names:
            full = os.path.join(root, n)
            out[os.path.relpath(full, d)] = hashlib.sha256(open(full, "rb").read()).hexdigest()
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--engine-dir", required=True)
    ap.add_argument("--fmt-dir", required=True)
    ap.add_argument("--reps", type=int, default=1)
    ap.add_argument("--out")
    ap.add_argument("--keep", help="keep the work directories under this path")
    ap.add_argument("docs", nargs="+")
    a = ap.parse_args()
    out = open(a.out, "a") if a.out else None
    bad = 0
    for spec in a.docs:
        name, rest = spec.split("=", 1)
        src, entry = rest.rsplit(":", 1)
        stem = entry[:-4] if entry.endswith(".tex") else entry
        work = tempfile.mkdtemp(prefix="mki-w18-", dir=a.keep)
        # One TeX Live pass (the oracle), then the engine in each mode,
        # each in its own copy; the modes alternate over the repetitions.
        tl = os.path.join(work, "texlive")
        shutil.copytree(src, tl)
        r, so, se = timed(["/Library/TeX/texbin/pdflatex", "-interaction=batchmode", entry], tl, dict(BASE))
        rec = {"doc": name, "mode": "texlive-pdflatex", **r}
        print(json.dumps(rec), flush=True)
        if out:
            out.write(json.dumps(rec) + "\n")
        snaps = {}
        for rep in range(a.reps):
            for mode in ("external", "in-process"):
                d = os.path.join(work, "%s-%d" % (mode, rep))
                shutil.copytree(src, d)
                env = dict(BASE, FLASHTEX_POOL=os.path.join(a.engine_dir, "pdftex.pool"),
                           FLASHTEX_FORMATS=a.fmt_dir)
                if mode == "external":
                    env["FLASHTEX_MAKEINDEX"] = "external"
                cmd = [os.path.join(a.engine_dir, "pdftex"), "-fmt=pdflatex", "-interaction=batchmode",
                       "-jobname=" + stem, r"\pdfsetrandomseed 1\relax\input{" + entry + "}"]
                r, so, se = timed(cmd, d, env)
                log = open(os.path.join(d, stem + ".log"), "rb").read().decode("latin-1")
                runs = re.findall(r"runsystem\((makeindex[^)]*)\)\.\.\.([^\n]*)", log)
                rec = {"doc": name, "mode": mode, "rep": rep, "runsystem": runs, **r}
                print(json.dumps(rec), flush=True)
                if out:
                    out.write(json.dumps(rec) + "\n")
                # The terminal: standard output, and standard error up to
                # /usr/bin/time's report.
                term = so + re.split(rb"\n\s+[\d.]+ real ", b"\n" + se)[0]
                snaps.setdefault(mode, dict(snapshot(d), **{
                    "<terminal>": hashlib.sha256(term).hexdigest()}))
                if rep == 0:
                    shutil.copy(os.path.join(d, stem + ".log"), os.path.join(work, mode + ".log"))
        ext, inp = snaps["external"], snaps["in-process"]
        diff = sorted(k for k in set(ext) | set(inp) if ext.get(k) != inp.get(k))
        tls = snapshot(tl)
        ind_ilg = sorted(k for k in inp if k.endswith((".ind", ".ilg")))
        vs_tl = sorted(k for k in ind_ilg if tls.get(k) != inp.get(k))
        res = {"doc": name, "files": len(inp), "differ_external_vs_in_process": diff,
               "ind_ilg": ind_ilg, "ind_ilg_differ_from_texlive": vs_tl}
        print(json.dumps(res), flush=True)
        if out:
            out.write(json.dumps(res) + "\n")
        if diff or vs_tl:
            bad += 1
        if not a.keep:
            shutil.rmtree(work, ignore_errors=True)
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
