#!/usr/bin/env python3
"""Line-break delta that \\usepackage{microtype} causes in a real document.

Usage: python3 hw_delta.py path/to/HW1.tex [more.tex ...]

Builds each file twice in a temporary directory (as is, and with the
`\\usepackage{microtype}` line removed) with pdflatex (oracle only), extracts
the PDF text per line with Ghostscript `txtwrite`, removes whitespace (txtwrite
spacing is unreliable) and reports which lines differ. Requires pdflatex, gs.
"""

import difflib
import os
import re
import shutil
import subprocess
import sys
import tempfile

PDFLATEX = shutil.which("pdflatex") or "/Library/TeX/texbin/pdflatex"
GS = shutil.which("gs") or "/opt/homebrew/bin/gs"


def build(tmp, stem):
    subprocess.run([PDFLATEX, "-interaction=nonstopmode", stem + ".tex"], cwd=tmp,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    subprocess.run([GS, "-q", "-dNOPAUSE", "-dBATCH", "-sDEVICE=txtwrite", "-dTextFormat=3",
                    "-sOutputFile=" + stem + ".txt", stem + ".pdf"], cwd=tmp, check=True)
    with open(os.path.join(tmp, stem + ".txt"), encoding="utf-8", errors="replace") as fh:
        lines = [re.sub(r"\s+", "", l) for l in fh]
    with open(os.path.join(tmp, stem + ".log"), encoding="latin-1") as fh:
        log = fh.read().replace("\n", "")
    m = re.search(r"Output written on .*?\((\d+) page", log)
    return [l for l in lines if l], (m.group(1) if m else "?")


def main(paths):
    for path in paths:
        src = open(path).read()
        if "\\usepackage{microtype}\n" not in src:
            print(path, ": no \\usepackage{microtype} line")
            continue
        with tempfile.TemporaryDirectory() as tmp:
            open(os.path.join(tmp, "with.tex"), "w").write(src)
            open(os.path.join(tmp, "without.tex"), "w").write(src.replace("\\usepackage{microtype}\n", "\n"))
            a, pa = build(tmp, "with")
            b, pb = build(tmp, "without")
        sm = difflib.SequenceMatcher(a=b, b=a, autojunk=False)
        hunks = [op for op in sm.get_opcodes() if op[0] != "equal"]
        print(f"== {os.path.basename(path)}: lines with={len(a)} without={len(b)} pages {pa}/{pb} changed hunks={len(hunks)}")
        for _, i1, i2, j1, j2 in hunks:
            for l in b[i1:i2]:
                print("   -", l)
            for l in a[j1:j2]:
                print("   +", l)


if __name__ == "__main__":
    main(sys.argv[1:])
