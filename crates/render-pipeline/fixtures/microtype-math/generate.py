#!/usr/bin/env python3
"""Regenerate the pdflatex expectations (microtype + inline math) for tests/microtype_math.rs.

pdflatex (TeX Live 2026, pdfTeX 1.40.29) is an ORACLE only;
`cargo test` never runs TeX. For every `*.tex` next to this script it builds
the PDF in a scratch directory, reads each word's box with poppler's
`pdftotext -bbox` and writes `<name>.expected`:

    # pdflatex <version>
    line <page> <yMax bp>
    word <xMin bp> <text>
    ...

Words are grouped into lines by their `yMax` (within 3bp); `xMin` is the pen
position of the word's first glyph in PDF points, which with character
protrusion and font expansion is pdfTeX's `hlist_out` position.

usage: python3 generate.py [--pdflatex PATH] [--pdftotext PATH]
"""

import argparse
import glob
import html
import os
import re
import shutil
import subprocess
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
WORD = re.compile(r'<word xMin="([-\d.]+)" yMin="([-\d.]+)" xMax="([-\d.]+)" yMax="([-\d.]+)">(.*?)</word>')


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pdflatex", default=shutil.which("pdflatex") or "/Library/TeX/texbin/pdflatex")
    ap.add_argument("--pdftotext", default=shutil.which("pdftotext") or "pdftotext")
    args = ap.parse_args()
    version = subprocess.run([args.pdflatex, "--version"], capture_output=True, text=True).stdout.splitlines()[0]
    for tex in sorted(glob.glob(os.path.join(HERE, "*.tex"))):
        name = os.path.splitext(os.path.basename(tex))[0]
        with tempfile.TemporaryDirectory() as tmp:
            shutil.copy(tex, tmp)
            env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
            subprocess.run([args.pdflatex, "-interaction=nonstopmode", "-halt-on-error", name + ".tex"],
                           cwd=tmp, env=env, check=True, capture_output=True)
            xml = subprocess.run([args.pdftotext, "-bbox", os.path.join(tmp, name + ".pdf"), "-"],
                                 capture_output=True, text=True, check=True).stdout
        out = [f"# {version}"]
        for page_no, chunk in enumerate(xml.split("<page ")[1:], start=1):
            words = sorted((float(m.group(4)), float(m.group(1)), html.unescape(m.group(5))) for m in WORD.finditer(chunk))
            lines = []
            for y, x, t in words:
                if lines and abs(lines[-1][0] - y) <= 3.0:
                    lines[-1][1].append((x, t))
                else:
                    lines.append((y, [(x, t)]))
            for y, ws in lines:
                out.append(f"line {page_no} {y:.3f}")
                out.extend(f"word {x:.3f} {t}" for x, t in sorted(ws))
        with open(os.path.join(HERE, name + ".expected"), "w") as f:
            f.write("\n".join(out) + "\n")
        print(name, sum(1 for l in out if l.startswith("line")), "lines")


if __name__ == "__main__":
    main()
