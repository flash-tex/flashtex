#!/usr/bin/env python3
"""Deterministic benchmark documents for the T7 latency gate (DESIGN.md §1.2, §8).

usage: gen.py OUTDIR [--sizes 10,100,300,1000]

Writes, for every size N:

  plain-N/main.tex  article, geometry, amsmath: prose, inline and display math,
                    sections. Byte-identical to the P4-L2-L3 evidence's
                    plain-N.tex (docs/evidence/p4-l2-l3-2026-09-29/scripts/gen.py),
                    so the two sets of numbers are comparable.
  full-N/main.tex   the same prose plus hyperref, siunitx, cleveref, xcolor,
                    footnotes, labels and cross-references (as the evidence's
                    full-N.tex) and a small TikZ picture every 18 paragraphs.

Every paragraph is one source line, and every body paragraph starts with a
capital letter: latency-bench finds its edit sites that way. The random
stream is Python's `random.Random` with fixed seeds, so the output depends
only on this file.

MIT, like the rest of tools/latency-bench.
"""
import os
import random
import sys

WORDS = ("lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor "
         "incididunt ut labore et dolore magna aliqua enim ad minim veniam quis nostrud "
         "exercitation ullamco laboris nisi aliquip ex ea commodo consequat").split()

# Paragraphs per page, calibrated on the engine. plain and the evidence's full:
# about 4.6 paragraphs of ~95 words per page with one display per 6 paragraphs.
# full with TikZ: recalibrated on this generator (see README.md).
PER_PAGE = {"plain": 6.1, "full": 5.5}
TIKZ_EVERY = 18


def para(r, k, full):
    n = r.randint(80, 110)
    w = [r.choice(WORDS) for _ in range(n)]
    s = " ".join(w)
    s = s[0].upper() + s[1:]
    mid = len(s) // 2
    mid = s.index(" ", mid)
    s = s[:mid] + f" with $x_{{{k % 17}}}^2+\\frac{{a}}{{b}}=\\sum_{{i=1}}^n c_i$" + s[mid:]
    if full:
        if k % 5 == 2:
            s += f"\\footnote{{A note on {r.choice(WORDS)} {r.choice(WORDS)} number {k}, see \\cref{{eq:{max(0, k // 6 - 1)}}}.}}"
        if k % 7 == 3:
            s += f" The speed was \\SI{{{r.randint(1, 99)}.{r.randint(0, 9)}}}{{\\metre\\per\\second}} and \\num{{{r.randint(1000, 99999)}}} samples."
        if k % 11 == 5 and k > 12:
            s += f" As shown in \\cref{{sec:{max(1, k // 24)}}} and \\eqref{{eq:{max(0, k // 6 - 2)}}}."
    return s + ".\n"


def tikz(k):
    # Small, cheap, deterministic: a few paths, a circle, a node with math.
    a = 1 + (k // TIKZ_EVERY) % 5
    return ("\\begin{center}\\begin{tikzpicture}[scale=0.5]"
            f"\\draw[->] (0,0) -- ({a + 2},0); \\draw[->] (0,0) -- (0,{a});"
            f"\\draw[blue] (0,0) .. controls (1,{a}) and (2,0) .. ({a + 1},{a / 2:.1f});"
            f"\\draw (1,1) circle (0.4); \\node at ({a + 1},{a}) {{$t_{{{k}}}$}};"
            "\\end{tikzpicture}\\end{center}\n\n")


def doc(pages, kind):
    full = kind == "full"
    # Seeds as the evidence's gen.py: the prose is the same.
    r = random.Random(1000 + pages + (7 if full else 0))
    out = ["\\documentclass[11pt]{article}\n",
           "\\usepackage[margin=1in]{geometry}\n\\usepackage{amsmath,amssymb}\n"]
    if full:
        out.append("\\usepackage{xcolor}\n\\usepackage{siunitx}\n\\usepackage{tikz}\n"
                   "\\usepackage{hyperref}\n\\usepackage{cleveref}\n")
    out.append("\\begin{document}\n\n")
    n = int(pages * PER_PAGE[kind])
    sec = 0
    eq = 0
    for k in range(n):
        if k % 24 == 0:
            sec += 1
            lab = f"\\label{{sec:{sec}}}" if full else ""
            out.append(f"\\section{{Section {sec}}}{lab}\n\n")
        out.append(para(r, k, full))
        out.append("\n")
        if k % 6 == 5:
            lab = f"\\label{{eq:{eq}}}" if full else ""
            out.append(f"\\begin{{equation}}{lab}\\int_0^\\infty e^{{-x^2}}\\,dx=\\frac{{\\sqrt\\pi}}{{2}}+{eq}\\end{{equation}}\n\n")
            eq += 1
        if full and k % TIKZ_EVERY == TIKZ_EVERY - 7:
            out.append(tikz(k))
    out.append("\\end{document}\n")
    return "".join(out)


def main():
    args = sys.argv[1:]
    if not args or args[0] in ("-h", "--help"):
        print(__doc__)
        sys.exit(0 if args else 2)
    d = args[0]
    sizes = [10, 100, 300, 1000]
    if "--sizes" in args:
        sizes = [int(x) for x in args[args.index("--sizes") + 1].split(",")]
    for pages in sizes:
        for kind in ("plain", "full"):
            p = os.path.join(d, f"{kind}-{pages}")
            os.makedirs(p, exist_ok=True)
            with open(os.path.join(p, "main.tex"), "w") as f:
                f.write(doc(pages, kind))


if __name__ == "__main__":
    main()
