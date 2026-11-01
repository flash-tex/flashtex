#!/usr/bin/env python3
"""multicol oracle: `multicols`/`multicols*` against pdflatex.

pdflatex is the ORACLE ONLY (never in the product path, never in cargo
tests). This script writes fixtures/multicol/<name>.tex, runs pdflatex on
each (uncompressed PDF streams, which changes nothing typeset) and records
every word's origin and baseline and every rule from the content streams in
fixtures/multicol/expected/<name>.txt (bp, top-left origin), with the same
PDF reader as tools/page-frame-oracle/generate.py. multicol's own warnings
from the log are recorded as `warning <text>` lines.

Usage: generate.py [--keep DIR] [NAME ...]
"""
import argparse
import importlib.util
import os
import re
import shutil
import subprocess
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.dirname(os.path.dirname(HERE))
FIXTURES = os.path.join(CRATE, "fixtures", "multicol")
EXPECTED = os.path.join(FIXTURES, "expected")

_spec = importlib.util.spec_from_file_location("page_frame_oracle", os.path.join(CRATE, "tools", "page-frame-oracle", "generate.py"))
pf = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(pf)

sentences = pf.sentences
paras = pf.paras


def doc(body, opts="", preamble=""):
    o = f"[{opts}]" if opts else ""
    return (
        f"\\documentclass{o}{{article}}\n\\usepackage[T1]{{fontenc}}\n\\usepackage{{lmodern}}\n\\usepackage{{multicol}}\n"
        f"{preamble}\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )


def mc(body, n=2, star=False, preface=None, pre=None):
    name = "multicols*" if star else "multicols"
    head = f"\\begin{{{name}}}{{{n}}}"
    if preface is not None:
        head += f"[{preface}]"
    if pre is not None:
        head += f"[{pre}]"
    return f"{head}\n{body}\n\\end{{{name}}}"


def fixtures():
    f = {}
    f["01-two-balanced"] = doc(paras(1, 1, 40) + "\n\n" + mc(paras(101, 3, 60)) + "\n\n" + paras(2, 1, 40))
    f["02-three-columns"] = doc(paras(3, 1, 40) + "\n\n" + mc(paras(102, 4, 70), n=3) + "\n\n" + paras(4, 1, 40))
    f["03-four-columns"] = doc(mc(paras(103, 4, 80), n=4))
    f["04-uneven-content"] = doc(paras(5, 1, 30) + "\n\n" + mc(paras(104, 1, 170) + "\n\n" + sentences(1041, 12)) + "\n\n" + paras(6, 1, 30))
    f["05-multicols-star"] = doc(paras(7, 1, 40) + "\n\n" + mc(paras(105, 2, 60), star=True))
    f["06-columnbreak"] = doc(
        paras(8, 1, 40) + "\n\n" + mc(paras(106, 1, 50) + "\n\n\\columnbreak\n\n" + paras(107, 2, 60)) + "\n\n" + paras(9, 1, 30)
    )
    f["07-preface"] = doc(mc(paras(108, 3, 70), preface="\\section{Columns With a Preface}"))
    f["08-multipage"] = doc(paras(10, 1, 60) + "\n\n" + mc(paras(109, 18, 90)) + "\n\n" + paras(11, 1, 60))
    f["09-columnseprule"] = doc(mc(paras(110, 4, 70)) + "\n\n" + paras(12, 1, 40), preamble="\\setlength{\\columnseprule}{0.4pt}\n")
    f["10-raggedcolumns"] = doc(paras(13, 1, 30) + "\n\n" + mc(paras(111, 5, 45)) + "\n\n" + paras(14, 1, 30), preamble="\\raggedcolumns\n")
    f["11-itemize-inside"] = doc(
        mc(paras(112, 1, 40) + "\n\\begin{itemize}\n" + "".join(f"\\item {sentences(1120 + i, 14)}\n" for i in range(5)) + "\\end{itemize}\n" + paras(113, 1, 40))
    )
    f["12-display-inside"] = doc(
        mc(paras(114, 1, 50) + "\n\\[ a^2 + b^2 = c^2 \\]\n" + paras(115, 2, 50))
    )
    f["13-heading-before"] = doc("\\section{Introduction}\n" + mc(paras(116, 3, 60)) + "\n\n\\section{After}\n" + paras(15, 1, 40))
    f["14-twocolumn-document"] = doc(paras(16, 1, 40) + "\n\n" + mc(paras(117, 2, 50)) + "\n\n" + paras(17, 2, 40), opts="twocolumn")
    f["15-two-environments"] = doc(
        paras(18, 1, 30) + "\n\n" + mc(paras(118, 2, 40)) + "\n\n" + paras(19, 1, 30) + "\n\n" + mc(paras(119, 3, 40), n=3) + "\n\n" + paras(20, 1, 30)
    )
    f["16-premulticols-newpage"] = doc(paras(21, 8, 95) + "\n\n" + mc(paras(120, 2, 60), pre="200pt"))
    f["17-columnsep"] = doc(mc(paras(121, 3, 60)), preamble="\\setlength{\\columnsep}{2em}\n")
    f["18-multicolsep"] = doc(paras(22, 1, 40) + "\n\n" + mc(paras(122, 2, 50)) + "\n\n" + paras(23, 1, 40), preamble="\\setlength{\\multicolsep}{24pt plus 2pt minus 2pt}\n")
    f["19-three-multipage"] = doc(mc(paras(123, 24, 90), n=3))
    f["20-section-inside"] = doc(mc("\\section{First}\n" + paras(124, 2, 50) + "\n\n\\section{Second}\n" + paras(125, 2, 50)))
    f["21-star-multipage"] = doc(mc(paras(126, 16, 90), star=True) + "\n\n" + paras(24, 1, 40))
    f["22-twelve-point"] = doc(paras(25, 1, 40) + "\n\n" + mc(paras(127, 3, 60)) + "\n\n" + paras(26, 1, 40), opts="12pt")
    f["23-short-content"] = doc(paras(27, 1, 40) + "\n\n" + mc(sentences(128, 20)) + "\n\n" + paras(28, 1, 40))
    f["24-columnbreak-inline"] = doc(mc(paras(129, 1, 60) + " \\columnbreak " + paras(130, 1, 60)))
    f["25-mid-page-after-text"] = doc(paras(29, 4, 90) + "\n\n" + mc(paras(131, 6, 80)) + "\n\n" + paras(30, 2, 60))
    return f


def warnings_of(log):
    text = re.sub(r"\n(\(multicol\)|\s{2,})", " ", log)
    out = []
    for m in re.finditer(r"Package multicol Warning: ([^\n]*)", text):
        out.append(re.sub(r"\s+", " ", m.group(1)).strip())
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--keep", help="copy the pdflatex PDFs here")
    ap.add_argument("names", nargs="*")
    args = ap.parse_args()
    os.makedirs(EXPECTED, exist_ok=True)
    version = subprocess.run(["pdflatex", "--version"], capture_output=True, text=True).stdout.splitlines()[0]
    mcver = open(subprocess.run(["kpsewhich", "multicol.sty"], capture_output=True, text=True).stdout.strip()).read()
    mcver = re.search(r"\[(\d{4}/\d\d/\d\d v[\w.]+)", mcver).group(1)
    for name, src in fixtures().items():
        if args.names and name not in args.names:
            continue
        tex = os.path.join(FIXTURES, name + ".tex")
        with open(tex, "w") as fh:
            fh.write(src)
        with tempfile.TemporaryDirectory() as tmp:
            pdf = pf.run_pdflatex(tex, tmp)
            log = open(os.path.join(tmp, name + ".log"), encoding="latin-1").read()
            if args.keep:
                os.makedirs(args.keep, exist_ok=True)
                shutil.copy(pdf, args.keep)
                shutil.copy(os.path.join(tmp, name + ".log"), args.keep)
            pages = pf.extract(pdf)
        lines = [f"# {version}; multicol {mcver}", f"# source fixtures/multicol/{name}.tex (tools/multicol-oracle/generate.py)"]
        for w in warnings_of(log):
            lines.append(f"warning {w}")
        for p in pages:
            lines.append(f"page {p['number']} {p['width']:.4f} {p['height']:.4f}")
            for w in p["words"]:
                lines.append(f"word {p['number']} {w['x']:.4f} {w['baseline']:.4f} {w['font']} {w['size']:.4f} {w['text']}")
            for r in p["rules"]:
                lines.append(f"rule {p['number']} {r['x']:.4f} {r['top']:.4f} {r['width']:.4f} {r['height']:.4f}")
        with open(os.path.join(EXPECTED, name + ".txt"), "w") as fh:
            fh.write("\n".join(lines) + "\n")
        print(name, len(pages), "pages", sum(len(p["words"]) for p in pages), "words")


if __name__ == "__main__":
    main()
