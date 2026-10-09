#!/usr/bin/env python3
"""Focused run parity: pdflatex (MacTeX oracle) vs the FlashTeX engine on the
same command lines: the whole book to convergence (3 runs), then the
focused first line the host passes, `\\AtBeginDocument{\\includeonly{CH}}\\input
main.tex`, twice (FIRST=... gives another, e.g. the preamble form
`\\includeonly{%s}\\input main.tex`). Compares every .aux, the page count, and
P-T2-style per-page content streams (qpdf --qdf --normalize-content=y);
SAVE=FILE keeps the focused results to compare two FIRST forms.

Usage: oracle_focus.py CHAPTER [BOOKDIR]   (pdflatex on PATH is the oracle)"""
import base64
import glob
import hashlib
import json
import os
import shutil
import subprocess
import sys

S = os.environ.get("WORK", "/tmp/focus-bench")  # scratch: parity-* copies, tmp
IB = os.environ.get("INCR_BENCH_DIR", "/tmp/incr-bench")  # tools/incr-bench/mkeng.sh ENGINE
ENG = os.path.join(IB, os.environ.get("ENGINE", "focus"), "pdftex")
CH = sys.argv[1] if len(sys.argv) > 1 else "chapters/ch03"
SRC = sys.argv[2] if len(sys.argv) > 2 else os.path.join(S, "book-small")
os.makedirs(os.path.join(S, "tmp"), exist_ok=True)
ENV = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1", TMPDIR=os.path.join(S, "tmp"))


def run(engine, d, first, n):
    env = dict(ENV)
    argv = [engine, "-fmt=pdflatex"] if engine == ENG else ["pdflatex"]
    if engine == ENG:
        env["FLASHTEX_POOL"] = os.path.join(IB, os.environ.get("ENGINE", "focus"), "pdftex.pool")
        env["TEXFORMATS"] = os.path.join(IB, "fmt-" + os.environ.get("ENGINE", "focus")) + ":"
    for _ in range(n):
        subprocess.run(argv + ["-interaction=nonstopmode", "-jobname=main", first], cwd=d, env=env,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=600)


def pages(pdf):
    q = pdf + ".qdf"
    subprocess.run(["qpdf", "--qdf", "--normalize-content=y", "--object-streams=disable", pdf, q], check=False)
    j = json.loads(subprocess.run(["qpdf", "--json", "--json-stream-data=inline", "--decode-level=all", q],
                                  capture_output=True, text=True).stdout)
    objs = j["qpdf"][1]
    out = []
    for p in j["pages"]:
        cs = p["contents"]
        data = b""
        for ref in cs:
            o = objs["obj:" + ref]
            data += base64.b64decode(o["stream"]["data"])
        out.append(hashlib.sha256(data).hexdigest()[:16])
    return out


def auxes(d):
    r = {}
    for f in sorted(glob.glob(os.path.join(d, "**", "*.aux"), recursive=True)):
        r[os.path.relpath(f, d)] = hashlib.sha256(open(f, "rb").read()).hexdigest()[:16]
    return r


def written(d):
    for line in open(os.path.join(d, "main.log"), encoding="latin-1"):
        if line.startswith("Output written"):
            return line.split("(")[1].split(",")[0]
    return "?"


res = {}
for name, eng in (("pdflatex", "pdflatex"), ("flashtex", ENG)):
    d = os.path.join(S, "parity-" + name)
    shutil.rmtree(d, ignore_errors=True)
    shutil.copytree(SRC, d, ignore=shutil.ignore_patterns("*.aux", "*.log", "*.pdf", "*.toc"))
    run(eng, d, "main.tex", 3)
    full = (written(d), pages(os.path.join(d, "main.pdf")), auxes(d))
    run(eng, d, (os.environ.get("FIRST") or "\\AtBeginDocument{\\includeonly{%s}}\\input main.tex") % CH, 2)
    focus = (written(d), pages(os.path.join(d, "main.pdf")), auxes(d))
    res[name] = (full, focus)

for i, what in enumerate(("whole document", "focused " + CH)):
    a, b = res["pdflatex"][i], res["flashtex"][i]
    print(f"{what}: pages pdflatex {a[0]} flashtex {b[0]}; "
          f"content streams identical {a[1] == b[1]} ({sum(x == y for x, y in zip(a[1], b[1]))}/{len(a[1])}); "
          f"aux identical {a[2] == b[2]} ({len(a[2])} files)")
fp = res["pdflatex"][0][1]
cp = res["pdflatex"][1][1]
# the focused chapter's pages equal the whole document's own pages (oracle side)
same = [h for h in cp if h in fp]
print(f"oracle: {len(same)}/{len(cp)} focused pages are byte-identical content streams of whole-document pages")
if os.environ.get("SAVE"):
    json.dump({"pdflatex": res["pdflatex"][1], "flashtex": res["flashtex"][1]}, open(os.environ["SAVE"], "w"))
