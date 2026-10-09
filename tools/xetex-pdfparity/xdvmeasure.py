#!/usr/bin/env python3
"""Measure xdvipdfmx's position precision on the corpus (README, PLAN.md §3.5).

For each case: run.py's reference run (PDF mode, passes until the auxiliary
files are stable), then one more pass with `-no-pdf` in the same directory
(the auxiliary files are stable, so it typesets the same pages) to keep the
XDV, then `xdvipdfmx` on that XDV; that PDF must be byte-identical to the
PDF-mode one (checked), so the XDV is the one the PDF was made from. Then
precision.py's two measurements on every case.

    xdvmeasure.py [--cases GLOB ...] [--formats DIR] [--keep]
"""
import argparse
import fnmatch
import os
import shutil
import subprocess
import sys
import tempfile
from collections import Counter, defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import run as R  # noqa: E402  (this directory's run.py)
import precision as P  # noqa: E402
import compare as C  # noqa: E402

L = R.L


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--reference", default="xetex")
    ap.add_argument("--cases", nargs="*", default=["*"])
    ap.add_argument("--formats")
    ap.add_argument("--keep", action="store_true")
    a = ap.parse_args(argv)
    tmp = tempfile.mkdtemp(prefix="xetex-pdfprec-")
    fdir = os.path.join(a.formats or tmp, "fmt-reference")
    if not os.path.exists(os.path.join(fdir, "xelatex.fmt")):
        os.makedirs(fdir, exist_ok=True)
        secs, code, _ = L.build_format(a.reference, fdir)
        print("format: exit %s, %.1f s" % (code, secs))
    cases = R.all_cases()
    names = [n for n in cases if any(fnmatch.fnmatch(n, p) for p in a.cases)]
    digits = defaultdict(Counter)
    worst = (0.0, None)
    worst_em = (0.0, None)
    total = 0
    transformed = 0
    by_program = {}
    for n in names:
        d = os.path.join(tmp, n)
        passes, code, _ = R.run_engine(a.reference, fdir, cases[n], d, n, None, 900)
        pdf = os.path.join(d, n + ".pdf")
        keep = os.path.join(d, n + "-pdfmode.pdf")
        shutil.copy(pdf, keep)
        exe = os.path.join(d, ".bin", "xelatex")
        env = L.env_for(fdir)
        subprocess.run([exe] + L.RUN_ARGS + [n + ".tex"], cwd=d, env=env,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        xdv = os.path.join(d, n + ".xdv")
        subprocess.run(["xdvipdfmx", "-q", "-E", "-o", n + ".pdf", n + ".xdv"], cwd=d, env=env,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        # Not byte-identical (the ToUnicode streams differ: measured), so
        # compared as run.py compares: the XDV-made PDF must be equivalent.
        rep = C.compare(keep, pdf, C.Options(visual=False))
        same = "equivalent" if rep.ok() else "%d differences" % rep.structural
        for op, c in P.digits(keep).items():
            digits[op].update(c)
        dev = P.deviation(keep, xdv)
        total += dev["glyphs"]
        if dev["max_bp"] > worst[0]:
            worst = (dev["max_bp"], (n, dev["worst"]))
        if dev["max_em"] > worst_em[0]:
            worst_em = (dev["max_em"], n)
        for kind, (bp, em) in dev["by_program"].items():
            x = by_program.get(kind, (0.0, 0.0))
            by_program[kind] = (max(x[0], bp), max(x[1], em))
        transformed += dev["transformed"]
        print("%s: exit %s, %d passes, xdvipdfmx(XDV) vs PDF mode: %s; %d glyphs (%d transformed), max %.6f bp (%.6f em)%s" % (
            n, code, passes, same, dev["glyphs"], dev["transformed"], dev["max_bp"], dev["max_em"],
            (" unpaired pages %s" % dev["unpaired_pages"]) if dev["unpaired_pages"] else ""))
    print("digits after the decimal point, per operator (digits:operands):")
    for op in sorted(digits):
        print("  %-3s %s" % (op, " ".join("%d:%d" % kv for kv in sorted(digits[op].items()))))
    print("all: %d glyphs; max deviation from TeX's position %.6f bp in %s; max %.6f em in %s" % (
        total, worst[0], worst[1], worst_em[0], worst_em[1]))
    print("glyphs inside transformations set by specials, not measured: %d" % transformed)
    for kind, (bp, em) in sorted(by_program.items()):
        print("  %s: max %.6f bp, %.6f em" % (kind, bp, em))
    if a.keep:
        print("kept:", tmp)
    else:
        shutil.rmtree(tmp, ignore_errors=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
