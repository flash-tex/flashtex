#!/usr/bin/env python3
"""genincbook.py BASE: an `\\include` book for the soundness sweeps, src-book-inc/book-inc.tex and its
chapters ch01.tex .. ch06.tex (top-level files: incr_bench.py copies those only).

LaTeX's `\\include` takes each chapter's `\\pdffilesize` (expl3's `\\file_full_name:n`), a read of the
chapter (#1724); an edit deep in a chapter restarts at the chapter's start unless that read is
revalidated (READ-REVALIDATE, `crate::revalidate`). The sweeps edit `ch03.tex` (soundness.py's
`--extra DIR:DOC:ch03.tex`): letters, sentences, sections, labels and references, with references across
chapters so that the `.aux` passes see them. Deterministic."""
import os
import sys

WORDS = ("alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma "
         "tau upsilon phi chi psi omega kernel glue penalty boxes rules marks inserts floats").split()
CHAPTERS = 6
PARAS = 24


def para(c, p):
    w = [WORDS[(c * 7 + p * 3 + k * 5) % len(WORDS)] for k in range(80)]
    w[0] = w[0].capitalize()
    return " ".join(w) + "."


def files():
    main = ["\\documentclass{report}", "\\begin{document}", "\\tableofcontents"]
    main += ["\\include{ch%02d}" % c for c in range(1, CHAPTERS + 1)] + ["\\end{document}"]
    out = {"book-inc.tex": "\n".join(main) + "\n"}
    for c in range(1, CHAPTERS + 1):
        nxt = c % CHAPTERS + 1
        body = ["\\chapter{Chapter %d}\\label{ch:%d}" % (c, c),
                "See Chapter~\\ref{ch:%d} on page~\\pageref{ch:%d}." % (nxt, nxt), ""]
        for p in range(PARAS):
            body.append(para(c, p))
            body.append("")
            if p % 8 == 7:
                body.append("\\section{Part %d}\\label{sec:%d:%d}" % (p, c, p))
                body.append("As in Section~\\ref{sec:%d:%d}." % (nxt, p))
                body.append("")
        out["ch%02d.tex" % c] = "\n".join(body) + "\n"
    return out


def main():
    d = os.path.join(sys.argv[1], "src-book-inc")
    os.makedirs(d, exist_ok=True)
    for name, text in files().items():
        with open(os.path.join(d, name), "w") as f:
            f.write(text)


if __name__ == "__main__":
    main()
