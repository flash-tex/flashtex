#!/usr/bin/env python3
"""Write infdesc-x2.tex: Infinite Descent with its whole book typeset twice.

Owner benchmark (2026-10-06): "make the 1000 page document infinite descent copied
once (so two of the full books in one doc). that's a truly heavy representative load".

Usage: infdesc_x2.py <unpacked infdesc dir>   (books.json entry infdesc-48825c5)

The preamble, front matter, table of contents and back matter (indexes, licence)
appear once; the main matter and the appendices appear twice, so the document is
about 1,180 pages with every diagram, theorem and exercise style twice. Labels are
defined twice, so LaTeX warns about multiply-defined labels exactly as pdflatex does;
that is part of the load, not an error. Nothing from the book is committed here.
"""
import pathlib
import re
import sys


def main() -> int:
    root = pathlib.Path(sys.argv[1])
    src = (root / "infdesc.tex").read_text(encoding="utf-8")
    start = src.index("\\mainmatter")
    end = src.index("\\backmatter")
    body = src[start:end]
    second = re.sub(r"\\mainmatter", r"\\cleardoublepage", body, count=1)
    second = second.replace("\\appendix", "")  # \appendix once is enough for numbering
    out = src[:end] + "\n% --- second copy of the book (infdesc_x2.py) ---\n" + second + src[end:]
    (root / "infdesc-x2.tex").write_text(out, encoding="utf-8")
    print(root / "infdesc-x2.tex")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
