#!/usr/bin/env python3
"""Flat \\include book for incr_bench --verify (it copies top-level files only):
main.tex + chNN.tex. The focused chapter's size is read twice with
\\pdffilesize: on the contents page (before the chapter) and inside it."""
import os
import sys

out, chapters, focus = sys.argv[1], int(sys.argv[2]), sys.argv[3]
before = len(sys.argv) < 5 or sys.argv[4] != "nobefore"
os.makedirs(out, exist_ok=True)
WORDS = "alpha beta gamma delta kernel glue penalty boxes rules marks inserts floats".split()
main = [r"\documentclass{report}", r"\begin{document}", r"\tableofcontents"]
if before:
    main.append(r"\noindent Size of %s before it: \pdffilesize{%s.tex}." % (focus, focus))
main += [r"\include{ch%02d}" % c for c in range(1, chapters + 1)] + [r"\end{document}"]
open(os.path.join(out, "main.tex"), "w").write("\n".join(main) + "\n")
for c in range(1, chapters + 1):
    body = [r"\chapter{Chapter %d}\label{ch:%d}" % (c, c),
            r"See Chapter~\ref{ch:%d} on page~\pageref{ch:%d}." % (c % chapters + 1, c % chapters + 1)]
    if "ch%02d" % c == focus:
        body.append(r"\edef\chsz{\pdffilesize{ch%02d.tex}}This chapter is \chsz\ bytes." % c)
    for p in range(30):
        body.append(" ".join(WORDS[(c * 7 + p * 3 + w) % len(WORDS)] for w in range(70)) + ".")
        body.append("")
        if p % 10 == 9:
            body.append(r"\section{Part %d}" % p)
    open(os.path.join(out, "ch%02d.tex" % c), "w").write("\n".join(body) + "\n")
