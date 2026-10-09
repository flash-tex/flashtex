#!/usr/bin/env python3
"""Synthetic \\include book: CHAPTERS chapters of about PAGES pages each.

Usage: gen_book.py OUT_DIR [CHAPTERS] [PAGES_PER_CHAPTER]
Deterministic text; cross-chapter \\ref and \\pageref so the focused run's
numbers come from the other chapters' .aux files.
"""
import os
import sys

out = sys.argv[1]
chapters = int(sys.argv[2]) if len(sys.argv) > 2 else 40
pages = int(sys.argv[3]) if len(sys.argv) > 3 else 25
os.makedirs(os.path.join(out, "chapters"), exist_ok=True)

WORDS = ("alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi "
         "omicron pi rho sigma tau upsilon phi chi psi omega typeset paragraph line "
         "break penalty glue kern box rule page float mark insert output routine").split()


def para(seed, n=95):
    w = []
    x = seed * 2654435761 % 4294967296
    for _ in range(n):
        x = (x * 1103515245 + 12345) % 2147483648
        w.append(WORDS[x % len(WORDS)])
    w[0] = w[0].capitalize()
    return " ".join(w) + "."


main = [r"\documentclass{book}", r"\usepackage{amsmath}", r"\begin{document}",
        r"\tableofcontents"]
for c in range(1, chapters + 1):
    main.append(r"\include{chapters/ch%02d}" % c)
main.append(r"\end{document}")
open(os.path.join(out, "main.tex"), "w").write("\n".join(main) + "\n")

for c in range(1, chapters + 1):
    body = [r"\chapter{Chapter %d}\label{ch:%d}" % (c, c)]
    sections = max(1, pages // 5)
    for s in range(1, sections + 1):
        body.append(r"\section{Section %d.%d}\label{sec:%d:%d}" % (c, s, c, s))
        other = (c % chapters) + 1
        body.append(r"See Chapter~\ref{ch:%d} on page~\pageref{ch:%d}." % (other, other))
        for p in range(14):
            body.append(para(c * 1000 + s * 50 + p))
            body.append("")
        body.append(r"\begin{equation}\label{eq:%d:%d} \int_0^{%d} x^{%d}\,dx = \frac{%d^{%d}}{%d}\end{equation}"
                    % (c, s, c, s, c, s + 1, s + 1))
    open(os.path.join(out, "chapters", "ch%02d.tex" % c), "w").write("\n".join(body) + "\n")
print(out)
