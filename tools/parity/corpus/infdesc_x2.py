#!/usr/bin/env python3
"""Write infdesc-x2.tex: Infinite Descent with its whole book typeset twice.

Owner benchmark (2026-10-06): "make the 1000 page document infinite descent copied
once (so two of the full books in one doc). that's a truly heavy representative load".

Usage: infdesc_x2.py <unpacked infdesc dir>   (books.json entry infdesc-48825c5)

The preamble, front matter, table of contents and back matter (indexes, licence)
appear once; the main matter and the appendices appear twice, so the document is
about 1,140 pages with every diagram, theorem and exercise style twice. Labels are
defined twice, so LaTeX warns about multiply-defined labels exactly as pdflatex does;
that is part of the load, not an error. Nothing from the book is committed here.

The second copy inputs a copy of its own of every chapter file, x2/book/... (written
here, with their `\\input{book/...}` lines pointed at x2/book/), not the first copy's
files: a real 1,140-page document has each paragraph once, so an edit there changes
one place. (Inputting the same files twice made every keystroke change both copies,
pages ~286 and ~856: a benchmark artefact.) Images and listings stay shared.
"""
import pathlib
import re
import sys

INPUT = re.compile(r"\\input\{book/")


def second(text: str) -> str:
    """The second copy's text: its inputs are the x2/ copies."""
    return INPUT.sub(r"\\input{x2/book/", text)


def main() -> int:
    root = pathlib.Path(sys.argv[1])
    src = (root / "infdesc.tex").read_text(encoding="utf-8")
    start = src.index("\\mainmatter")
    end = src.index("\\backmatter")
    body = src[start:end]
    copy = re.sub(r"\\mainmatter", r"\\cleardoublepage", body, count=1)
    copy = copy.replace("\\appendix", "")  # \appendix once is enough for numbering
    for f in sorted((root / "book").rglob("*.tex")):
        out = root / "x2" / f.relative_to(root)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(second(f.read_text(encoding="utf-8")), encoding="utf-8")
    out = src[:end] + "\n% --- second copy of the book (infdesc_x2.py) ---\n" + second(copy) + src[end:]
    (root / "infdesc-x2.tex").write_text(out, encoding="utf-8")
    print(root / "infdesc-x2.tex")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
