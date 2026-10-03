#!/usr/bin/env python3
"""genrefs.py OUTDIR: the P4-L5 cross-reference documents refs-30 and refs-120
(about 30 and 120 pages): article, hyperref and cleveref, a table of contents,
sections and subsections with labels, backward and forward \\ref, \\pageref and
\\cref, footnotes, citations of a thebibliography, prose paragraphs. The edits
of the soundness driver's structural kinds (section, label, ref, cite,
footnote, unlabel, unsection) change their .aux and .toc in every way a pass
can see. Written to OUTDIR/src-refs-N/refs-N.tex."""
import os
import random
import sys

WORDS = ("lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor "
         "incididunt ut labore et dolore magna aliqua enim ad minim veniam quis nostrud "
         "exercitation ullamco laboris nisi aliquip ex ea commodo consequat").split()


def doc(pages):
    r = random.Random(4000 + pages)
    nsec = max(4, pages // 3)
    out = ["\\documentclass[11pt]{article}\n\\usepackage{hyperref}\n\\usepackage{cleveref}\n",
           "\\begin{document}\n\\tableofcontents\n\n"]
    for s in range(nsec):
        out.append(f"\\section{{Topic {s}}}\\label{{sec:{s}}}\n\n")
        for p in range(6):
            n = r.randint(70, 100)
            w = " ".join(r.choice(WORDS) for _ in range(n))
            w = w[0].upper() + w[1:]
            if p == 1:
                w += f" As \\cref{{sec:{(s + 2) % nsec}}} on page~\\pageref{{sec:{(s + 2) % nsec}}} shows"
            if p == 3:
                w += f"\\footnote{{A note on {r.choice(WORDS)}, see section~\\ref{{sec:{max(0, s - 1)}}}.}}"
            if p == 4:
                w += f" as in \\cite{{key{s % 7}}}"
            if p == 2 and s % 3 == 0:
                out.append(f"\\subsection{{Detail {s}}}\\label{{sub:{s}}}\n\n")
            out.append(w + ".\n\n")
    out.append("\\begin{thebibliography}{9}\n")
    for b in range(7):
        out.append(f"\\bibitem{{key{b}}} A. Author{b}. \\emph{{A title {b}}}. 20{10 + b}.\n")
    out.append("\\end{thebibliography}\n\\end{document}\n")
    return "".join(out)


def main():
    d = sys.argv[1]
    for pages in (30, 120):
        sd = os.path.join(d, f"src-refs-{pages}")
        os.makedirs(sd, exist_ok=True)
        with open(os.path.join(sd, f"refs-{pages}.tex"), "w") as f:
            f.write(doc(pages))


if __name__ == "__main__":
    main()
