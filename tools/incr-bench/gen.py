#!/usr/bin/env python3
"""Deterministic benchmark documents for P4-L2-L3 (DESIGN.md §1.2, T7).

usage: gen.py OUTDIR
Writes plain-{10,100,300,1000}.tex (article, amsmath, geometry: prose, inline and
display math, sections) and full-{10,100,300,1000}.tex (the same plus hyperref,
siunitx, cleveref, xcolor, footnotes, labels and cross-references)."""
import os
import random
import sys

WORDS = ("lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor "
         "incididunt ut labore et dolore magna aliqua enim ad minim veniam quis nostrud "
         "exercitation ullamco laboris nisi aliquip ex ea commodo consequat").split()

# paragraphs per page, calibrated on the engine (about 4.6 paragraphs of ~95 words
# per page with one display per 6 paragraphs)
PER_PAGE = {False: 6.1, True: 5.75}


def para(r, k, full):
    n = r.randint(80, 110)
    w = [r.choice(WORDS) for _ in range(n)]
    s = " ".join(w)
    s = s[0].upper() + s[1:]
    # inline math in the middle
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


def doc(pages, full):
    r = random.Random(1000 + pages + (7 if full else 0))
    out = ["\\documentclass[11pt]{article}\n",
           "\\usepackage[margin=1in]{geometry}\n\\usepackage{amsmath,amssymb}\n"]
    if full:
        out.append("\\usepackage{xcolor}\n\\usepackage{siunitx}\n\\usepackage{hyperref}\n\\usepackage{cleveref}\n")
    out.append("\\begin{document}\n\n")
    n = int(pages * PER_PAGE[full])
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
    out.append("\\end{document}\n")
    return "".join(out)


def main():
    d = sys.argv[1]
    os.makedirs(d, exist_ok=True)
    for pages in (10, 100, 300, 1000):
        for full in (False, True):
            name = f"{'full' if full else 'plain'}-{pages}.tex"
            with open(os.path.join(d, name), "w") as f:
                f.write(doc(pages, full))


if __name__ == "__main__":
    main()
